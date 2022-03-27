


use crate::{
    song::Song,
    content_providers::{SongProvider, SPProvider},
    image_handler::ImageHandler,
};
use musiplayer::Player;



pub struct ContentHandler {
    songs: ContentManager<Song>,
    song_providers: ContentManager<SongProvider>,
    sp_providers: ContentManager<SPProvider>,

    content_stack: Vec<ContentIndex>,
    image_handler: ImageHandler,
    player: Player,
    active_song: Option<ContentIndex>,
}

impl ContentHandler {
    pub fn new() -> Self {
        Self {
            songs: ContentManager::new(),
            song_providers: ContentManager::new(),
            sp_providers: ContentManager::new(),
            content_stack: vec![],
            image_handler: ImageHandler {},
            player: Player::new(),
            active_song: None,
        }
    }

    // TODO: temporary implimentation
    pub fn load() -> Self {
        Self::new()
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

    fn dealloc(&mut self, content_index: ContentIndex) -> Option<T> {
        if T::get_content_type() != content_index.content_type {
            return None;
        }

        self.generation += 1;
        self.empty_indices.push(content_index.index);

        match self.items.remove(content_index.index) {
            Some(s) => Some(s.val),
            None => None,
        }
    }

    fn get(&self, content_index: ContentIndex) -> Option<&T> {
        if T::get_content_type() != content_index.content_type {
            return None;
        }
        
        match self.items.get(content_index.index) {
            Some(e) => match e {
                Some(e) => Some(&e.val),
                None => None,
            },
            None => None,
        }
    }

    // TODO: temporary inefficient implimentation
    // maybe use a min heap ?
    fn alloc(&mut self, item: T) -> ContentIndex {
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
    fn set(&mut self, item: T, index: usize) -> ContentIndex {
        if index < self.items.len() {
            self.generation += 1;
        }
        self.items.insert(index, Some(ContentEntry {val: item, generation: self.generation}));
        ContentIndex {
            index: index,
            generation: self.generation,
            content_type: T::get_content_type(),
        }
    }
}

struct ContentEntry<T> {
    val: T,
    generation: u64,
}

struct ContentIndex {
    index: usize,
    generation: u64,
    content_type: ContentType,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContentType {
    Song,
    SongProvider,
    SPProvider,
}

pub trait Content {
    fn get_content_type() -> ContentType;
}
