
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use serde::{self, Serialize, Deserialize};

use crate::{
    content::{
        song::{
            Song,
            yt_song::YtSong,
        },
    },
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct YTDLPlaylist {
    pub title: Option<String>,
    #[serde(rename(deserialize = "entries"))]
    pub songs: Vec<YTDLPlaylistSong>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct YTDLPlaylistSong {
    pub id: Option<String>,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub channel_id: Option<String>,
}
impl Into<Song> for YTDLPlaylistSong {
    fn into(self) -> Song {
        YtSong {
            title: self.title.unwrap(),
            id: self.id.unwrap(),
        }.into()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YtdlSong {
    pub id: Option<String>,
    pub title: Option<String>,
    pub thumbnails: Option<Vec<YtdlSongThumbnail>>,
    pub uploader: Option<String>,
    pub uploader_id: Option<String>,
    pub channel_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub track: Option<String>,
    pub channel: Option<String>,
    pub creator: Option<String>,
    pub alt_title: Option<String>,
    pub availability: Option<String>,
    pub fulltitle: Option<String>,
    pub formats: Option<Vec<YtdlSongFormat>>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct YtdlSongThumbnail { // the fields always seem to be there, but just to be sure
    pub preference: Option<i32>,
    pub url: Option<String>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct YtdlSongFormat {
    pub ext: Option<String>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub url: Option<String>,
    pub audio_ext: Option<String>,
    pub video_ext: Option<String>,
    pub format: Option<String>,
}
