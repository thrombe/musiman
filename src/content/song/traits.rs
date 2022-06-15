
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



pub type Func = Box<dyn FnOnce(String) -> Result<ContentManagerAction> + Send + Sync>;

impl<T> From<T> for Song
where T: SongTrait + 'static
{
    fn from(s: T) -> Self {
        Self::new(Box::new(s))
    }
}
pub trait SongTrait: Send + Sync + Debug {
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
    fn get_name(&self) -> &str;
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

