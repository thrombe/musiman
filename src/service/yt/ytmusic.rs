
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
        providers::{
            ContentProvider,
            ytalbum::YTAlbum,
        },
    },
};


// https://serde.rs/attributes.html
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct YTMusicSearchAlbum {
    pub title: Option<String>,
    pub browse_id: Option<String>,
    pub artists: Option<Vec<Option<YTMusicSearchArtist>>>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct YTMusicSearchArtist {
    pub name: Option<String>,
    pub id: Option<String>,
}
impl Into<ContentProvider> for YTMusicSearchAlbum {
    fn into(self) -> ContentProvider {
        if self.title.is_some() && self.browse_id.is_some() {
            YTAlbum::new_browse_id(self.title.unwrap(), self.browse_id.unwrap()).into()
        } else {
            panic!() // BAD: create a "borked" provider instead of panicing
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct YTMusicAlbum {
    pub title: Option<String>,
    pub artists: Option<Vec<Option<YTMusicSearchArtist>>>,
    pub audio_playlist_id: Option<String>,
    pub playlist_id: Option<String>,
    // tracks from here are not as useful as the ones from the playlist_id
}
impl Into<ContentProvider> for YTMusicAlbum {
    fn into(self) -> ContentProvider {
        if self.title.is_some() && (self.audio_playlist_id.is_some() || self.playlist_id.is_some()) {
            YTAlbum::new_playlist_id(
                self.title.unwrap(),
                if self.audio_playlist_id.is_some() {self.audio_playlist_id.unwrap()} else {self.playlist_id.unwrap()}
            ).into()
        } else {
            panic!() // BAD: create a "borked" provider instead of panicing
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct YTMusicSearchVideo {
    pub title: Option<String>,
    pub video_id: Option<String>,
    pub artists: Vec<YTMusicSearchArtist>,
}
impl Into<Song> for YTMusicSearchVideo {
    fn into(self) -> Song {
        YtSong {
            title: self.title.unwrap(),
            id: self.video_id.unwrap(),
        }.into()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct YTMusicSong {
    pub video_details: Option<YTMusicSongVideoDetails>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct YTMusicSongVideoDetails {
    pub video_id: Option<String>,
    pub title: Option<String>,
    pub channel_id: Option<String>,
    pub thumbnail: Option<YTMusicSongThumbnails>,
    pub author: Option<String>,
    pub microformat: Option<YTMusicSongVideoDetailsMicroformat>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct YTMusicSongThumbnails {
    pub thumbnails: Option<Vec<YTMusicSongThumbnail>>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct YTMusicSongThumbnail {
    pub url: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct YTMusicSongVideoDetailsMicroformat { // eh

}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct YTMusicSearchSong {
    pub title: Option<String>,
    pub album: Option<YTMusicSongSearchAlbum>,
    pub video_id: Option<String>,
    pub artists: Option<Vec<YTMusicSearchArtist>>,
    pub thumbnails: Option<Vec<YTMusicSongThumbnail>>,
}
impl Into<Song> for YTMusicSearchSong {
    fn into(self) -> Song {
        YtSong {
            title: self.title.unwrap(),
            id: self.video_id.unwrap(),
        }.into()
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct YTMusicSongSearchAlbum {
    pub name: Option<String>,
    pub id: Option<String>,
}

