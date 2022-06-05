

#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};


use anyhow::{
    Result,
    Context,
};
use derivative::Derivative;
use std::{
    thread,
    sync::mpsc::{
        self,
        Receiver,
        Sender,
    }, path::PathBuf,
};


use crate::{
    content::{
        providers::ContentProvider,
        handler::ContentHandler,
        manager::{
            GlobalContent,
            ID,
            ContentProviderID,
        },
        song::{
            Song,
            SongArt,
            SongPath,
        },
    },
    app::action::AppAction,
    service::yt::YTAction,
    service::yt::YTManager,
    image::UnprocessedImage,
};

impl Into<ContentHandlerAction> for Vec<ContentHandlerAction> {
    fn into(self) -> ContentHandlerAction {
        ContentHandlerAction::Actions(self)
    }
}
impl Into<ContentHandlerAction> for Option<ContentHandlerAction> {
    fn into(self) -> ContentHandlerAction {
        match self {
            Self::Some(a) => {
                a
            }
            None => {
                ContentHandlerAction::None
            }
        }
    }
}
impl ContentHandlerAction {
    pub fn apply(self, ch: &mut ContentHandler) -> Result<()> {
        self.dbg_log();
        match self {
            Self::None => (),
            Self::TryLoadContentProvider {loader_id} => {
                let loaded = ch.get_provider_mut(loader_id);
                let action = loaded.load(loader_id);
                action.apply(ch)?;
            }
            Self::LoadContentProvider {songs, content_providers, loader_id} => {
                let songs = songs.into_iter().map(|s| ch.alloc_song(s)).collect::<Vec<_>>();
                let content_providers = content_providers.into_iter().map(|cp| ch.alloc_content_provider(cp)).collect::<Vec<_>>();
                let cp = ch.get_provider_mut(loader_id);
                songs.into_iter().for_each(|s| cp.add_song(s));
                content_providers.into_iter().for_each(|c| cp.add_provider(c));
                // let mut loader = ch.get_provider(loader_id).clone();
                // for s in songs {
                //     let ci = ch.alloc_song(s);
                //     loader.add_song(ci);
                // }
                // for cp in content_providers {
                //     let id = ch.alloc_content_provider(cp);
                //     loader.add_provider(id);
                // }
                // let old_loader = ch.get_provider_mut(loader_id);
                // *old_loader = loader;        
            }
            Self::ReplaceContentProvider {old_id, cp} => {
                let p = ch.get_provider_mut(old_id);
                *p = cp;
                Self::TryLoadContentProvider { loader_id: old_id }.apply(ch)?;
            }
            Self::AddCPToCP {id, cp} => {
                let mut loaded_id = ch.alloc_content_provider(cp);
                // loaded_id.set_content_type(new_cp_content_type);
                let loader = ch.get_provider_mut(id);
                loader.add_provider(loaded_id);

                Self::TryLoadContentProvider { loader_id: loaded_id }.apply(ch)?;
            }
            Self::AddCPToCPAndContentStack {id, cp} => {
                let mut loaded_id = ch.alloc_content_provider(cp);
                // loaded_id.set_content_type(new_cp_content_type);
                let loader = ch.get_provider_mut(id);
                loader.add_provider(loaded_id);

                ch.content_stack.push(loaded_id);
                ch.register(loaded_id.into());

                Self::TryLoadContentProvider { loader_id: loaded_id }.apply(ch)?;

                ch.app_action.queue(AppAction::UpdateDisplayContent { content: ch.get_content_names() });
            }
            Self::PushToContentStack { id } => {
                dbg!(id);
                ch.content_stack.push(id);
                ch.register(id.into());
                match id {
                    GlobalContent::ID(ID::ContentProvider(id)) => {
                        Self::TryLoadContentProvider { loader_id: id }.apply(ch)?;
                    }
                    _ => (),
                }
            }
            Self::EnableTyping { content } => {
                ch.app_action.queue(
                    AppAction::EnableTyping {content}
                );
            }
            Self::PopContentStack => {
                match ch.content_stack.pop() {
                    Some(id) => {
                        ch.unregister(id.into());
                        Self::RefreshDisplayContent.apply(ch)?;    
                    }
                    None => (),
                }
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
                ch.app_action.queue(AppAction::UpdateDisplayContent { content: ch.get_content_names() });
            }
            Self::UpdateImage {img} => {
                ch.image_handler.set_image(img);
            }
            Self::PlaySong {song, art} => {
                let art = if let SongArt::YTSong(p) = art {
                    Some(p)
                } else {
                    art.load().apply(ch)?;
                    None
                };
                match song {
                    SongPath::LocalPath(..) => {
                        ch.player.play(song.to_string())?;
                    }
                    SongPath::YTPath(p) => {
                        let action: Self = YTAction::GetSong {
                            url: p.to_string(),
                            callback: Box::new(move |uri: String, thumbnail: String| -> ContentHandlerAction {
                                vec![
                                    ContentHandlerAction::PlaySongURI {uri},
                                    if art.is_some() {
                                        RustParallelAction::ProcessAndUpdateImageFromUrl {url: thumbnail}.into()
                                    } else {
                                        None.into()
                                    },
                                ].into()
                            })
                        }.into();
                        action.apply(ch)?;
                    }
                }
            }
            Self::PlaySongURI {uri} => {
                ch.player.play(uri)?;
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
    receiver: Receiver<ContentHandlerAction>,
    sender: Sender<ContentHandlerAction>,
    yt_man: YTManager,
}
impl Default for ParallelHandle {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            yt_man: YTManager::new().unwrap(),
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

    pub fn poll(&mut self) -> ContentHandlerAction {
        match self.receiver.try_recv().ok() {
            Some(a) => {
                dbg!("action received");
                a
            },
            None => self.yt_man.poll(),
        }
    }
}

#[derive(Debug)]
pub enum RustParallelAction {
    ProcessAndUpdateImageFromUrl {
        url: String,
    },
    ProcessAndUpdateImageFromSongPath {
        path: PathBuf,
    }
}
impl RustParallelAction {
    fn run(self, send: Sender<ContentHandlerAction>) -> Result<()> {
        match self {
            Self::ProcessAndUpdateImageFromUrl {url}=> {
                let mut img = UnprocessedImage::Url(url);
                img.prepare_image()?;
                send.send(ContentHandlerAction::UpdateImage {
                    img,
                })?;
            }
            Self::ProcessAndUpdateImageFromSongPath {path} => {
                let tf = lofty::read_from_path(&path, true)?;
                let tags = tf.primary_tag().context("no primary tag on the image")?;
                let pics = tags.pictures();
                let img = if pics.len() >= 1 {
                    Ok(
                        image::io::Reader::new(
                            std::io::Cursor::new(
                                pics[0].data().to_owned()
                            )
                        )
                        .with_guessed_format()?
                        .decode()?
                    )
                } else {
                    Err(anyhow::anyhow!("no image"))
                };

                let mut img = UnprocessedImage::Image {img: img?};
                img.prepare_image()?;
                send.send(ContentHandlerAction::UpdateImage {
                    img,
                })?;
            }
        }
        Ok(())
    }
}
#[derive(Debug)]
pub enum ParallelAction {
    Rust(RustParallelAction),
    Python(YTAction),
}
impl Into<ParallelAction> for YTAction {
    fn into(self) -> ParallelAction {
        ParallelAction::Python(self)
    }
}
impl Into<ParallelAction> for RustParallelAction {
    fn into(self) -> ParallelAction {
        ParallelAction::Rust(self)
    }
}
impl<T: Into<ParallelAction>> From<T> for ContentHandlerAction {
    fn from(a: T) -> Self {
        Self::ParallelAction { action: a.into() }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum ContentHandlerAction {
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
        // new_cp_content_type: ContentProviderContentType,
    },
    AddCPToCPAndContentStack {
        id: ContentProviderID,
        cp: ContentProvider,
        // new_cp_content_type: ContentProviderContentType,
    },
    PushToContentStack {
        id: GlobalContent,
    },
    EnableTyping {
        content: String,
    },
    PopContentStack,
    Actions(Vec<Self>),
    ParallelAction {
        action: ParallelAction,
    },
    UpdateImage{
        img: UnprocessedImage,
    },
    RefreshDisplayContent,
    PlaySong {
        song: SongPath,
        art: SongArt,
    },
    PlaySongURI {
        uri: String,
    },
    None,
}

