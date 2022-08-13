
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};


use std::{
    fmt::Debug,
    borrow::Cow,
};
use anyhow::{
    Result,
};
use typetag;

use crate::{
    content::{
        manager::{
            action::{
                ContentManagerAction,
            },
        },
        song::Song,
    },
};


pub trait SongClone {
    fn song_clone(&self) -> Box<dyn SongTrait>;
}

impl<T> SongClone for T
    where T: 'static + Clone + Debug + SongTrait
{
    fn song_clone(&self) -> Box<dyn SongTrait> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn SongTrait> {
    fn clone(&self) -> Self {
        self.song_clone()
    }
}


pub type Func = Box<dyn FnOnce(String) -> Result<ContentManagerAction> + Send + Sync>;

impl<T> From<T> for Song
where T: SongTrait + 'static
{
    fn from(s: T) -> Self {
        Self::new(Box::new(s))
    }
}

#[typetag::serde(tag = "type")]
pub trait SongTrait: Send + Sync + Debug + SongClone {
    fn play(&self) -> Result<ContentManagerAction>;
    // song might have to get the uri from the interwebs, so cant directly retrun a string
    fn get_uri(&self, callback: Func) -> Result<ContentManagerAction>;

    fn get_art(&self) -> MusicArt {
        panic!()
    }
    fn show_art(&self) -> Result<ContentManagerAction>;

    fn is_online(&self) -> bool;
    fn save_to_path(&self, _: &str) {
        unreachable!()
    }

    fn get_all_info(&self) -> Box<dyn Iterator<Item = Cow<'static, str>>> {
        self.get_showable_info()
    }
    fn get_showable_info(&self) -> Box<dyn Iterator<Item = Cow<'static, str>>>;

    fn as_display(&self) -> &dyn SongDisplay;
}

pub trait Playable {
    fn play(&self) -> ContentManagerAction;
}


pub enum MusicArt {

}

pub trait Artistic {
    fn get_art(&self) -> MusicArt;
    fn show_art(&self) -> ContentManagerAction;
}

pub trait Online {

}

pub trait Showable {

}

pub trait SongDisplay { // nothing is gonna be 'static str here, so not bothering to do Cow s here
    fn title(&self) -> &str;
    fn artist(&self) -> Option<&str> {
        None
    }
    fn album(&self) -> Option<&str> {
        None
    }
}

use crate::content::providers::traits::{Menu, Editable};
