


use crate::{
    song::Song,
    content_providers::{SongProvider, SPProvider},
    image_handler::ImageHandler,
    editors::{Yanker, UndoManager},
};
use musiplayer::Player;

pub struct ContentHandler {
    songs: ContentManager<Song>,
    song_providers: ContentManager<SongProvider>,
    sp_providers: ContentManager<SPProvider>,

    content_stack: Vec<ContentIdentifier>,
    yanker: Yanker,
    undo_manager: UndoManager,
    image_handler: ImageHandler,
    player: Player,
    active_song: Option<ContentIdentifier>,
}

impl ContentHandler {
    pub fn new() -> Self {
        Self {
            songs: ContentManager::new(),
            song_providers: ContentManager::new(),
            sp_providers: ContentManager::new(),
            content_stack: vec![],
            yanker: Yanker::new(),
            undo_manager: UndoManager::new(),
            image_handler: ImageHandler {},
            player: Player::new(),
            active_song: None,
        }
    }

    // TODO: temporary implimentation
    pub fn load() -> Self {
        Self::new()
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

    fn get_provider(&self, content_identifier: ContentIdentifier) -> Box<&dyn ContentProvider> {
        match content_identifier.content_type {
            // ContentType::Song => self.songs.get(content_identifier),
            ContentType::SongProvider => Box::new(self.song_providers.get(content_identifier).unwrap()),
            ContentType::SPProvider => Box::new(self.sp_providers.get(content_identifier).unwrap()),
            _ => panic!(),
        }
    }

    fn get_song(&self, content_identifier: ContentIdentifier) -> Option<&Song> {
        if !content_identifier.content_type.is_song() {return None}
        self.songs.get(content_identifier)
    }
}

struct ContentManager<T> {
    items: Vec<Option<ContentEntry<T>>>,
    
    // allocator
    empty_indices: Vec<usize>,
    generation: u64,
}

impl<T> ContentManager<T>
where T: Content
{
    fn new() -> Self {
        Self {
            items: vec![],
            empty_indices: vec![],
            generation: 0,
        }
    }

    fn dealloc(&mut self, content_identifier: ContentIdentifier) -> Option<T> {
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

    fn get(&self, content_identifier: ContentIdentifier) -> Option<&T> {
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

    // TODO: temporary inefficient implimentation
    // maybe use a min heap ?
    fn alloc(&mut self, item: T) -> ContentIdentifier {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentIdentifier {
    index: Option<usize>,
    generation: Option<u64>,
    pub content_type: ContentType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentType {
    Song,
    SongProvider,
    SPProvider,
    MainProvider,
    
    Edit, // edit/add fields like search_name, artist_name stuff
    Menu, // -yank, fetch artist from yt, download pl, delete
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
}

pub trait Content {
    fn get_content_type() -> ContentType;
}

pub trait ContentProvider {
    fn provide(&self) -> &Vec<ContentIdentifier>;
    fn provide_mut(&mut self) -> &mut Vec<ContentIdentifier>;
    
    /// panics if out of bounds
    fn swap(&mut self, a: usize, b:  usize) {
        self.provide_mut().swap(a, b)
    }

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
