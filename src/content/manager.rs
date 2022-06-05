
use std::marker::PhantomData;

use crate::{
    content::{
        providers::ContentProvider,
        song::Song,
    },
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SongID {
    id: ContentID<Song>,
}
impl SongID {
    fn from_id(id: ContentID<Song>) -> Self {
        Self { id }
    }
}
to_from_content_id!(SongID, Song);


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentProviderID {
    id: ContentID<ContentProvider>,
}
impl ContentProviderID {
    fn from_id(id: ContentID<ContentProvider>) -> Self {
        Self {
            id,
        }
    }
}
to_from_content_id!(ContentProviderID, ContentProvider);


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ID {
    Song(SongID),
    ContentProvider(ContentProviderID),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalContent {
    Notifier,
    ID(ID),
}
impl<T> From<T> for GlobalContent
where T: Into<ID>
{
    fn from(id: T) -> Self {
        Self::ID(id.into())
    }
}


#[derive(Debug)]
pub struct ContentManager<T, P> {
    items: Vec<Option<ContentEntry<T>>>,
    
    // allocator
    empty_indices: Vec<usize>,
    generation: u64,

    _phantom: PhantomData<P>,
}

impl<T, P> ContentManager<T, P>
    where
        T: Clone,
        P: From<ContentID<T>> + Into<ContentID<T>>
{
    pub fn new() -> Self {
        Self {
            items: vec![],
            empty_indices: vec![],
            generation: 0,
            _phantom: PhantomData,
        }
    }

    fn dealloc(&mut self, id: P) -> Option<T> {
        let id: ContentID<T> = id.into();
        self.generation += 1;
        self.empty_indices.push(id.index);

        match self.items.remove(id.index) {
            Some(s) => Some(s.val),
            None => None,
        }
    }

    pub fn get(&self, id: P) -> Option<&T> {
        let id: ContentID<T> = id.into();
        match self.items.get(id.index) {
            Some(Some(e)) =>  Some(&e.val),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: P) -> Option<&mut T> {
        let id: ContentID<T> = id.into();
        match self.items.get_mut(id.index) {
            Some(Some(e)) => Some(&mut e.val),
            _ => None,
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

    pub fn register(&mut self, id: P) {
        let id: ContentID<_> = id.into();
        match self.items.get_mut(id.index) {
            Some(Some(e)) => {
                e.id_counter += 1;
            }
            _ => panic!("cant register if its not there"),
        }
    } 

    pub fn unregister(&mut self, id: P) -> Option<T> {
        let id: ContentID<_> = id.into();
        match self.items.get_mut(id.index) {
            Some(Some(e)) => {
                e.id_counter -= 1;
                if e.id_counter == 0 {
                    self.dealloc(id.into())
                } else {
                    None
                }
            }
            _ => panic!("cant unregister if its not there"),
        }
    }

    /// panics if index > len
    fn set(&mut self, item: T, index: usize) -> P {
        if index < self.items.len() {
            self.generation += 1;
        }
        self.items.insert(index, Some(ContentEntry {
            val: item,
            generation: self.generation,
            id_counter: 1,
        }));
        P::from(ContentID {
            index,
            generation: self.generation,
            _phantom: PhantomData,
        })
    }
}

#[derive(Debug)]
struct ContentEntry<T> {
    val: T,
    generation: u64,
    id_counter: u32,
}

// TODO: maybe impliment some RC to auto yeet unneeded content
#[derive(Debug)]
pub struct ContentID<T> {
    index: usize,
    generation: u64,
    _phantom: PhantomData<T>,
}
impl<T> Clone for ContentID<T> {
    fn clone(&self) -> Self {
        Self { index: self.index, generation: self.generation, _phantom: PhantomData }
    }
}
impl<T> Copy for ContentID<T> {}
impl<T> PartialEq for ContentID<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}
impl<T> Eq for ContentID<T> {}
