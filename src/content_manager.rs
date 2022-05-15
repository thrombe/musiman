
use std::marker::PhantomData;

use crate::{
    song::{
        Song,
        SongContentType,
    },
    content_providers::{
        ContentProvider,
        ContentProviderContentType,
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
    pub t: SongContentType, 
}
impl SongID {
    fn from_id(id: ContentID<Song>) -> Self {
        Self { id, t: Default::default() }
    }
}
to_from_content_id!(SongID, Song);


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistentContentID {
    id: ContentID<ContentProvider>,
}
impl PersistentContentID {
    fn from_id(id: ContentID<ContentProvider>) -> Self {
        Self { id }
    }
}
to_from_content_id!(PersistentContentID, ContentProvider);


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporaryContentID {
    id: ContentID<ContentProvider>,
}
impl TemporaryContentID {
    fn from_id(id: ContentID<ContentProvider>) -> Self {
        Self { id }
    }
}
to_from_content_id!(TemporaryContentID, ContentProvider);


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
impl<T> From<T> for ID
where T: Into<ContentProviderID>
{
    fn from(id: T) -> Self {
        Self::ContentProvider(id.into())
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    pub fn get_content_type(self) -> ContentProviderContentType {
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


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
            index,
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
