
#[allow(unused_imports)]
use crate::{dbg, debug};

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
    },
    image_handler::ImageHandler,
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
        PersistentContentID,
        TemporaryContentID,
        ID,
    },
};
use musiplayer::Player;

pub struct ContentHandler {
    // TODO: maybe try having just one ContentManager of enum of Song, ContentProvider, etc
    songs: ContentManager<Song, SongID>,
    content_providers: ContentManager<ContentProvider, PersistentContentID>,
    temp_content_providers: ContentManager<ContentProvider, TemporaryContentID>,
    db_handler: DBHandler,

    content_stack: Vec<GlobalContent>,
    yanker: Yanker,
    edit_manager: EditManager,
    image_handler: ImageHandler,
    pub player: Player,
    notifier: Notifier,
    logger: Logger,
    
    active_queue: Option<ContentProviderID>, // can also be a bunch of queues? like -> play all artists
    active_song: Option<SongID>,
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

pub enum Action {
    LoadContentManager {
        songs: Vec<Song>,
        content_providers: Vec<ContentProvider>,
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
    PushToContentStack {
        id: ContentProviderID,
    },
    EnableTyping,
    PopContentStack,
    Actions(Vec<Self>),
    SetSelectedIndex {
        index: usize,
    },
    None,
}
impl Into<Action> for Vec<Action> {
    fn into(self) -> Action {
        Action::Actions(self)
    }
}
impl Action {
    fn apply(self, ch: &mut ContentHandler) {
        match self {
            Self::LoadContentManager {songs, content_providers, loader_id} => {
                let mut loader = ch.get_provider(loader_id).clone();
                for s in songs {
                    let ci = ch.songs.alloc(s);
                    loader.songs.push(ci.into());
                }
                for cp in content_providers {
                    let id = if loader_id.is_temp() {
                        ch.temp_content_providers.alloc(cp).into()
                    } else {
                        ch.content_providers.alloc(cp).into()
                    };
                    loader.providers.push(id);
                }
                let old_loader = ch.get_provider_mut(loader_id);
                *old_loader = loader;        
            }
            Self::ReplaceContentProvider {..} => {
                todo!()
            }
            Self::AddCPToCP {id, cp, new_cp_content_type} => {
                let mut loaded_id: ContentProviderID = if id.is_temp() {
                    ch.temp_content_providers.alloc(cp).into()
                } else {
                    ch.content_providers.alloc(cp).into()
                };
                loaded_id.set_content_type(new_cp_content_type);
                let loader = ch.get_provider_mut(id);
                loader.add(loaded_id.into());
            }
            Self::PushToContentStack { id } => {
                ch.content_stack.push(id.into());
            }
            Self::EnableTyping => {
                todo!()
            }
            Self::PopContentStack => {
                ch.back();
            }
            Self::SetSelectedIndex { index: _ } => {
                todo!()
            }
            Self::Actions(actions) => {
                for action in actions {
                    action.apply(ch);
                }
            }
            Self::None => (),
        }
    }
}

pub enum GetNames {
    Names(Vec<String>),
    IDS(Vec<ID>),
}
impl GetNames {
    fn get_names(self, ch: &ContentHandler) -> Vec<String> {
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
    pub fn new() -> Self {
        let dbh = DBHandler::try_load();
        let cp = ContentManager::new();
        let mut tcp = ContentManager::new();
        let main_ci = tcp.alloc(ContentProvider::new_main_provider());
        Self {
            songs: ContentManager::new(),
            content_providers: cp,
            temp_content_providers: tcp,
            db_handler: dbh,
            content_stack: vec![main_ci.into()],
            yanker: Yanker::new(),
            edit_manager: EditManager::new(),
            image_handler: ImageHandler {},
            player: Player::new(),
            notifier: Notifier::new(),
            logger: Logger::new(),
            active_queue: None,
            active_song: None,
        }
    }

    // TODO: temporary implimentation
    pub fn load() -> Self {
        Self::new()
    }

    pub fn enter(&mut self, index: usize) {
        let &id = self.content_stack.last().unwrap();
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        match id.t {
                            SongContentType::Menu => {
                                self.apply_option(index);
                                self.back();
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
                                cp.selected_index = index;
                                let content_id = cp.get(index);
                                match content_id {
                                    ID::Song(song_id) => {
                                        self.play_song(song_id);
                                        self.set_queue(id);
                                    }
                                    ID::ContentProvider(id) => {
                                        self.content_stack.push(id.into());
                                        let action = self.get_provider_mut(id).load(id);
                                        action.apply(self)
                                    }
                                }
                            }
                            ContentProviderContentType::Menu => {
                                self.apply_option(index);
                                self.back();
                            }
                            ContentProviderContentType::Edit(e) => { 
                                let cp = self.get_provider_mut(id);
                                let action = cp.choose_editable(index, id, e);
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
    }

    pub fn back(&mut self) {
        // TODO: should also delete a few kinds of content
        // maybe use refrence counting to yeet the content without a identifier

        if self.content_stack.len() > 1 {
            self.content_stack.pop().unwrap();
        }
    }

    pub fn get_content_names(&mut self) -> Vec<String> {
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
                        cn.get_names(self)
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

    pub fn get_selected_index(&self) -> usize {
        let &id = self.content_stack.last().unwrap();
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(..) => {
                        0 // no need to save index for options
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        cp.selected_index
                    }
                }
            }
            GlobalContent::Log | GlobalContent::Notifier => {
                0 // no need to save index
            }
        }
    }

    pub fn apply_option(&mut self, index: usize) {
        let &id = self.content_stack.last().unwrap();
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        let s = self.get_song_mut(id);
                        let opts = s.get_menu_options();
                        let opt = opts[index]; // TODO: track with edit manager
                        debug!("choosing option {opt:#?}");
                        let action = s.apply_option(opt);
                        action.apply(self);
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider_mut(id);
                        let opts = cp.get_menu_options();
                        let opt = opts[index];
                        debug!("choosing option {opt:#?}");
                        let action = cp.apply_option(opt, id);
                        action.apply(self);
                    }
                }
            }
            _ => todo!(),
        }
        self.back();
    }

    pub fn open_menu_for_current(&mut self) -> bool {
        let &id = self.content_stack.last().unwrap();
        let id = match id {
            GlobalContent::ID(id) => GlobalContent::ID(
                match id {
                    ID::Song(id) => ID::Song({
                        let s = self.get_song(id);
                        if !s.has_menu() {return false}
                        let mut id = id;
                        id.t = SongContentType::Menu;
                        id
                    }),
                    ID::ContentProvider(id) => ID::ContentProvider({
                        let cp = self.get_provider(id);
                        if !cp.has_menu() {return false}
                        let id = match id {
                            ContentProviderID::PersistentContent { id, .. } => {
                                ContentProviderID::PersistentContent { id, t: ContentProviderContentType::Menu }
                            }
                            ContentProviderID::TemporaryContent {id, .. } => {
                                ContentProviderID::TemporaryContent { id, t: ContentProviderContentType::Menu }
                            }
                        };
                        id
                    }),
                }
            ),
            GlobalContent::Log | GlobalContent::Notifier => {
                return false
            }
        };
        
        self.content_stack.push(id);
        true
    }
    
    pub fn open_menu_for_selected(&mut self, index: usize) {
        let &id = self.content_stack.last().unwrap();
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(..) => {
                        // when a menu/something else for this song is already open
                        return
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        let id = cp.get(index);
                        self.content_stack.push(id.into());
                        if !self.open_menu_for_current() {
                            self.content_stack.pop();
                        }
                    }
                }
            }
            GlobalContent::Log | GlobalContent::Notifier => {
                return
            }
        }
    }

    fn get_provider(&self, id: ContentProviderID) -> &ContentProvider {
        match id {
            ContentProviderID::PersistentContent {id, ..} => {
                self.content_providers.get(id).unwrap()
            }
            ContentProviderID::TemporaryContent {id, ..} => {
                self.temp_content_providers.get(id).unwrap()
            }
        }
    }

    fn get_provider_mut(&mut self, id: ContentProviderID) -> &mut ContentProvider {
        match id {
            ContentProviderID::PersistentContent {id, .. } => {
                self.content_providers.get_mut(id).unwrap()
            }
            ContentProviderID::TemporaryContent {id, ..} => {
                self.temp_content_providers.get_mut(id).unwrap()
            }
        }
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
    }
    pub fn toggle_song_pause(&mut self) {
        self.player.toggle_pause().unwrap();
    }
    fn get_mut_queue(&mut self) -> Option<&mut ContentProvider> {
        self.active_queue.map(|id| self.get_provider_mut(id))
    }
    pub fn next_song(&mut self) {
        debug!("trying to play next");
        let q = match self.get_mut_queue() {
            Some(q) => q,
            None => return,
        };
        if q.selected_index < q.get_size()-1 {
            q.selected_index += 1;
        } else {
            return
        }        
        let song_id = q.get(q.selected_index);
        if let ID::Song(id) = song_id {
            self.play_song(id);
        }
    }
    pub fn prev_song(&mut self) {
        let q = match self.get_mut_queue() {
            Some(q) => q,
            None => return,
        };
        if q.selected_index > 0 {
            q.selected_index -= 1;
        } else {
            return
        }        
        let song_id = q.get(q.selected_index);
        if let ID::Song(id) = song_id {
            self.play_song(id);
        }
    }
    pub fn seek_song(&mut self, t: f64) {
        self.player.seek(t).unwrap();
    }
}

