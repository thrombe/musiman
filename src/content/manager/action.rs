

#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use anyhow::Result;
use derivative::Derivative;
use std::{
    thread,
    fmt::Debug,
    borrow::Cow,
};
use tokio::{
    sync::mpsc::{
        unbounded_channel,
        UnboundedReceiver,
        UnboundedSender,
    },
    select,
};


use crate::{
    content::{
        providers::{
            traits::YankContext,
            ContentProvider,
        },
        manager::{
            manager::ContentManager,
            callback::ContentManagerCallback,
        },
        register::{
            ContentProviderID,
            GlobalProvider,
            ID,
        },
        song::Song,
    },
    app::action::{
        AppAction,
        TypingCallback,
    },
    service::{
        python::{
            action::PyAction,
            manager::PyManager,
        },
        editors::{
            Yanker,
            Edit,
        }
    },
    image::UnprocessedImage,
};

impl Into<ContentManagerAction> for Vec<ContentManagerAction> {
    fn into(self) -> ContentManagerAction {
        ContentManagerAction::Actions(self)
    }
}
impl Into<ContentManagerAction> for Option<ContentManagerAction> {
    fn into(self) -> ContentManagerAction {
        match self {
            Self::Some(a) => {
                a
            }
            None => {
                ContentManagerAction::None
            }
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum ContentManagerAction {
    LoadContentProvider {
        #[derivative(Debug="ignore")]
        songs: Vec<Song>,
        #[derivative(Debug="ignore")]
        content_providers: Vec<ContentProvider>,
        loader_id: ContentProviderID,
    },
    TryLoadContentProvider {
        loader_id: ContentProviderID,
    },
    ReplaceContentProvider {
        old_id: ContentProviderID,
        cp: ContentProvider,
    },
    AddCPToCP {
        id: ContentProviderID,
        cp: ContentProvider,
    },
    AddCPToCPAndContentStack {
        id: ContentProviderID,
        cp: ContentProvider,
    },
    PushToContentStack {
        id: GlobalProvider,
    },
    MaybePushToContentStack {
        id: GlobalProvider,
    },
    EnableTyping {
        content: String,
        #[derivative(Debug="ignore")]
        callback: TypingCallback,
        loader: ID,
    },
    PopContentStack,
    Actions(Vec<Self>),
    ParallelAction {
        action: ParallelAction,
    },
    UpdateImage{
        img: UnprocessedImage,
    },
    ClearImage,
    RefreshDisplayContent,
    PlaySongURI {
        uri: String,
    },
    OpenEditForCurrent,
    OpenEditFor {id: ID},
    Callback {
        callback: ContentManagerCallback,
    },
    Unregister {
        ids: Vec<ID>,
    },
    Register {
        ids: Vec<ID>,
    },
    LogTimeSince {
        instant: std::time::Instant,
        message: Cow<'static, str>,
    },
    None,
}

impl ContentManagerAction {
    pub fn apply(self, ch: &mut ContentManager) -> Result<()> {
        self.dbg_log();
        match self {
            Self::None => (),
            Self::TryLoadContentProvider {loader_id} => {
                let cp = ch.get_provider_mut(loader_id).as_loadable();
                if let Some(cp) = cp {
                    let action = cp.maybe_load(loader_id)?;
                    action.apply(ch)?;
                }
            }
            Self::LoadContentProvider {songs, content_providers, loader_id} => {
                let songs = songs
                .into_iter()
                .map(|s| ch.alloc_song(s))
                .collect::<Vec<_>>();
                
                let content_providers = content_providers
                .into_iter()
                .map(|cp| ch.alloc_content_provider(cp))
                .collect::<Vec<_>>();
                
                let cp = ch.get_provider_mut(loader_id);
                songs
                .into_iter()
                .for_each(|s| cp.as_song_provider_mut().unwrap().add_song(s));
                content_providers
                .into_iter()
                .for_each(|c| cp.as_provider_mut().unwrap().add_provider(c));
            }
            Self::ReplaceContentProvider {old_id, cp} => {
                let p = ch.get_provider_mut(old_id);
                *p = cp;
                Self::TryLoadContentProvider { loader_id: old_id }.apply(ch)?;
            }
            Self::AddCPToCP {id, cp} => {
                let loaded_id = ch.alloc_content_provider(cp);
                let loader = ch.get_provider_mut(id);
                loader.as_provider_mut().unwrap().add_provider(loaded_id);
            }
            Self::AddCPToCPAndContentStack {id, cp} => {
                let loaded_id = ch.alloc_content_provider(cp);
                let loader = ch.get_provider_mut(id);
                loader.as_provider_mut().unwrap().add_provider(loaded_id);

                ContentManagerAction::PushToContentStack {id: loaded_id.into()}.apply(ch)?;
                ContentManagerAction::RefreshDisplayContent.apply(ch)?;
            }
            Self::PushToContentStack { id } => {
                dbg!(id);
                ch.content_stack.push(id);
                ch.register(id);
                match id {
                    GlobalProvider::ContentProvider(id) => {
                        Self::TryLoadContentProvider { loader_id: id }.apply(ch)?;
                    }
                    _ => (),
                }
            }
            Self::MaybePushToContentStack { id } => {
                if ch.content_stack.last() != id {
                    Self::PushToContentStack { id }.apply(ch)?;
                }
            }
            Self::EnableTyping { content, callback, loader } => {
                ch.app_action_sender.send(
                    AppAction::EnableTyping {content, callback, loader}
                )?;
            }
            Self::PopContentStack => {
                match ch.content_stack.pop() {
                    Some(id) => {
                        ch.unregister(id);
                    }
                    None => (),
                }
                Self::RefreshDisplayContent.apply(ch)?;    
            }
            Self::Actions(actions) => {
                for action in actions {
                    action.apply(ch)?;
                }
            }
            Self::ParallelAction { action } => {
                ch.parallel_handle.run(action);
            }
            Self::RefreshDisplayContent => {
                ch.app_action_sender.send(AppAction::UpdateDisplayContent)?;
            }
            Self::UpdateImage {img} => {
                ch.image_handler.set_image(img);
            }
            Self::ClearImage => {
                ch.image_handler.clear_image();
                ch.app_action_sender.send(AppAction::Redraw)?;
            }
            Self::PlaySongURI {uri} => {
                ch.player.play(uri)?;
            }
            Self::OpenEditForCurrent => {
                ch.open_edit_for_current()?;
            }
            Self::OpenEditFor { id } => {
                ch.open_edit_for(id)?;
            }
            Self::Callback {callback} => {
                callback.call(ch)?;
            }
            Self::Unregister {ids} => {
                ids.into_iter().for_each(|id| ch.unregister(id));
            }
            Self::Register { ids } => {
                ids.into_iter().for_each(|id| ch.register(id));
            }
            Self::LogTimeSince { message, instant } => {
                let duration = std::time::Instant::now().duration_since(instant).as_secs_f64();
                debug!("{message}: {duration}");
            }
        }
        Ok(())
    }


    fn dbg_log(&self) {
        if let Self::None = self {return;}
        dbg!(&self);
    }
}




pub struct ParallelHandle {
    handles: Vec<thread::JoinHandle<Result<()>>>,
    receiver: UnboundedReceiver<ContentManagerAction>,
    sender: UnboundedSender<ContentManagerAction>,
    yt_man: PyManager,
}
impl Default for ParallelHandle {
    fn default() -> Self {
        let (sender, receiver) = unbounded_channel();
        Self {
            yt_man: PyManager::new().unwrap(),
            handles: Default::default(),
            receiver,
            sender,
        }
    }
}

impl ParallelHandle {
    fn run(&mut self, action: ParallelAction) { // TODO: use threadpool crate instead of creating threads as they are required
        let action = match action {
            ParallelAction::Python(a) => {
                return self.yt_man.run(a).unwrap();
            }
            ParallelAction::Rust(a) => a,
        };
        let sender = self.sender.clone();
        match self.handles.iter_mut().filter(|h| h.is_finished()).next() {
            Some(h) => {
                let handle = thread::spawn(move || action.run(sender));
                match std::mem::replace(h, handle).join() {
                    Ok(Ok(())) => (),
                    Err(e) => log::error!("{:#?}", e),
                    Ok(Err(e)) => log::error!("{:#?}", e),
                }
            }
            None => {
                self.handles.push(thread::spawn(move || action.run(sender)));
            }
        }
    }

    pub async fn recv(&mut self) -> ContentManagerAction {
        let a1 = self.receiver.recv();
        let a2 = self.yt_man.recv();
        select! {
            a1 = a1 => a1.unwrap(),
            a2 = a2 => a2,
        }
    }
}

pub type RsCallback = Box<dyn FnOnce() -> Result<RustParallelAction> + Sync + Send>;

#[derive(Derivative)]
#[derivative(Debug)]
pub enum RustParallelAction {
    Callback {
        #[derivative(Debug="ignore")]
        callback: RsCallback,
    },
    ContentManagerAction {
        action: Box<ContentManagerAction>,
    },
}
impl From<ContentManagerAction> for RustParallelAction {
    fn from(a: ContentManagerAction) -> Self {
        Self::ContentManagerAction { action: Box::new(a) }
    }
}
impl From<Vec<ContentManagerAction>> for RustParallelAction {
    fn from(a: Vec<ContentManagerAction>) -> Self {
        let a: ContentManagerAction = a.into();
        a.into()
    }
}

impl RustParallelAction {
    fn run(self, send: UnboundedSender<ContentManagerAction>) -> Result<()> {
        match self {
            Self::Callback {callback} => {
                callback()?.run(send)?;
            }
            Self::ContentManagerAction {action} => {
                send.send(*action)?;
            }
        }
        Ok(())
    }
}
#[derive(Debug)]
pub enum ParallelAction {
    Rust(RustParallelAction),
    Python(PyAction),
}
impl Into<ParallelAction> for PyAction {
    fn into(self) -> ParallelAction {
        ParallelAction::Python(self)
    }
}
impl Into<ParallelAction> for RustParallelAction {
    fn into(self) -> ParallelAction {
        ParallelAction::Rust(self)
    }
}
impl<T: Into<ParallelAction>> From<T> for ContentManagerAction {
    fn from(a: T) -> Self {
        Self::ParallelAction { action: a.into() }
    }
}
