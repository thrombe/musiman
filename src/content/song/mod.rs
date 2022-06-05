

#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};


use image::DynamicImage;
use lofty::{
    self,
    AudioFile,
    TaggedFile,
    ItemKey,
};
use anyhow::{
    Result,
};
use derivative::Derivative;
use std::{
    path::PathBuf,
    fmt::Debug,
};
use serde::{
    Serialize,
    Deserialize,
};


use crate::{
    content::action::{
        ContentHandlerAction,
        RustParallelAction,
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


#[derive(Derivative)]
#[derivative(Debug)]
pub enum SongArt {
    DynamicImage {
        #[derivative(Debug="ignore")]
        img: DynamicImage,
    },
    TaggedFile(PathBuf),
    YTSong(YTSongPath),
    ImageUrl(String),
    None,
}
impl SongArt {
    pub fn load(self) -> ContentHandlerAction {
        match self {
            Self::DynamicImage {img} => {
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
        let st: TaggedSong = tf.into();
        let title = st.title();
        let album = st.album();
        let artist = st.artist();
        if title.is_some() && album.is_some() && artist.is_some() {
            return Ok(Self {
                metadata: SongMetadata::TaggedFile {
                    path,
                    title: title.unwrap().to_owned(),
                    album: album.unwrap().to_owned(),
                    artist: artist.unwrap().to_owned(),
                }
            })
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

struct TaggedSong(TaggedFile);
impl From<TaggedFile> for TaggedSong {
    fn from(f: TaggedFile) -> Self {
        Self(f)
    }
}
impl TaggedSong {
    fn artist(&self) -> Option<&str> {
        self.get_val(&ItemKey::TrackArtist)
    }
    fn title(&self) -> Option<&str> {
        self.get_val(&ItemKey::TrackTitle)
    }
    fn album(&self) -> Option<&str> {
        self.get_val(&ItemKey::AlbumTitle)
    }
    fn get_val(&self, key: &ItemKey) -> Option<&str> {
        self.0
        .tags()
        .iter()
        .map(lofty::Tag::items)
        .map(|t| t.iter())
        .flatten()
        .filter(|t| t.key() == key)
        .find_map(|t| t.value().text())
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
    .tags().iter()
    .map(lofty::Tag::items)
    .map(|e| e.iter()).flatten()
    .map(|e| (format!("{:#?}", e.key()), e.value().text().unwrap()))
    .collect::<Vec<_>>()
    ;
    let pics = tagged_file
    .tags().iter()
    .map(lofty::Tag::pictures)
    .collect::<Vec<_>>()
    ;
    let tag_type = tagged_file.primary_tag_type();
    dbg!(file_type, properties, tags, pics, tag_type);
    Ok(())
}
