

use std::ops::{
    Deref,
    DerefMut,
};
use serde::{Serialize, Deserialize};

pub mod traits;
pub mod tagged_file_song;
pub mod untagged_file_song;
pub mod yt_song;

use traits::SongTrait;

#[derive(Debug, Serialize, Deserialize)]
pub struct Song(Box<dyn SongTrait>);
impl Song {
    pub fn new(s: Box<dyn SongTrait>) -> Self {
        Self(s)
    }
}

impl Deref for Song {
    type Target = Box<dyn SongTrait>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Song {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
