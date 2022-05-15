
use std::marker::PhantomData;

use crate::{
    song::{Song, SongContentType, SongMenuOptions},
    content_providers::{ContentProvider, MainContentProviderMenuOptions, ContentProviderContentType, ContentProviderMenuOptions},
    image_handler::ImageHandler,
    editors::{Yanker, EditManager},
    db_handler::DBHandler,
    notifier::Notifier,
};
use musiplayer::Player;

macro_rules! to_from_content_id {
    ($e:ident, $t: ident) => {
        impl std::convert::From<ContentID<$t>> for $e {
            fn from(id: ContentID<$t>) -> Self {
                Self::from_id(id)
            }
        }
        impl std::convert::Into<ContentID<$t>> for $e {
            fn into(self) -> ContentID<$t> {self.id}
        }        
    };
}

#[derive(Clone, Copy, Debug)]
#[derive(PartialEq, Eq)]
pub struct SongID {
    id: ContentID<Song>,
    pub t: SongContentType, 
}
impl SongID {
    fn from_id(id: ContentID<Song>) -> Self {
        Self { id, t: Default::default() }
    }
}
to_from_content_id!(SongID, Song);

#[derive(Clone, Copy, Debug)]
#[derive(PartialEq, Eq)]
pub struct PersistentContentID{
    id: ContentID<ContentProvider>,
}
impl PersistentContentID {
    fn from_id(id: ContentID<ContentProvider>) -> Self {
        Self { id }
    }
}
to_from_content_id!(PersistentContentID, ContentProvider);

#[derive(Clone, Copy, Debug)]
#[derive(PartialEq, Eq)]
pub struct TemporaryContentID {
    id: ContentID<ContentProvider>,
}
impl TemporaryContentID {
    fn from_id(id: ContentID<ContentProvider>) -> Self {
        Self { id }
    }
}
to_from_content_id!(TemporaryContentID, ContentProvider);

#[derive(Clone, Copy, Debug)]
#[derive(PartialEq, Eq)]
pub enum ID {
    Song(SongID),
    ContentProvider(ContentProviderID),
}
impl From<SongID> for ID {
    fn from(id: SongID) -> Self {
        Self::Song(id)
    }
}
impl<T> From<T> for ID
where T: Into<ContentProviderID>
{
    fn from(id: T) -> Self {
        Self::ContentProvider(id.into())
    }
}


#[derive(Clone, Copy, Debug)]
#[derive(PartialEq, Eq)]
pub enum ContentProviderID {
    PersistentContent{
        id: PersistentContentID,
        t: ContentProviderContentType,
    },
    TemporaryContent {
        id: TemporaryContentID,
        t: ContentProviderContentType,
    },
}
impl ContentProviderID {
    fn get_content_type(self) -> ContentProviderContentType {
        match self {
            Self::PersistentContent {t, ..} => {
                t
            }
            Self::TemporaryContent {t, ..} => {
                t
            }
        }
    }
}
impl From<PersistentContentID> for ContentProviderID {
    fn from(id: PersistentContentID) -> Self {
        Self::PersistentContent {
            id,
            t: Default::default(),
        }
    }
}
impl From<TemporaryContentID> for ContentProviderID {
    fn from(id: TemporaryContentID) -> Self {
        Self::TemporaryContent {
            id,
            t: Default::default()
        }
    }
}


#[derive(Clone, Copy, Debug)]
#[derive(PartialEq, Eq)]
pub enum GlobalContent {
    Notifier,
    Log,
    ID(ID),
}
impl<T> From<T> for GlobalContent
where T: Into<ID>
{
    fn from(id: T) -> Self {
        Self::ID(id.into())
    }
}

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
    player: Player,
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
        Self { entries: vec![] }
    }
    pub fn log(&mut self, s: &str) {
        self.entries.push(s.into())
    }
}

pub enum ActionEntry {
    LoadContentManager(LoadEntry),
    ReplaceContentProvider {old_id: ContentProviderID, cp: ContentProvider},
    AddCPToCP {id: ContentProviderID, cp: ContentProvider},
}
impl ActionEntry {
    fn apply(self, ch: &mut ContentHandler) {
        match self {
            Self::LoadContentManager(e) => {
                e.load(ch);
            }
            Self::ReplaceContentProvider {old_id, cp} => {
                todo!()
            }
            Self::AddCPToCP {id, cp} => {
                let loaded_id = ch.content_providers.alloc(cp); // TODO: check where to add this. in temp or perma
                let loader = ch.get_provider_mut(id);
                loader.add(loaded_id.into());
            }
        }
    }
}
pub struct LoadEntry {
    pub s: Vec<Song>,
    pub sp: Vec<ContentProvider>,
    pub loader: ContentProviderID,
}
impl LoadEntry {
    fn load(self, ch: &mut ContentHandler) {
        let mut c = match self.loader {
            ContentProviderID::PersistentContent {id, ..} => {
                ch.content_providers.get(id)
            }
            ContentProviderID::TemporaryContent {id, ..} => {
                ch.temp_content_providers.get(id)
            }
        }.unwrap().clone();
        for s in self.s {
            let ci = ch.songs.alloc(s);
            c.content.push(ci.into());
        }
        for s in self.sp {
            let ci = ch.content_providers.alloc(s); // TODO: check where to add this. in temp or perma
            c.content.push(ci.into());
        }
        let c1 = match self.loader {
            ContentProviderID::PersistentContent {id, ..} => {
                ch.content_providers.get_mut(id)
            }
            ContentProviderID::TemporaryContent {id, ..} => {
                ch.temp_content_providers.get_mut(id)
            }
        }.unwrap();
        *c1 = c;
    }
}
pub enum GetNames<'a> {
    Names(Vec<String>),
    IDS(&'a Vec<ID>),
}
impl GetNames<'_> {
    fn get_names(self, ch: &ContentHandler) -> Vec<String> {
        match self {
            Self::Names(names) => names,
            Self::IDS(ids) => {
                ids.iter().map(|&id| {
                    match id {
                        ID::Song(id) => {
                            ch.get_song(id).unwrap().get_name()
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
                                cp.update_index(index);
                                let i = cp.selected_index();
                                let content_id = cp.provide()[i];
                                match content_id {
                                    ID::Song(song_id) => {
                                        self.play_song(song_id);
                                        self.active_queue = Some(id);
                                    }
                                    ID::ContentProvider(id) => {
                                        self.content_stack.push(id.into());
                                        let action = self.get_provider_mut(id).load(id);
                                        if let Some(action) = action {
                                            action.apply(self)
                                        }
                                    }
                                }
                            }
                            ContentProviderContentType::Menu => {
                                self.apply_option(index);
                                self.back();
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
            let ci = self.content_stack.pop().unwrap();
        }
    }

    pub fn get_content_names(&mut self) -> Vec<String> {
        let &id = self.content_stack.last().unwrap();
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        let s = self.get_song(id).unwrap();
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
                    ID::Song(id) => {
                        0 // no need to save index for options
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        cp.selected_index()
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
                        let s = self.get_song_mut(id).unwrap();
                        let opts = s.get_menu_options();
                        let opt = opts[index]; // TODO: track with edit manager + logs
                        let action = s.apply_option(opt);
                        if let Some(action) = action {
                            action.apply(self);
                        }
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider_mut(id);
                        let opts = cp.get_menu_options();
                        let opt = opts[index];
                        let action = cp.apply_option(opt, id);
                        if let Some(action) = action {
                            action.apply(self);
                        }
                    }
                }
            }
            GlobalContent::Log | GlobalContent::Notifier => {
                panic!("should never happen");
            }
        }
        self.back();
    }

    pub fn open_menu_for_current(&mut self) -> bool {
        let &id = self.content_stack.last().unwrap();
        let id = match id {
            GlobalContent::ID(id) => GlobalContent::ID(
                match id {
                    ID::Song(id) => ID::Song({
                        let s = self.get_song(id).unwrap();
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
                    ID::Song(id) => {
                        // when a menu/something else for this song is already open
                        return
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        let id = cp.provide()[index];
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
    
    // pub fn edit_selected(&mut self, index: usize) {
    //     let &ci = self.content_stack.last().unwrap();
    //     if !ci.content_type.is_provider() {return}
    //     let ci = self.get_provider(ci).provide()[index];
    //     if ci.content_type.has_edit_menu() {
    //         self.content_stack.push(ContentID { index: None, generation: None, content_type: ContentType::Edit });
    //     }
    // }

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

    fn get_song(&self, content_identifier: SongID) -> Option<&Song> {
        self.songs.get(content_identifier)
    }
    fn get_song_mut(&mut self, content_identifier: SongID) -> Option<&mut Song> {
        self.songs.get_mut(content_identifier)
    }

    pub fn get_logs(&self) -> &Vec<String> {
        &self.logger.entries
    }

    pub fn play_song(&mut self, ci: SongID) {
        let song = self.songs.get(ci).unwrap();
        let path = song.path().to_owned();
        let path = format!("file://{path}"); // TODO: this is temp, the song should provide some kinda general path that can be uri or local path
        self.player.stop().unwrap();
        self.player.play(path).unwrap();
        self.active_song = Some(ci);
    }
    pub fn toggle_song_pause(&mut self) {
        self.player.toggle_pause().unwrap();
    }
    fn get_mut_queue(&mut self) -> Option<&mut ContentProvider> {
        let queue_ci = {
            if self.active_queue.is_some() {
                if let ContentProviderID::PersistentContent {id, ..} = self.active_queue.unwrap() {
                    id
                } else {
                    return None
                }
            } else {
                return None
            }
        };
        let q = self.content_providers.get_mut(queue_ci).unwrap();
        Some(q)
    }
    pub fn next_song(&mut self) {
        let q = match self.get_mut_queue() {
            Some(q) => q,
            None => return,
        };
        if q.selected_index < q.provide().len()-1 {
            q.selected_index += 1;
        } else {
            return
        }        
        let song_id = q.provide()[q.selected_index];
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
        let song_id = q.provide()[q.selected_index];
        if let ID::Song(id) = song_id {
            self.play_song(id);
        }
    }
    pub fn seek_song(&mut self, t: f64) {
        self.player.seek(t).unwrap();
    }
}

pub struct ContentManager<T, P> {
    items: Vec<Option<ContentEntry<T>>>,
    
    // allocator
    empty_indices: Vec<usize>,
    generation: u64,
    _phantom: PhantomData<P>,
}

impl<T, P> ContentManager<T, P>
where T: Clone, P: From<ContentID<T>> + Into<ContentID<T>>
{
    pub fn new() -> Self {
        Self {
            items: vec![],
            empty_indices: vec![],
            generation: 0,
            _phantom: PhantomData,
        }
    }

    pub fn dealloc(&mut self, content_identifier: P) -> Option<T> {
        let id: ContentID<T> = content_identifier.into();
        self.generation += 1;
        self.empty_indices.push(id.index);

        match self.items.remove(id.index) {
            Some(s) => Some(s.val),
            None => None,
        }
    }

    pub fn get(&self, content_identifier: P) -> Option<&T> {
        let id: ContentID<T> = content_identifier.into();
        match self.items.get(id.index) {
            Some(e) => match e {
                Some(e) => Some(&e.val),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_mut(&mut self, content_identifier: P) -> Option<&mut T> {
        let id: ContentID<T> = content_identifier.into();
        match self.items.get_mut(id.index) {
            Some(e) => match e {
                Some(e) => Some(&mut e.val),
                None => None,
            },
            None => None,
        }
    }

    // TODO: temporary inefficient implimentation
    // maybe use a min heap ?
    pub fn alloc(&mut self, item: T) -> P {
        self.empty_indices.sort_by(|a, b| b.partial_cmp(a).unwrap()); // sorting reversed
        match self.empty_indices.pop() {
            Some(i) => {
                self.set(item, i)
            },
            None => {
                self.set(item, self.items.len())
            }
        }
    }

    /// panics if index > len
    fn set(&mut self, item: T, index: usize) -> P {
        if index < self.items.len() {
            self.generation += 1;
        }
        self.items.insert(index, Some(ContentEntry {val: item, generation: self.generation}));
        P::from(ContentID {
            index: index,
            generation: self.generation,
            _phantom: PhantomData,
        })
    }
}

struct ContentEntry<T> {
    val: T,
    generation: u64,
}

// TODO: maybe impliment some RC to auto yeet unneeded content
#[derive(Debug)]
pub struct ContentID<T> {
    index: usize,
    generation: u64,
    _phantom: PhantomData<T>,
}
impl<T: Clone> Clone for ContentID<T> {
    fn clone(&self) -> Self {
        Self { index: self.index, generation: self.generation, _phantom: PhantomData }
    }
}
impl<T: Clone> Copy for ContentID<T> {}
impl<T> PartialEq for ContentID<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}
impl<T> Eq for ContentID<T> {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentType {
    Normal,
    Edit, // edit/add fields like search_name, artist_name stuff
    Menu, // -yank, fetch artist from yt, download pl, delete
    Notifier,
}
