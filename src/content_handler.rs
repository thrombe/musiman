
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

use crate::{
    song::{
        Song,
        SongContentType,
        SongMenuOptions,
        SongPath,
        SongArt,
    },
    content_providers::{
        self,
        HumanReadable, FriendlyID,
        // ContentProvider,
        // ContentProviderContentType,
        // ContentProviderMenuOptions,
        // ContentProviderType,
        // ContentProviderEditables,
    },
    image::{
        ImageHandler,
        UnprocessedImage,
    },
    editors::{
        Yanker,
        EditManager,
    },
    db_handler::DBHandler,
    notifier::Notifier,
    content_manager::{
        SongID,
        ContentManager,
        ContentProviderID,
        GlobalContent,
        ID,
    },
    ui::{
        AppAction,
        SelectedIndex,
    },
    yt_manager::{
        YTManager,
        YTAction,
    },
};
use musiplayer::Player;
use anyhow::{
    Result,
    Context,
};
use derivative::Derivative;
use std::{
    thread,
    borrow::Cow,
    sync::mpsc::{
        self,
        Receiver,
        Sender,
    }, path::PathBuf,
};
#[derive(Debug, Clone)]
// pub struct ContentProvider(Box<dyn content_providers::ContentProvider<MenuOption = dyn HumanReadable>>);
pub struct ContentProvider(Box<dyn content_providers::ContentProvider>);
// pub type ContentProvider = Box<dyn content_providers::ContentProvider>;
impl std::ops::Deref for ContentProvider {
    type Target = Box<dyn content_providers::ContentProvider>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ContentProvider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<Box<dyn content_providers::ContentProvider>> for ContentProvider {
    fn from(o: Box<dyn content_providers::ContentProvider>) -> Self {
        Self(o)
    }
}
impl ContentProvider {
    pub fn new(t: Box<dyn content_providers::ContentProvider>) -> Self {
        Self(t)
    }
}

struct ParallelHandle {
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

#[derive(Clone, Debug)]
pub enum ContentState {
    Normal,
    Menu {
        ctx: StateContext,
        id: GlobalContent,
    },
    Edit {
       ctx: StateContext,
       id: GlobalContent,
    },
    GlobalMenu(SelectedIndex),
}
impl Default for ContentState {
    fn default() -> Self {
        Self::Normal
    }
}
#[derive(Clone, Debug)]
pub struct StateContext(Vec<SelectedIndex>);
impl Default for StateContext {
    fn default() -> Self {
        Self(vec![Default::default()])
    }
}
impl StateContext {
    pub fn pop(&mut self) -> Option<SelectedIndex> {
        self.0.pop()
    }
    pub fn push(&mut self, i: SelectedIndex) {
        self.0.push(i);
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn last_mut(&mut self) -> &mut SelectedIndex {
        self.0.last_mut().unwrap()
    }
    pub fn last(&self) -> &SelectedIndex {
        self.0.last().unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct ContentStack {
    state: ContentState,
    stack: Vec<GlobalContent>,
}
impl ContentStack {
    pub fn new<T>(main_provider: T) -> Self
        where T: Into<ID>
    {
        Self {
            state: Default::default(),
            stack: vec![main_provider.into().into()],
        }
    }

    pub fn get_state(&self) -> &ContentState {
        &self.state
    }

    pub fn get_state_mut(&mut self) -> &mut ContentState {
        &mut self.state
    }

    pub fn set_state(&mut self, state: ContentState) {
        self.state = state;
    }

    pub fn open_menu<T>(&mut self, id: T)
        where T: Into<GlobalContent>
    {
        self.state = ContentState::Menu {
            ctx: Default::default(),
            id: id.into(),
        }
    }

    pub fn open_edit<T>(&mut self, id: T)
        where T: Into<GlobalContent>
    {
        self.state = ContentState::Edit {
            ctx: Default::default(),
            id: id.into(),
        }
    }

    pub fn open_global_menu(&mut self) {
        self.state = ContentState::GlobalMenu(Default::default());
    }

    pub fn set_state_normal(&mut self) {
        self.state = ContentState::Normal;
    }
    
    pub fn main_provider(&self) -> ContentProviderID {
        if let GlobalContent::ID(id) = self.stack.first().unwrap() {
            match id {
                ID::ContentProvider(id) => return *id,
                _ => (),
            }
        }
        unreachable!()
    }
    
    pub fn push<T>(&mut self, id: T)
        where T: Into<GlobalContent>
    {
        self.stack.push(id.into());
    }
    
    pub fn pop(&mut self) -> Option<GlobalContent> {
        dbg!(&self);
        debug!("popping");
        match self.state {
            ContentState::Normal => {
                if self.stack.len() > 1 {
                    self.stack.pop()
                } else {
                    None
                }
            }
            _ => {
                self.state = ContentState::Normal;
                None
            }
        }
    }

    pub fn last(&self) -> GlobalContent {
        *self.stack.last().unwrap()
    }
}

pub struct ContentHandler {
    // TODO: maybe try having just one ContentManager of enum of Song, ContentProvider, etc
    songs: ContentManager<Song, SongID>,
    content_providers: ContentManager<ContentProvider, ContentProviderID>,
    db_handler: DBHandler,

    content_stack: ContentStack,
    yanker: Yanker,
    edit_manager: EditManager,
    pub image_handler: ImageHandler,
    pub player: Player,
    notifier: Notifier,
    
    active_queue: Option<ContentProviderID>, // can also be a bunch of queues? like -> play all artists
    active_song: Option<SongID>,

    parallel_handle: ParallelHandle,
    app_action: AppAction,
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

pub enum DisplayContent {
    Names(Vec<String>),
    IDs(Vec<ID>),
    FriendlyID(Vec<FriendlyID>)
}
impl<'a> From<Box<dyn Iterator<Item = FriendlyID> + 'a>> for DisplayContent {
    fn from(ids: Box<dyn Iterator<Item = FriendlyID> + 'a>) -> Self {
        Self::FriendlyID(ids.collect())
    }
}
impl DisplayContent {
    fn get(self, ch: &ContentHandler) -> Vec<String> {
        match self {
            Self::Names(names) => names,
            Self::IDs(ids) => {
                ids.iter().map(|&id| {
                    match id {
                        ID::Song(id) => {
                            ch.get_song(id).get_name()
                        }
                        ID::ContentProvider(id) => {
                            ch.get_provider(id).get_name()
                        }
                    }.to_owned()
                }).collect()
            }
            Self::FriendlyID(fids) => {
                fids
                .into_iter()
                .map(|fid| {
                    match fid {
                        FriendlyID::String(c) => c,
                        FriendlyID::ID(id) => {
                            match id {
                                ID::Song(id) => {
                                    ch.get_song(id).get_name()
                                }
                                ID::ContentProvider(id) => {
                                    &ch.get_provider(id).get_name()
                                }
                            }.to_owned()
                        }
                    }
                })
                .collect()
            }
        }
    }
}

pub enum MenuOptions {
    Song(SongMenuOptions),
    // ContentProvider(ContentProviderMenuOptions),
}

impl ContentHandler {
    pub fn alloc_song(&mut self, s: Song) -> SongID {
        self.songs.alloc(s)
    }

    pub fn alloc_content_provider(&mut self, cp: ContentProvider) -> ContentProviderID {
        self.content_providers.alloc(cp)
    }

    pub fn register(&mut self, id: GlobalContent) {
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        self.songs.register(id);
                    }
                    ID::ContentProvider(id) => {
                        self.content_providers.register(id);
                    }
                }
            }
            GlobalContent::Notifier => (),
        }
    }
    
    pub fn unregister(&mut self, id: GlobalContent) {
        dbg!("unregister id", id);
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        let _ = self.songs.unregister(id);
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.content_providers.unregister(id);
                        match cp {
                            Some(cp) => {
                                for &s_id in cp.songs() {
                                    let _ = self.unregister(s_id.into());
                                }
                                for &cp_id in cp.providers() {
                                    let _ = self.unregister(cp_id.into());
                                }
                            }
                            None => (),
                        }
                    }
                }
            }
            GlobalContent::Notifier => (),
        }
    }
}

impl ContentHandler {
    pub fn new() -> Result<Self> {
        let dbh = DBHandler::try_load();
        let mut cp = ContentManager::new();
        let main_id = cp.alloc(
            ContentProvider(
                Box::new(
                    content_providers::MainProvider::default()) as Box<dyn content_providers::ContentProvider>
                )
            );
        let ch = Self {
            songs: ContentManager::new(),
            content_providers: cp,
            db_handler: dbh,
            content_stack: ContentStack::new(main_id),
            yanker: Yanker::new(),
            edit_manager: EditManager::new(),
            image_handler: Default::default(),
            player: Player::new()?,
            notifier: Notifier::new(),
            active_queue: None,
            active_song: None,
            parallel_handle: Default::default(),
            app_action: Default::default(),
        };
        Ok(ch)
    }

    // TODO: temporary implimentation
    pub fn load() -> Result<Self> {
        Self::new()
    }

    pub fn debug_current(&mut self) {
        // dbg!(&self.content_providers);
        dbg!(&self.content_stack);
        dbg!(&self.player);
        dbg!(self.player.is_finished());
        dbg!(self.player.duration());
        dbg!(self.player.position());
        dbg!(self.player.progress());
        let id = self.content_stack.last();
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        let s = self.get_song(id);
                        dbg!(s);
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        dbg!(cp);
                    }
                }
            }
            _ => (),
        }
    }

    pub fn poll_action(&mut self) -> Result<()> {
        self.parallel_handle.poll().apply(self)
    }

    pub fn enter_selected(&mut self) -> Result<()> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                match id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::ContentProvider(id) => {
                                let cp = self.get_provider_mut(id);
                                let content_id = cp.get_selected();
                                match content_id {
                                    ID::Song(song_id) => {
                                        self.play_song(song_id)?;
                                        self.set_queue(id);
                                    }
                                    ID::ContentProvider(id) => {
                                        ContentHandlerAction::PushToContentStack { id: id.into() }.apply(self)?;
                                    }
                                }
                            }
                            ID::Song(id) => {
                                unreachable!()
                            }
                        }
                    }
                    GlobalContent::Notifier => {
                        // do nothing?
                    }
                }
            }
            ContentState::Menu {ctx, id} => {
                match *id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::ContentProvider(id) => {
                                let cp = self.content_providers.get_mut(id).unwrap();
                                let action = cp.apply_option(ctx, id);
                                action.apply(self)?;
                            }
                            ID::Song(id) => {
                                todo!()
                            }
                        }
                    }
                    GlobalContent::Notifier => {
                        todo!()
                    }
                }
            }
            ContentState::Edit {ctx, id} => {
                match id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::ContentProvider(id) => {
                                todo!()
                            }
                            ID::Song(id) => {
                                todo!()
                            }
                        }
                    }
                    GlobalContent::Notifier => {
                        todo!()
                    }
                }
            }
            ContentState::GlobalMenu(i) => {
                todo!()
            }
        }
        self.app_action.queue(AppAction::UpdateDisplayContent { content: self.get_content_names() });
        Ok(())
    }

    pub fn get_content_names(&self) -> Vec<String> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                match id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::ContentProvider(id) => {
                                let cp = self.get_provider(id);
                                let dc: DisplayContent = cp.get_friendly_ids().into();
                                dc.get(self)
                            }
                            ID::Song(id) => unreachable!(),
                        }
                    }
                    GlobalContent::Notifier => todo!(),
                }
            }
            ContentState::Menu {ctx, id} => {
                match id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::Song(id) => todo!(),
                            ID::ContentProvider(id) => {
                                let cp = self.get_provider(*id);
                                let dc: DisplayContent = cp.menu_options(ctx).into();
                                dc.get(self)
                            }
                        }
                    }
                    GlobalContent::Notifier => todo!(),
                }
            }
            ContentState::Edit {ctx, id} => {
                todo!()
            }
            ContentState::GlobalMenu(i) => {
                todo!()
            }
        }
    }

    pub fn get_selected_index(&mut self) -> &mut SelectedIndex {
        let id = self.content_stack.last();
        let state = self.content_stack.get_state_mut();
        match state {
            ContentState::Normal => {
                match id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::ContentProvider(id) => {
                                let cp = self.content_providers.get_mut(id).unwrap();
                                cp.get_selected_index_mut()
                            }
                            ID::Song(..) => {
                                unreachable!()
                            }
                        }
                    }
                    GlobalContent::Notifier => {
                        todo!()
                    }
                }
            }
            ContentState::Edit {ctx, ..} => {
                ctx.last_mut()
            }
            ContentState::Menu {ctx, ..} => {
                ctx.last_mut()
            }
            ContentState::GlobalMenu(i) => {
                i
            }
        }
    }

    pub fn open_menu_for_current(&mut self) -> Result<()> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                self.open_menu_for(self.content_stack.last())?;
            }
            ContentState::Edit { .. } | ContentState::Menu { .. } | ContentState::GlobalMenu(..) => {
                // do nothing
            }
        }
        Ok(())
    }
    
    pub fn open_menu_for_selected(&mut self) -> Result<()> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                match id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::Song(id) => {
                                unreachable!()
                            }
                            ID::ContentProvider(id) => {
                                let cp = self.get_provider(id);
                                let id = cp.get_selected();
                                self.open_menu_for(id.into())?;
                            }
                        }
                    }
                    GlobalContent::Notifier => {
                        // do nothing
                    }
                }
            },
            ContentState::Menu { .. } | ContentState::Edit { .. } | ContentState::GlobalMenu(..) => {
                // do nothing
            }
        }
        Ok(())
    }

    fn open_menu_for(&mut self, id: GlobalContent) -> Result<()> {
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        let s = self.get_song(id);
                        if s.has_menu() {
                            self.content_stack.open_menu(id);
                        }
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        if cp.has_menu() {
                            self.content_stack.open_menu(id);
                        }
                    }
                }
            }
            GlobalContent::Notifier => {
                todo!()
            }
        }
        self.app_action.queue(AppAction::UpdateDisplayContent { content: self.get_content_names() });
        Ok(())
    }    

    pub fn open_edit_for_current(&mut self) -> Result<()> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                self.open_edit_for(self.content_stack.last())?;
            }
            ContentState::Edit { .. } | ContentState::Menu { .. } | ContentState::GlobalMenu(..) => {
                // do nothing
            }
        }
        Ok(())
    }    

    pub fn open_edit_for_selected(&mut self) -> Result<()> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                match id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::Song(id) => {
                                unreachable!()
                            }
                            ID::ContentProvider(id) => {
                                let cp = self.get_provider(id);
                                let id = cp.get_selected();
                                self.open_edit_for(id.into())?;
                            }
                        }
                    }
                    GlobalContent::Notifier => {
                        // do nothing
                    }
                }
            },
            ContentState::Menu { .. } | ContentState::Edit { .. } | ContentState::GlobalMenu(..) => {
                // do nothing
            }
        }
        Ok(())
    }

    fn open_edit_for(&mut self, id: GlobalContent) -> Result<()> {
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        let s = self.get_song(id);
                        todo!()
                        // if s.has_editables() {
                        //     self.content_stack.open_edit(id);
                        // }
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        if cp.has_editables() {
                            self.content_stack.open_edit(id);
                        }
                    }
                }
            }
            GlobalContent::Notifier => {
                todo!()
            }
        }
        self.app_action.queue(AppAction::UpdateDisplayContent { content: self.get_content_names() });
        Ok(())
    }    

    fn get_provider(&self, id: ContentProviderID) -> &ContentProvider {
        self.content_providers.get(id).unwrap()
    }

    fn get_provider_mut(&mut self, id: ContentProviderID) -> &mut ContentProvider {
        self.content_providers.get_mut(id).unwrap()
    }

    fn get_song(&self, id: SongID) -> &Song {
        self.songs.get(id).unwrap()
    }
    fn get_song_mut(&mut self, content_identifier: SongID) -> &mut Song {
        self.songs.get_mut(content_identifier).unwrap()
    }

    pub fn set_queue(&mut self, id: ContentProviderID) {
        self.active_queue = Some(id);
        let mp_id = self.content_stack.main_provider();

        // TODO: bad code to find queue provider. think of a better soloution
        let mp = self.get_provider(mp_id);
        for &cp_id in mp.providers().collect::<Vec<_>>() {
            // let cp = self.get_provider_mut(cp_id);
            // if cp.cp_type == ContentProviderType::Queues {
            //     cp.add(id.into());
            // }
        }
    }
    pub fn play_song(&mut self, id: SongID) -> Result<()> {
        let song = self.get_song(id);
        let art = song.get_art();
        let path = song.path();
        debug!("playing song {song:#?}");
        self.player.stop().unwrap();
        self.active_song = Some(id);
        ContentHandlerAction::PlaySong {
            song: path,
            art,
        }.apply(self)?;
        Ok(())
    }
    pub fn toggle_song_pause(&mut self) {
        self.player.toggle_pause().unwrap();
    }
    pub fn next_song(&mut self) -> Result<()> { // FIX: browsing around changes the next song instead of choosing the song next to the current song
        let id = match self.active_queue {
            Some(id) => id,
            None => return Ok(()),
        };
        if !self.increment_selection_on(id.into()) {
            return Ok(())
        }        
        let q = self.get_provider_mut(id);
        let song_id = q.get_selected();
        if let ID::Song(id) = song_id {
            self.play_song(id)?;
        }
        Ok(())
    }
    pub fn prev_song(&mut self) -> Result<()> {
        let id = match self.active_queue {
            Some(id) => id,
            None => return Ok(()),
        };
        if !self.increment_selection_on(id.into()) {
            return Ok(())
        }        
        let q = self.get_provider_mut(id);
        let song_id = q.get_selected();
        if let ID::Song(id) = song_id {
            self.play_song(id)?;
        }
        Ok(())
    }
    pub fn seek_song(&mut self, t: f64) -> Result<()> {
        self.player.seek(t)
    }

    pub fn get_app_action(&mut self) -> AppAction {
        std::mem::replace(&mut self.app_action, Default::default())
    }

    pub fn increment_selection(&mut self) {
        let state = self.content_stack.get_state_mut();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                self.increment_selection_on(id);
            }
            ContentState::Edit { ctx, id } => {
                let i = ctx.last_mut();
                let num_items = match *id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::Song(id) => {
                                todo!()
                            }
                            ID::ContentProvider(id) => {
                                let cp = self.content_providers.get(id).unwrap();
                                cp.num_editables()
                            }
                        }
                    }
                    GlobalContent::Notifier => todo!(),
                };
                if i.selected_index()+1 < num_items {
                    let index = i.selected_index();
                    i.select(index + 1);
                }
            }
            ContentState::Menu { ctx, id } => {
                let num_items = match *id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::Song(id) => {
                                todo!()
                            }
                            ID::ContentProvider(id) => {
                                let cp = self.content_providers.get(id).unwrap();
                                cp.menu_options(ctx).size_hint().0
                            }
                        }
                    }
                    GlobalContent::Notifier => todo!(),
                };
                let i = ctx.last_mut();
                if i.selected_index()+1 < num_items {
                    let index = i.selected_index();
                    i.select(index + 1);
                }
            }
            ContentState::GlobalMenu(i) => {
                todo!()
            }
        }
    }
    pub fn decrement_selection(&mut self) {
        let state = self.content_stack.get_state_mut();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                self.decrement_selection_on(id);
            }
            ContentState::Edit { ctx, id } => {
                let i = ctx.last_mut();
                if i.selected_index() > 0 {
                    let index = i.selected_index();
                    i.select(index - 1);
                }
            }
            ContentState::Menu { ctx, id } => {
                let i = ctx.last_mut();
                if i.selected_index() > 0 {
                    let index = i.selected_index();
                    i.select(index - 1);
                }
            }
            ContentState::GlobalMenu(i) => {
                todo!()
            }
        }
    }

    fn increment_selection_on(&mut self, id: GlobalContent) -> bool {
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider_mut(id);
                        cp.selection_increment()
                    }
                    ID::Song(id) => {
                        unreachable!()
                    }
                }
            }
            GlobalContent::Notifier => todo!(),
        }
    }
    fn decrement_selection_on(&mut self, id: GlobalContent) -> bool {
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider_mut(id);
                        cp.selection_decrement()
                    }
                    ID::Song(id) => {
                        unreachable!()
                    }
                }
            }
            GlobalContent::Notifier => todo!(),
        }
    }

    pub fn apply_typed(&mut self, content: String) -> Result<()> {
        todo!()
    }
}

