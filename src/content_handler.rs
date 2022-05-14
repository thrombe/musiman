


use crate::{
    song::Song,
    content_providers::{SongProvider, SPProvider},
    image_handler::ImageHandler,
    editors::{Yanker, UndoManager},
    db_handler::DBHandler,
    notifier::Notifier,
};
use musiplayer::Player;

pub struct ContentHandler {
    songs: ContentManager<Song>,
    song_providers: ContentManager<SongProvider>,
    sp_providers: ContentManager<SPProvider>,
    // TODO: maybe have temp_spp: ContentManager<SPProvider>, with ways to move items from here to permanent?
    // or have a bool to know if its worth saving (like file explorer)
    main_provider: MainProvider,
    db_handler: DBHandler,

    content_stack: Vec<ContentIdentifier>,
    yanker: Yanker,
    undo_manager: UndoManager,
    image_handler: ImageHandler,
    player: Player,
    notifier: Notifier,
    
    active_queue: Option<ContentIdentifier>, // can also be a bunch of queues? like -> play all artists
    active_song: Option<ContentIdentifier>,
}

pub struct LoadEntry {
    pub s: Vec<Song>,
    pub sp: Vec<SongProvider>,
    pub spp: Vec<SPProvider>,
}
impl LoadEntry {
    fn load(self, ch: &mut ContentHandler, loader: ContentIdentifier) {
        let mut c = ch.sp_providers.get(loader).unwrap().clone();
        for s in self.s {
            let ci = ch.songs.alloc(s);
            c.sp_providers.push(ci);
        }
        for s in self.sp {
            let ci = ch.song_providers.alloc(s);
            c.sp_providers.push(ci);
        }
        for s in self.spp {
            let ci = ch.sp_providers.alloc(s);
            c.sp_providers.push(ci);
        }
        let c1 = ch.sp_providers.get_mut(loader).unwrap();
        *c1 = c;
    }
}

impl ContentHandler {
    pub fn new() -> Self {
        let mut dbh = DBHandler::try_load();
        Self {
            songs: ContentManager::new(),
            song_providers: dbh.song_providers(),
            sp_providers: dbh.sp_providers(),
            main_provider: dbh.main_provider(),
            db_handler: dbh,
            content_stack: vec![ContentIdentifier {index: None, generation: None, content_type: ContentType::MainProvider}],
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
                let ple = self.sp_providers.get_mut(ci_content).unwrap().load();
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
            ContentType::MainProvider => {
                self.get_names_from(self.main_provider.provide())
            }
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
            ContentType::SPProvider => {
                let spp = self.sp_providers.get(ci).unwrap();
                self.get_names_from(spp.provide())
            }
            ContentType::SongProvider => {
                let sp = self.song_providers.get(ci).unwrap();
                self.get_names_from(sp.provide())
            }
            _ => todo!() // TODO
        }
    }

    fn get_names_from(&self, ci_list: &Vec<ContentIdentifier>) -> Vec<String> {
        ci_list.into_iter().map(|&ci| {
            match ci.content_type { // unwrapping as these ci should not really be invalid if everyting goes right
                ContentType::SongProvider => {
                    self.song_providers.get(ci).unwrap().get_name()
                }
                ContentType::SPProvider => {
                    self.sp_providers.get(ci).unwrap().get_name()
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

    fn get_menu_options(&self, ci: ContentIdentifier) -> Vec<MenuOptions> {
        match ci.content_type {
            ContentType::MainProvider => {
                self.main_provider.get_menu_options()
            }
            _ => todo!()
        }
    }

    pub fn choose_option(&mut self, index: usize) {
        let &ci = self.content_stack.last().unwrap();
        if !ci.content_type.is_menu() {return}
        self.back();
        let &ci = self.content_stack.last().unwrap();
        let op = self.get_menu_options(ci)[index];

        match ci.content_type {
            ContentType::MainProvider => {
                match op {
                    MenuOptions::ADD_FILE_EXPLORER => {
                        let ci = self.sp_providers.alloc(SPProvider::new_file_explorer("/home/issac/".to_owned()));
                        self.main_provider.add(ci);
                    },
                    _ => panic!()
                }
            }
            _ => todo!()
        }
    }

    pub fn open_menu_for_current(&mut self) {
        let &ci = self.content_stack.last().unwrap();
        if !ci.content_type.has_menu() {
            self.content_stack.push(ContentIdentifier { index: None, generation: None, content_type: ContentType::Menu });
        }
    }
    
    pub fn open_menu_for_selected(&mut self, index: usize) {
        let &ci = self.content_stack.last().unwrap();
        if !ci.content_type.is_provider() {return}
        let ci = self.get_provider(ci).provide()[index];
        if ci.content_type.has_menu() {
            self.content_stack.push(ContentIdentifier { index: None, generation: None, content_type: ContentType::Menu });
        }
    }
    
    pub fn edit_selected(&mut self, index: usize) {
        let &ci = self.content_stack.last().unwrap();
        if !ci.content_type.is_provider() {return}
        let ci = self.get_provider(ci).provide()[index];
        if ci.content_type.has_edit_menu() {
            self.content_stack.push(ContentIdentifier { index: None, generation: None, content_type: ContentType::Edit });
        }
    }

    // TODO: maybe change the trait ComtentProvider to a enum?
    fn get_provider(&self, content_identifier: ContentIdentifier) -> Box<&dyn ContentProvider> {
        match content_identifier.content_type {
            // ContentType::Song => self.songs.get(content_identifier),
            ContentType::SongProvider => Box::new(self.song_providers.get(content_identifier).unwrap()),
            ContentType::SPProvider => Box::new(self.sp_providers.get(content_identifier).unwrap()),
            ContentType::MainProvider => Box::new(&self.main_provider),
            _ => panic!(),
        }
    }

    fn get_provider_mut(&mut self, content_identifier: ContentIdentifier) -> Box<&mut dyn ContentProvider> {
        match content_identifier.content_type {
            // ContentType::Song => self.songs.get(content_identifier),
            ContentType::SongProvider => Box::new(self.song_providers.get_mut(content_identifier).unwrap()),
            ContentType::SPProvider => Box::new(self.sp_providers.get_mut(content_identifier).unwrap()),
            ContentType::MainProvider => Box::new(&mut self.main_provider),
            _ => panic!(),
        }
    }

    fn get_song(&self, content_identifier: ContentIdentifier) -> Option<&Song> {
        if !content_identifier.content_type.is_song() {return None}
        self.songs.get(content_identifier)
    }
    fn get_song_mut(&mut self, content_identifier: ContentIdentifier) -> Option<&mut Song> {
        if !content_identifier.content_type.is_song() {return None}
        self.songs.get_mut(content_identifier)
    }

    pub fn play_song(&mut self, ci: ContentIdentifier) {
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
    pub fn next_song(&mut self) {
        let queue_ci = if self.active_queue.is_some() {self.active_queue.unwrap()} else {return};
        let q = self.song_providers.get_mut(queue_ci).unwrap();
        if q.selected_index < q.provide().len()-1 {
            q.selected_index += 1;
        } else {
            return
        }        
        let song_ci = q.provide()[q.selected_index];
        if !song_ci.content_type.is_song() {return}
        self.play_song(song_ci);
    }
    pub fn prev_song(&mut self) {
        let queue_ci = if self.active_queue.is_some() {self.active_queue.unwrap()} else {return};
        let q = self.song_providers.get_mut(queue_ci).unwrap();
        if q.selected_index > 0 {
            q.selected_index -= 1;
        } else {
            return
        }        
        let song_ci = q.provide()[q.selected_index];
        if !song_ci.content_type.is_song() {return}
        self.play_song(song_ci);
    }
    pub fn seek_song(&mut self, t: f64) {
        self.player.seek(t).unwrap();
    }
}

pub struct ContentManager<T> {
    items: Vec<Option<ContentEntry<T>>>,
    
    // allocator
    empty_indices: Vec<usize>,
    generation: u64,
}

// TODO: i dont having to check the type of content_id manually in every function
impl<T> ContentManager<T>
where T: Content
{
    pub fn new() -> Self {
        Self {
            items: vec![],
            empty_indices: vec![],
            generation: 0,
        }
    }

    pub fn dealloc(&mut self, content_identifier: ContentIdentifier) -> Option<T> {
        if T::get_content_type() != content_identifier.content_type {
            return None;
        }

        self.generation += 1;
        self.empty_indices.push(content_identifier.index.unwrap());

        match self.items.remove(content_identifier.index.unwrap()) {
            Some(s) => Some(s.val),
            None => None,
        }
    }

    pub fn get(&self, content_identifier: ContentIdentifier) -> Option<&T> {
        if T::get_content_type() != content_identifier.content_type {
            return None;
        }
        
        match self.items.get(content_identifier.index.unwrap()) {
            Some(e) => match e {
                Some(e) => Some(&e.val),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_mut(&mut self, content_identifier: ContentIdentifier) -> Option<&mut T> {
        if T::get_content_type() != content_identifier.content_type {
            return None;
        }
        
        match self.items.get_mut(content_identifier.index.unwrap()) {
            Some(e) => match e {
                Some(e) => Some(&mut e.val),
                None => None,
            },
            None => None,
        }
    }

    // TODO: temporary inefficient implimentation
    // maybe use a min heap ?
    pub fn alloc(&mut self, item: T) -> ContentIdentifier {
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
    fn set(&mut self, item: T, index: usize) -> ContentIdentifier {
        if index < self.items.len() {
            self.generation += 1;
        }
        self.items.insert(index, Some(ContentEntry {val: item, generation: self.generation}));
        ContentIdentifier {
            index: Some(index),
            generation: Some(self.generation),
            content_type: T::get_content_type(),
        }
    }
}

struct ContentEntry<T> {
    val: T,
    generation: u64,
}

// TODO: maybe impliment some RC to auto yeet unneeded content
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentIdentifier {
    index: Option<usize>,
    generation: Option<u64>,
    pub content_type: ContentType,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuOptions {
    ADD_ARTIST_PROVIDER,
    ADD_PLAYLIST_PROVIDER,
    ADD_QUEUE_PROVIDER,
    ADD_FILE_EXPLORER,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentType {
    Song,
    SongProvider,
    SPProvider,
    MainProvider,
    
    Edit, // edit/add fields like search_name, artist_name stuff
    Menu, // -yank, fetch artist from yt, download pl, delete
    Notifier,
}

pub struct MainProvider{
    providers: Vec<ContentIdentifier>,
    selected_index: usize,
}
impl ContentProvider for MainProvider {
    fn provide(&self) -> &Vec<ContentIdentifier> {
        &self.providers
    }

    fn provide_mut(&mut self) -> &mut Vec<ContentIdentifier> {
        &mut self.providers
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn update_index(&mut self, i: usize) {
        self.selected_index = i;
    }
}
impl MainProvider {
    pub fn new() -> Self {
        Self {
            providers: vec![],
            selected_index: 0,
        }
    }

    fn get_menu_options(&self) -> Vec<MenuOptions> {
        [
            MenuOptions::ADD_ARTIST_PROVIDER,
            MenuOptions::ADD_PLAYLIST_PROVIDER,
            MenuOptions::ADD_QUEUE_PROVIDER,
            MenuOptions::ADD_FILE_EXPLORER
        ].into_iter()
        .collect()
    }
}

impl ContentType {
    fn has_menu(self) -> bool {
        if [Self::Song, Self::SongProvider, Self::SPProvider].into_iter().any(|e| e == self) {
            true
        } else {
            false
        }
    }

    fn has_edit_menu(self) -> bool {
        if [Self::Song, Self::SongProvider].into_iter().any(|e| e == self) {
            true
        } else {
            false
        }
    }

    fn is_provider(self) -> bool {
        if [Self::SongProvider, Self::SPProvider, Self::MainProvider].into_iter().any(|e| e == self) {
            true
        } else {
            false
        }
    }

    fn is_song(self) -> bool {
        if Self::Song == self {
            true
        } else {
            false
        }
    }
    
    fn is_menu(self) -> bool {
        if Self::Menu == self {
            true
        } else {
            false
        }
    }
    
    fn is_main(self) -> bool {
        if Self::MainProvider == self {
            true
        } else {
            false
        }
    }
}

pub trait Content {
    fn get_content_type() -> ContentType;
    fn get_name(&self) -> &str;
}

pub trait ContentProvider {
    fn provide(&self) -> &Vec<ContentIdentifier>;
    fn provide_mut(&mut self) -> &mut Vec<ContentIdentifier>;
    
    /// panics if out of bounds
    fn swap(&mut self, a: usize, b:  usize) {
        self.provide_mut().swap(a, b)
    }

    fn selected_index(&self) -> usize;
    fn update_index(&mut self, i: usize);

    // TODO: reimpliment these for all of the diff types of content providers
    fn add(&mut self, content_identifier: ContentIdentifier) {
        self.provide_mut().push(content_identifier);
    }
    /// panics if out of bounds
    fn remove_using_index(&mut self, index: usize) -> ContentIdentifier {
        self.provide_mut().remove(index)
    }
    fn remove(&mut self, cid: ContentIdentifier) {
        self.provide_mut().iter().position(|&e| e == cid);
    }
}
