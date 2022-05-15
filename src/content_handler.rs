
use std::marker::PhantomData;

use crate::{
    song::Song,
    content_providers::{ContentProvider, MenuOptions},
    image_handler::ImageHandler,
    editors::{Yanker, UndoManager},
    db_handler::DBHandler,
    notifier::Notifier,
};
use musiplayer::Player;

macro_rules! contentid_wrapper {
    ($e:ident, $t: ident) => {
        pub struct $e(ContentID<$t>);
        impl std::convert::From<ContentID<$t>> for $e {
            fn from(cid: ContentID<$t>) -> Self {
                Self(cid)
            }
        }
        impl std::convert::Into<ContentID<$t>> for $e {
            fn into(self) -> ContentID<$t> {self.0}
        }        
    };
}

contentid_wrapper!(SongID, Song);
contentid_wrapper!(PersistentContentID, ContentProvider);
contentid_wrapper!(TemporaryContentID, ContentProvider);
pub enum ID {
    Song(SongID),
    ContentProvider(ContentProviderID),
}
pub enum ContentProviderID {
    PersistentContent(PersistentContentID),
    TemporaryContent(TemporaryContentID),
}

impl From<SongID> for ID {
    fn from(id: SongID) -> Self {
        Self::Song(id)
    }
}
impl From<ContentProviderID> for ID {
    fn from(id: ContentProviderID) -> Self {
        Self::ContentProvider(id)
    }
}
impl From<PersistentContentID> for ID {
    fn from(id: PersistentContentID) -> Self {
        Self::ContentProvider(ContentProviderID::PersistentContent(id))
    }
}
impl From<TemporaryContentID> for ID {
    fn from(id: TemporaryContentID) -> Self {
        Self::ContentProvider(ContentProviderID::TemporaryContent(id))
    }
}
impl From<PersistentContentID> for ContentProviderID {
    fn from(id: PersistentContentID) -> Self {
        Self::PersistentContent(id)
    }
}
impl From<TemporaryContentID> for ContentProviderID {
    fn from(id: TemporaryContentID) -> Self {
        Self::TemporaryContent(id)
    }
}

pub struct ContentHandler {
    // TODO: maybe try having just one ContentManager of enum of Song, ContentProvider, etc
    songs: ContentManager<Song, SongID>,
    content_providers: ContentManager<ContentProvider, PersistentContentID>,
    temp_content_providers: ContentManager<ContentProvider, TemporaryContentID>,
    db_handler: DBHandler,
    content_type: ContentType,

    content_stack: Vec<ID>,
    yanker: Yanker,
    undo_manager: UndoManager,
    image_handler: ImageHandler,
    player: Player,
    notifier: Notifier,
    
    active_queue: Option<ContentProviderID>, // can also be a bunch of queues? like -> play all artists
    active_song: Option<SongID>,
}

pub struct LoadEntry {
    pub s: Vec<Song>,
    pub sp: Vec<ContentProvider>,
}
impl LoadEntry {
    fn load(self, ch: &mut ContentHandler, loader: ContentProviderID) {
        let mut c = match loader {
            ContentProviderID::PersistentContent(id) => {
                ch.content_providers.get(id)
            }
            ContentProviderID::TemporaryContent(id) => {
                ch.temp_content_providers.get(id)
            }
        }.unwrap().clone();
        for s in self.s {
            let ci = ch.songs.alloc(s);
            c.content.push(ci.into());
        }
        for s in self.sp {
            let ci = ch.content_providers.alloc(s);
            c.content.push(ci.into());
        }
        let c1 = match loader {
            ContentProviderID::PersistentContent(id) => {
                ch.content_providers.get_mut(id)
            }
            ContentProviderID::TemporaryContent(id) => {
                ch.temp_content_providers.get_mut(id)
            }
        }.unwrap();
        *c1 = c;
    }
}

impl ContentHandler {
    pub fn new() -> Self {
        let mut dbh = DBHandler::try_load();
        let cp = ContentManager::new();
        let mut tcp = ContentManager::new();
        let main_ci = tcp.alloc(ContentProvider::new_main_provider());
        Self {
            songs: ContentManager::new(),
            content_providers: cp,
            temp_content_providers: tcp,
            db_handler: dbh,
            content_type: ContentType::Normal,
            content_stack: vec![main_ci.into()],
            yanker: Yanker::new(),
            undo_manager: UndoManager::new(),
            image_handler: ImageHandler {},
            player: Player::new(),
            notifier: Notifier::new(),
            active_queue: None,
            active_song: None,
        }
    }

    // TODO: temporary implimentation
    pub fn load() -> Self {
        Self::new()
    }

    pub fn enter(&mut self, index: usize) {
        let &ci_provider = self.content_stack.last().unwrap();
        if !ci_provider.content_type.is_provider() {return}
        let p = self.get_provider_mut(ci_provider);
        p.update_index(index);
        let ci_content = p
        .provide()
        .get(index);
        if let Some(&ci_content) = ci_content {
            if ContentType::Song == ci_content.content_type {
                self.play_song(ci_content);
                self.active_queue = Some(ci_provider);
            } else {
                self.content_stack.push(ci_content);
                let ple = self.content_providers.get_mut(ci_content).unwrap().load();
                ple.map(|s| s.load(self, ci_content));
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
        let &ci = self.content_stack.last().unwrap();
        match ci.content_type {
            ContentType::Notifier => {
                self.notifier.notifs.clone()
            }
            ContentType::Menu => {
                let &ci = self.content_stack.get(self.content_stack.len()-2).unwrap();
                self.get_menu_options(ci)
                .into_iter()
                .map(|o| {
                    format!("{o:#?}")
                    .replace("_", " ")
                    .to_lowercase()
                }).collect()
            }
            ContentType::SongProvider => {
                let sp = self.content_providers.get(ci).unwrap();
                self.get_names_from(sp.provide())
            }
            _ => todo!() // TODO
        }
    }

    fn get_names_from(&self, ci_list: &Vec<ID>) -> Vec<String> {
        ci_list.into_iter().map(|&ci| {
            match ci.content_type { // unwrapping as these ci should not really be invalid if everyting goes right
                ContentType::SongProvider => {
                    self.content_providers.get(ci).unwrap().get_name()
                }
                ContentType::Song => {
                    self.songs.get(ci).unwrap().get_name()
                }
                _ => panic!()
            }.to_owned()
        }).collect()
    }

    pub fn get_selected_index(&self) -> usize {
        let &ci = self.content_stack.last().unwrap();
        if !ci.content_type.is_provider() {return 0}
        let p = self.get_provider(ci);
        p.selected_index()
    }

    fn get_menu_options(&self, ci: ID) -> Vec<MenuOptions /*or String? or Into<String>*/> {
        match ci {
            ID::Song(id) => {
                // song decides what kinda menu it has
                self.get_song(id).get_menu_options() // returns SongMenuOpts ??
                .into_iter().map(|o| o.to_string())
            }
            ID::ContentProvider(id) => {
                self.get_provider(id).get_menu_options()
                .into_iter().map(|o| o.to_string())
            }
        }
    }

    pub fn choose_option(&mut self, index: usize) {
        let &ci = self.content_stack.last().unwrap();
        // i need something that can say "widget is currently showing menu"
        // or just call this from enter() and enter can figure out easily is its menu or not??
        match ci {
            ID::Song(id) => {

            }
            ID::ContentProvider(id) => {

            }
        }
        if !ci.content_type.is_menu() {return}
        self.back();
        let &ci = self.content_stack.last().unwrap();
        let op = self.get_menu_options(ci)[index];

        match ci.content_type {
            ContentType::SongProvider => {
                match op {
                    MenuOptions::ADD_FILE_EXPLORER => {
                        let ci_fe = self.content_providers.alloc(
                            ContentProvider::new_file_explorer("/home/issac/daata/phon-data/.musi/IsBac/".to_owned())
                        );
                        let mp = self.content_providers.get_mut(ci).unwrap();
                        mp.add(ci_fe);
                    },
                    _ => panic!()
                }
            }
            _ => todo!()
        }
    }

    // pub fn open_menu_for_current(&mut self) {
    //     let &ci = self.content_stack.last().unwrap();
    //     if ci.content_type.has_menu() {
    //         self.content_stack.push(ContentID { index: None, generation: None, content_type: ContentType::Menu });
    //     }
    // }
    
    // pub fn open_menu_for_selected(&mut self, index: usize) {
    //     let &ci = self.content_stack.last().unwrap();
    //     if !ci.content_type.is_provider() {return}
    //     let ci = self.get_provider(ci).provide()[index];
    //     if ci.content_type.has_menu() {
    //         self.content_stack.push(ContentID { index: None, generation: None, content_type: ContentType::Menu });
    //     }
    // }
    
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
            ContentProviderID::PersistentContent(id) => {
                self.content_providers.get(id).unwrap()
            }
            ContentProviderID::TemporaryContent(id) => {
                self.temp_content_providers.get(id).unwrap()
            }
        }
    }

    fn get_provider_mut(&mut self, id: ContentProviderID) -> &mut ContentProvider {
        match id {
            ContentProviderID::PersistentContent(id) => {
                self.content_providers.get_mut(id).unwrap()
            }
            ContentProviderID::TemporaryContent(id) => {
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
                if let ContentProviderID::PersistentContent(id) = self.active_queue.unwrap() {
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
where P: From<ContentID<T>> + Into<ContentID<T>>
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentID<T> {
    index: usize,
    generation: u64,
    _phantom: PhantomData<T>,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentType {
    Normal,
    Edit, // edit/add fields like search_name, artist_name stuff
    Menu, // -yank, fetch artist from yt, download pl, delete
    Notifier,
}
