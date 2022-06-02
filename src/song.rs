
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

use image::DynamicImage;
use lofty::{
    self,
    AudioFile,
    Accessor,
};
use anyhow::{
    Result,
};
use std::{
    path::PathBuf,
    fmt::Debug,
};
use serde::{
    Serialize,
    Deserialize,
};

use crate::{
    content_handler::{
        ContentHandlerAction, RustParallelAction,
    },
};


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Song {
    pub metadata: SongMetadata,
    // stype: SongType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SongMetadata {
    YT {
        title: String,
        id: String,
    },
    YTFile {
        id: String,
        path: String,
    },
    TaggedFile {
        path: String,
        title: String,
        artist: String,
        album: String,
        // duration: f32,
    },
    UntaggedFile {
        path: String,
    },
    Seperator,
    Borked,
}

#[derive(Debug, Clone, Copy)]
pub enum SongMenuOptions {}


pub enum SongArt {
    DynamicImage(DynamicImage),
    TaggedFile(PathBuf),
    YTSong(YTSongPath),
    ImageUrl(String),
    None,
}
impl Debug for SongArt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DynamicImage(..) => {
                write!(f, "SongArt::DynamicImage")
            }
            Self::TaggedFile(p) => {
                write!(f, "{p:#?}")
            }
            Self::YTSong(path) => {
                write!(f, "{path:#?}")
            }
            Self::ImageUrl(url) => {
                write!(f, "{url}")
            }
            Self::None => {
                write!(f, "SongArt::None")
            }
        }
    }
}

impl SongArt {
    pub fn load(self) -> ContentHandlerAction {
        match self {
            Self::DynamicImage(img) => {
                ContentHandlerAction::UpdateImage { img: img.into() }
            }
            Self::TaggedFile(path) => {
                RustParallelAction::ProcessAndUpdateImageFromSongPath { path }.into()
            }
            Self::YTSong(..) => {
                unreachable!();
            }
            Self::ImageUrl(url) => {
                RustParallelAction::ProcessAndUpdateImageFromUrl { url }.into()
            }
            Self::None => {
                None.into()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum SongPath {
    LocalPath(String),
    YTPath(YTSongPath),
}
#[derive(Debug, Clone)]
pub enum YTSongPath {
    Key(String),
    URL(String),    
}
impl ToString for SongPath {
    fn to_string(&self) -> String {
        match self {
            Self::LocalPath(s) => format!("file://{s}"),
            Self::YTPath(p) => p.to_string(),
        }
    }
}
impl ToString for YTSongPath {
    fn to_string(&self) -> String {
        match self {
            Self::Key(s) => format!("https://youtu.be/{s}"),
            Self::URL(p) => p.into(),
        }
    }
}
impl Into<String> for SongPath {
    fn into(self) -> String {
        self.to_string()
    }
}
impl Into<String> for YTSongPath {
    fn into(self) -> String {
        self.to_string()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SongType {
    YTOnline,
    YTOnDisk,
    UnknownOnDisk,
    Seperator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SongContentType {
    Menu,
    Normal,
    Edit,
}
impl Default for SongContentType {
    fn default() -> Self {
        Self::Normal
    }
}

impl Song {
    pub fn has_menu(&self) -> bool {
        true
    }

    pub fn get_menu_options(&self) -> Vec<SongMenuOptions> {
        vec![]
    }
    
    pub fn apply_option(&mut self, opt: SongMenuOptions) -> ContentHandlerAction {
        match opt {

        }
    }

    pub fn get_name(&self) -> &str {
        match &self.metadata {
            SongMetadata::TaggedFile { title, .. } => {
                title
            }
            SongMetadata::UntaggedFile { path } => {
                path.rsplit_terminator("/").next().unwrap()
            }
            SongMetadata::YT {title, ..} => {
                title
            }
            _ => panic!()
        }
    }

    pub fn get_content_names(&self, t: SongContentType) -> Vec<String> {
        match t {
            SongContentType::Menu => {
                self.get_menu_options()
                .into_iter()
                .map(|o| {
                    format!("{o:#?}")
                    .replace("_", " ")
                    .to_lowercase()
                })
                .collect()
            }
            SongContentType::Edit => {
                // send_vec_of_self_details
                todo!()
            }
            SongContentType::Normal => {
                panic!("no content names in song Normal mode")
            }
        } 
    }
    
    pub fn from_file(path: String) -> Result<Self> {
        let tf = lofty::read_from_path(&path, true)?;
        let _ = log_song(&path);
        let tags = tf.primary_tag();
        if tags.is_some() {
            let tags = tags.unwrap();
            let artist = tags.artist();
            let title = tags.title();
            let album = tags.album();
            if artist.is_some() && title.is_some() {
                return Ok(Self {
                    metadata: SongMetadata::TaggedFile {
                        path,
                        title: title.unwrap().into(),
                        artist: title.unwrap().into(),
                        album: album.unwrap_or("none").into(),
                    }
                })
            }
        }
        Ok(Self {
            metadata: SongMetadata::UntaggedFile {path},
        })
    }

    pub fn get_art(&self) -> SongArt {
        match &self.metadata {
            SongMetadata::YT { id, .. } => {
                SongArt::YTSong(YTSongPath::Key(id.clone()))
            }
            SongMetadata::TaggedFile { path, .. } => {
                SongArt::TaggedFile(PathBuf::from(path))
            }
            _ => todo!(),
        }
    }
    
    pub fn path(&self) -> SongPath {
        match &self.metadata {
            SongMetadata::TaggedFile { path, .. } => {
                SongPath::LocalPath(path.into())
            }
            SongMetadata::UntaggedFile { path } => {
                SongPath::LocalPath(path.into())
            }
            SongMetadata::YTFile { path , ..} => {
                SongPath::LocalPath(path.into())
            }
            SongMetadata::YT { id, .. } => {
                SongPath::YTPath(YTSongPath::Key(id.clone()))
            }
            _ => panic!()
        }
    }
}

/// a function i used for checking what is returned by lofty
fn log_song(path: &str) -> Result<()> {
    debug!("logging song {path}");
    let probe = lofty::Probe::open(&path)?;
    let file_type = probe.file_type();
    // https://docs.rs/lofty/latest/lofty/struct.TaggedFile.html
    let tagged_file = probe.read(true)?;
    let properties = tagged_file.properties();
    // apparently a file can have multiple tags in it
    let tags = tagged_file
    .tags().into_iter()
    .map(lofty::Tag::items)
    .map(|e| e.iter()).flatten()
    .map(|e| (format!("{:#?}", e.key()), e.value().text().unwrap()))
    .collect::<Vec<_>>()
    ;
    let pics = tagged_file
    .tags().into_iter()
    .map(lofty::Tag::pictures)
    .collect::<Vec<_>>()
    ;
    dbg!(file_type, properties, tags, pics);
    Ok(())
}
