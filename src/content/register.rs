
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use std::{marker::PhantomData, fmt::Debug};
use serde::{Deserialize, Serialize};
use derivative::Derivative;

use crate::{
    content::{
        providers::ContentProvider,
        song::Song,
    },
};

macro_rules! to_from_content_id {
    ($e:ident, $t:ident) => {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct SongID {
    id: ContentID<Song>,
}
impl SongID {
    fn from_id(id: ContentID<Song>) -> Self {
        Self { id }
    }
}
to_from_content_id!(SongID, Song);


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
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


#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
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
impl From<GlobalProvider> for GlobalContent {
    fn from(o: GlobalProvider) -> Self {
        match o {
            GlobalProvider::Notifier => Self::Notifier,
            GlobalProvider::ContentProvider(id) => Self::ID(id.into()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalProvider {
    Notifier,
    ContentProvider(ContentProviderID),
}
impl<T> From<T> for GlobalProvider
where T: Into<ContentProviderID>
{
    fn from(id: T) -> Self {
        Self::ContentProvider(id.into())
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
enum Operation {
    Insert,
    Remove,
    None,
}

#[derive(Derivative, Serialize, Deserialize, Clone)]
#[derivative(Debug)]
pub struct ContentRegister<T, P> {
    items: Vec<Option<ContentEntry<T>>>,
    
    // allocator
    empty_indices: Vec<usize>,
    generation: u64,
    last_operation: Operation,

    #[serde(skip_serializing, skip_deserializing, default = "Default::default")]
    #[derivative(Debug="ignore")]
    _phantom: PhantomData<P>,
}

impl<T, P> ContentRegister<T, P>
    where
        P: From<ContentID<T>> + Into<ContentID<T>>,
        T: Debug,
{
    pub fn new() -> Self {
        Self {
            items: vec![],
            empty_indices: vec![],
            generation: 0,
            last_operation: Operation::None,
            _phantom: PhantomData,
        }
    }

    fn dealloc(&mut self, id: P) -> Option<T> {
        let id: ContentID<T> = id.into();
        // self.generation += 1;
        self.empty_indices.push(id.index);

        match self.items.get_mut(id.index) {
            Some(s) => {
                // generational id only needs to go up after every remove operation that happens immediately after a insert operation
                if let Operation::Insert = self.last_operation {
                    self.generation += 1;
                }
                self.last_operation = Operation::Remove;
                Some(s.take().unwrap().val)
            }
            None => None,
        }
    }

    pub fn get(&self, id: P) -> Option<&T> {
        let id: ContentID<T> = id.into();
        match self.items.get(id.index) {
            Some(Some(e)) => {
                if e.generation == id.generation {
                    Some(&e.val)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: P) -> Option<&mut T> {
        let id: ContentID<T> = id.into();
        match self.items.get_mut(id.index) {
            Some(Some(e)) => {
                if e.generation == id.generation {
                    Some(&mut e.val)
                } else {
                    None
                }
            }
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
        self.last_operation = Operation::Insert;
        let entry = Some(ContentEntry {
            val: item,
            generation: self.generation,
            id_counter: 1,
        });
        if self.items.len() > index {
            self.items[index] = entry;
        } else {
            self.items.insert(index, entry);
        }
        P::from(ContentID {
            index,
            generation: self.generation,
            _phantom: PhantomData,
        })
    }

    pub fn get_id_count(&self, index: usize) -> Option<(ContentID<T>, u32)> {
        self.items.get(index).map(|e| e.as_ref().map(|e| (
            ContentID {
                index,
                generation: e.generation,
                _phantom: PhantomData,
            },
            e.id_counter.into(),
        ))).flatten()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ContentEntry<T> {
    val: T,
    generation: u64,
    id_counter: u32,
}

// TODO: maybe impliment some RC to auto yeet unneeded content
#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Debug, Hash)]
pub struct ContentID<T> {
    index: usize,
    generation: u64,

    #[serde(skip_serializing, skip_deserializing, default = "Default::default")]
    #[derivative(Debug="ignore")]
    #[derivative(Hash="ignore")]
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
