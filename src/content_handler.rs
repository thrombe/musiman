
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
    },
    content_providers::{
        ContentProvider,
        ContentProviderContentType,
        ContentProviderMenuOptions,
        ContentProviderType,
        ContentProviderEditables,
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
use anyhow::Result;

use std::{
    thread,
    sync::mpsc::{
        self,
        Receiver,
        Sender,
    }, path::PathBuf,
};

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
                img.prepare_image().unwrap();
                send.send(ContentHandlerAction::UpdateImage {
                    img,
                })?;
            }
            Self::ProcessAndUpdateImageFromSongPath {path} => {
                let tf = lofty::read_from_path(&path, true)?;
                let tags = tf.primary_tag().unwrap(); // its a tagged image tho it may still crash if user does something fishy
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

                let mut img = UnprocessedImage::Image(img?);
                img.prepare_image().unwrap();
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

pub struct ContentHandler {
    // TODO: maybe try having just one ContentManager of enum of Song, ContentProvider, etc
    songs: ContentManager<Song, SongID>,
    content_providers: ContentManager<ContentProvider, ContentProviderID>,
    db_handler: DBHandler,

    content_stack: Vec<GlobalContent>,
    yanker: Yanker,
    edit_manager: EditManager,
    pub image_handler: ImageHandler,
    pub player: Player,
    notifier: Notifier,
    logger: Logger,
    
    active_queue: Option<ContentProviderID>, // can also be a bunch of queues? like -> play all artists
    active_song: Option<SongID>,

    parallel_handle: ParallelHandle,
    app_action: AppAction,
}

pub struct Logger {
    entries: Vec<String>,
}


impl Logger {
    fn new() -> Self {
        Self { entries: vec![]}
    }
    pub fn log(&mut self, s: &str) {
        self.entries.push(s.into())
    }
}

#[derive(Debug)]
pub enum ContentHandlerAction {
    LoadContentProvider {
        songs: Vec<Song>,
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
        new_cp_content_type: ContentProviderContentType,
    },
    AddCPToCPAndContentStack {
        id: ContentProviderID,
        cp: ContentProvider,
        new_cp_content_type: ContentProviderContentType,
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
    pub fn apply(self, ch: &mut ContentHandler) {
        self.dbg_log();
        match self {
            Self::None => (),
            Self::TryLoadContentProvider {loader_id} => {
                let loaded = ch.get_provider_mut(loader_id);
                let action = loaded.load(loader_id);
                action.apply(ch);
            }
            Self::LoadContentProvider {songs, content_providers, loader_id} => {
                let mut loader = ch.get_provider(loader_id).clone();
                for s in songs {
                    let ci = ch.alloc_song(s);
                    loader.songs.push(ci);
                }
                for cp in content_providers {
                    let id = ch.alloc_content_provider(cp);
                    loader.providers.push(id);
                }
                let old_loader = ch.get_provider_mut(loader_id);
                *old_loader = loader;        
            }
            Self::ReplaceContentProvider {old_id, cp} => {
                let p = ch.get_provider_mut(old_id);
                *p = cp;
                Self::TryLoadContentProvider { loader_id: old_id }.apply(ch);
            }
            Self::AddCPToCP {id, cp, new_cp_content_type} => {
                let mut loaded_id = ch.alloc_content_provider(cp);
                loaded_id.set_content_type(new_cp_content_type);
                let loader = ch.get_provider_mut(id);
                loader.add(loaded_id.into());

                Self::TryLoadContentProvider { loader_id: loaded_id }.apply(ch);
            }
            Self::AddCPToCPAndContentStack {id, cp, new_cp_content_type} => {
                let mut loaded_id = ch.alloc_content_provider(cp);
                loaded_id.set_content_type(new_cp_content_type);
                let loader = ch.get_provider_mut(id);
                loader.add(loaded_id.into());

                ch.content_stack.push(loaded_id.into());
                ch.register(loaded_id.into());

                Self::TryLoadContentProvider { loader_id: loaded_id }.apply(ch);

                ch.app_action.queue(AppAction::UpdateDisplayContent { content: ch.get_content_names() })
            }
            Self::PushToContentStack { id } => {
                dbg!(id);
                ch.content_stack.push(id.into());
                ch.register(id.into());
                match id {
                    GlobalContent::ID(ID::ContentProvider(id)) => {
                        Self::TryLoadContentProvider { loader_id: id }.apply(ch);
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
                if ch.content_stack.len() > 1 {
                    let id = ch.content_stack.pop().unwrap();
                    ch.unregister(id.into());
                    Self::RefreshDisplayContent.apply(ch);
                }
            }
            Self::Actions(actions) => {
                for action in actions {
                    action.apply(ch);
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
        }
    }


    fn dbg_log(&self) {
        if let Self::None = self {return;}
        dbg!(&self);
    }
}

pub enum DisplayContent {
    Names(Vec<String>),
    IDS(Vec<ID>),
}
impl DisplayContent {
    fn get(self, ch: &ContentHandler) -> Vec<String> {
        match self {
            Self::Names(names) => names,
            Self::IDS(ids) => {
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
        }
    }
}

pub enum MenuOptions {
    Song(SongMenuOptions),
    ContentProvider(ContentProviderMenuOptions),
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
            GlobalContent::Log | GlobalContent::Notifier => (),
        }
    }
    
    pub fn unregister(&mut self, id: GlobalContent) {
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
                                for s_id in cp.songs {
                                    let _ = self.unregister(s_id.into());
                                }
                                for cp_id in cp.providers {
                                    let _ = self.unregister(cp_id.into());
                                }
                            }
                            None => (),
                        }
                    }
                }
            }
            GlobalContent::Log | GlobalContent::Notifier => (),
        }
    }
}

impl ContentHandler {
    pub fn new() -> Self {
        let dbh = DBHandler::try_load();
        let mut cp = ContentManager::new();
        let main_ci = cp.alloc(ContentProvider::new_main_provider());
        Self {
            songs: ContentManager::new(),
            content_providers: cp,
            db_handler: dbh,
            content_stack: vec![main_ci.into()],
            yanker: Yanker::new(),
            edit_manager: EditManager::new(),
            image_handler: Default::default(),
            player: Player::new(),
            notifier: Notifier::new(),
            logger: Logger::new(),
            active_queue: None,
            active_song: None,
            parallel_handle: Default::default(),
            app_action: Default::default(),
        }
    }

    // TODO: temporary implimentation
    pub fn load() -> Self {
        Self::new()
    }

    pub fn debug_current(&self) {
        dbg!(&self.content_providers);
        dbg!(&self.content_stack);
        let &id = self.content_stack.last().unwrap();
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

    pub fn poll_action(&mut self) {
        self.parallel_handle.poll().apply(self);
    }

    pub fn enter_selected(&mut self) {
        let &id = self.content_stack.last().unwrap();
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        match id.t {
                            SongContentType::Menu => {
                                let s = self.get_song_mut(id);
                                let _opts = s.get_menu_options();
                                // let opt = opts[index]; // TODO: track with edit manager
                                // let action = s.apply_option(opt);
                                // action.apply(self);
                                todo!()
                            }
                            SongContentType::Edit => {
                                todo!()
                            }
                            SongContentType::Normal => {
                                panic!("normal song mode should not be in content stack")
                            }
                        }
                    }
                    ID::ContentProvider(id) => {
                        let t = id.get_content_type();
                        match t {
                            ContentProviderContentType::Normal => {
                                let cp = self.get_provider_mut(id);
                                let content_id = cp.get_selected();
                                match content_id {
                                    ID::Song(song_id) => {
                                        self.play_song(song_id);
                                        self.set_queue(id);
                                    }
                                    ID::ContentProvider(id) => {
                                        ContentHandlerAction::PushToContentStack { id: id.into() }.apply(self);
                                    }
                                }
                            }
                            ContentProviderContentType::Menu => {
                                let cp = self.get_provider_mut(id);
                                let action = cp.apply_selected_option(id);
                                action.apply(self);
                            }
                            ContentProviderContentType::Edit(e) => { 
                                let cp = self.get_provider_mut(id);
                                let action = cp.choose_selected_editable(id, e);
                                action.apply(self);
                            }
                        }
                    }
                }
            }
            GlobalContent::Log | GlobalContent::Notifier => {
                return
            }
        }
        self.app_action.queue(AppAction::UpdateDisplayContent { content: self.get_content_names() });
    }

    pub fn get_content_names(&self) -> Vec<String> {
        let &id = self.content_stack.last().unwrap();
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        let s = self.get_song(id);
                        s.get_content_names(id.t)
                    }
                    ID::ContentProvider(id) => {
                        let t = id.get_content_type();
                        let cp = self.get_provider(id);
                        let cn = cp.get_content_names(t);
                        cn.get(self)
                    }
                }
            }
            GlobalContent::Log => {
                // self.logs.clone()
                todo!()
            }
            GlobalContent::Notifier => {
                // self.notifs.clone()
                todo!()
            }
        }
    }

    pub fn get_selected_index(&mut self) -> &mut SelectedIndex {
        let &id = self.content_stack.last().unwrap();
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(..) => {
                        // &mut Default::default() // no need to save index for options
                        todo!()
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider_mut(id);
                        let t = id.get_content_type();
                        cp.get_selected_index(t)
                    }
                }
            }
            GlobalContent::Log | GlobalContent::Notifier => {
                // &mut Default::default() // no need to save index
                todo!()
            }
        }
    }

    pub fn open_menu_for_current(&mut self) {
        let &id = self.content_stack.last().unwrap();
        self.open_menu_for(id);
    }

    fn open_menu_for(&mut self, id: GlobalContent) {
        let id = match id {
            GlobalContent::ID(id) => GlobalContent::ID(
                match id {
                    ID::Song(id) => ID::Song({
                        let s = self.get_song(id);
                        if !s.has_menu() {return}
                        let mut id = id;
                        id.t = SongContentType::Menu;
                        id
                    }),
                    ID::ContentProvider(mut id) => ID::ContentProvider({
                        let cp = self.get_provider(id);
                        if !cp.has_menu() {return}
                        id.set_content_type(ContentProviderContentType::Menu);
                        id
                    }),
                }
            ),
            GlobalContent::Log | GlobalContent::Notifier => {
                return
            }
        };
        
        ContentHandlerAction::PushToContentStack { id: id.into() }.apply(self);
        self.app_action.queue(AppAction::UpdateDisplayContent { content: self.get_content_names() });
    }

    pub fn open_edit_for_current(&mut self) {
        let &id = self.content_stack.last().unwrap();
        self.open_edit_for(id);
    }

    fn open_edit_for(&mut self, id: GlobalContent) {
        let id = match id {
            GlobalContent::ID(id) => GlobalContent::ID(
                match id {
                    ID::Song(..) => {
                        todo!()
                    },
                    ID::ContentProvider(mut id) => ID::ContentProvider({
                        let cp = self.get_provider(id);
                        if !cp.has_editables() {return}
                        id.set_content_type(ContentProviderContentType::Edit(ContentProviderEditables::None));
                        id
                    }),
                }
            ),
            GlobalContent::Log | GlobalContent::Notifier => {
                return
            }
        };
        
        ContentHandlerAction::PushToContentStack { id: id.into() }.apply(self);
        self.app_action.queue(AppAction::UpdateDisplayContent { content: self.get_content_names() });
    }
    
    pub fn open_menu_for_selected(&mut self) {
        let &id = self.content_stack.last().unwrap();
        dbg!(&self.content_stack);
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(..) => {
                        // when a menu/something else for this song is already open
                        return
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        let id = cp.get_selected();
                        self.open_menu_for(id.into());
                    }
                }
            }
            GlobalContent::Log | GlobalContent::Notifier => {
                return
            }
        }
    }

    pub fn open_edit_for_selected(&mut self) {
        let &id = self.content_stack.last().unwrap();
        dbg!(&self.content_stack);
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(..) => {
                        todo!();
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        let id = cp.get_selected();
                        self.open_edit_for(id.into());
                    }
                }
            }
            GlobalContent::Log | GlobalContent::Notifier => {
                return
            }
        }
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

    pub fn get_logs(&self) -> &Vec<String> {
        &self.logger.entries
    }

    pub fn set_queue(&mut self, id: ContentProviderID) {
        self.active_queue = Some(id);
        let mp_id = match self.content_stack[0] {
            GlobalContent::ID(ID::ContentProvider(id)) => id,
            _ => panic!(), // 0th content_provider will always be main_provider
        };

        // TODO: bad code to find queue provider. think of a better soloution
        let mp = self.get_provider(mp_id);
        for cp_id in mp.providers.clone() {
            let cp = self.get_provider_mut(cp_id);
            if cp.cp_type == ContentProviderType::Queues {
                cp.add(id.into());
            }
        }
    }
    pub fn play_song(&mut self, id: SongID) {
        let song = self.songs.get(id).unwrap();
        let path = song.path();
        debug!("playing song {song:#?}");
        self.player.stop().unwrap();
        self.player.play(path.into()).unwrap();
        self.active_song = Some(id);
        let art = song.get_art();
        let action = art.load();
        action.apply(self);
    }
    pub fn toggle_song_pause(&mut self) {
        self.player.toggle_pause().unwrap();
    }
    pub fn next_song(&mut self) { // FIX: browsing around changes the next song instead of choosing the song next to the current song
        debug!("trying to play next");
        let id = match self.active_queue {
            Some(id) => id,
            None => return,
        };
        if !self.increment_selection_on(id.into()) {
            return
        }        
        let q = self.get_provider_mut(id);
        let song_id = q.get_selected();
        if let ID::Song(id) = song_id {
            self.play_song(id);
        }
    }
    pub fn prev_song(&mut self) {
        debug!("trying to play previous");
        let id = match self.active_queue {
            Some(id) => id,
            None => return,
        };
        if !self.increment_selection_on(id.into()) {
            return
        }        
        let q = self.get_provider_mut(id);
        let song_id = q.get_selected();
        if let ID::Song(id) = song_id {
            self.play_song(id);
        }
    }
    pub fn seek_song(&mut self, t: f64) {
        self.player.seek(t).unwrap();
    }

    pub fn get_app_action(&mut self) -> AppAction {
        std::mem::replace(&mut self.app_action, Default::default())
    }

    pub fn increment_selection(&mut self) {
        self.increment_selection_on(*self.content_stack.last().unwrap());
    }
    pub fn decrement_selection(&mut self) {
        self.decrement_selection_on(*self.content_stack.last().unwrap());
    }

    fn increment_selection_on(&mut self, id: GlobalContent) -> bool {
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider_mut(id);
                        let t = id.get_content_type();
                        cp.selection_increment(t)
                    }
                    ID::Song(..) => {
                        todo!()
                    }
                }        
            }
            _ => todo!()
        }
    }
    fn decrement_selection_on(&mut self, id: GlobalContent) -> bool {
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider_mut(id);
                        let t = id.get_content_type();
                        cp.selection_decrement(t)
                    }
                    ID::Song(..) => {
                        todo!()
                    }
                }        
            }
            _ => todo!()
        }
    }

    pub fn apply_typed(&mut self, content: String) {
        let &id = self.content_stack.last().unwrap();
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider_mut(id);
                        let action = cp.apply_typed(id, content);
                        action.apply(self);
                    }
                    ID::Song(..) => {
                        todo!()
                    }
                }
            }
            _ => panic!(), // should not happen
        }
    }
}

