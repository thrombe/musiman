
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
        song::{
            tagged_file_song::TaggedFileSong,
            traits::{
                SongTrait,
                Func,
                SongDisplay,
            },
        },
    },
};

#[derive(Debug)]
pub struct UntaggedFileSong {
    title: String,
    path: Cow<'static, str>,
}
impl UntaggedFileSong {
    pub fn from_file_path<'a>(path: Cow<'a, str>) -> Self {
        let song = Self {
            title: path.rsplit_terminator("/").next().unwrap().to_owned(),
            path: path.into_owned().into(),
        };
        song
    }
}

impl SongTrait for UntaggedFileSong {
    fn is_online(&self) -> bool {
        false
    }
    fn get_showable_info(&self) -> Box<dyn Iterator<Item = Cow<'static, str>>> {
        Box::new([
            format!("title: {}", self.title),
        ].into_iter().map(Into::into))
    }
    fn get_uri(&self, callback: Func) -> Result<ContentManagerAction> {
        callback(format!("file://{}", self.path))
    }
    fn play(&self) -> Result<ContentManagerAction> {
        self.get_uri(Box::new(|uri: String| {
            Ok(ContentManagerAction::PlaySongURI { uri })
        }))
    }
    fn show_art(&self) -> Result<ContentManagerAction> {
        Ok(TaggedFileSong::show_art_action(self.path.clone()))
    }

    fn as_display(&self) -> &dyn super::traits::SongDisplay {
        self
    }
}

impl SongDisplay for UntaggedFileSong {
    fn title(&self) -> &str {
        self.title.as_ref()
    }
}
