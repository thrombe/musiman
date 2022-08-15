
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use serde::{self, Serialize, Deserialize};



#[derive(Serialize, Deserialize, Debug)]
pub struct MusimanagerDB {
    artists: Vec<MusiArtist>,
    auto_search_artists: Vec<MusiArtist>,
    playlists: Vec<MusiSongProvider>,
    queues: Vec<MusiSongProvider>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MusiArtist {
    name: String,
    keys: Vec<String>,
    check_stat: bool,
    ignore_no_songs: bool,
    name_confirmation_status: bool,
    songs: Vec<MusiSong>,
    known_albums: Vec<MusiAlbum>,
    keywords: Vec<String>,
    non_keywords: Vec<String>,
    search_keywords: Vec<String>,
    last_auto_search: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MusiAlbum {
    name: String,
    browse_id: String,
    playlist_id: String, // not sure if optional
    songs: Vec<MusiSong>,
    artist_name: String,
    artist_keys: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MusiSong { // in python, everything here is marked optional
    title: String,
    key: String,
    artist_name: Option<String>,
    info: SongInfo,
    last_known_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SongInfo {
    titles: Vec<String>,
    video_id: String,
    duration: Option<f64>,
    tags: Vec<String>,
    thumbnail_url: String,
    album: Option<String>,
    artist_names: Vec<String>,
    channel_id: String,
    uploader_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MusiSongProvider {
    #[serde(rename(deserialize = "data_list"))]
    songs: Vec<MusiSong>,
    #[serde(rename(deserialize = "name"))]
    title: String,
    current_index: i64,
}

pub fn test() {
    use std::io::Read;
    
    let musitracker_path = "/home/issac/0Git/musimanager/db/musitracker.json";
    let mut musitracker = std::fs::File::open(musitracker_path).unwrap();
    let mut buf = String::new();
    musitracker.read_to_string(&mut buf).unwrap();
    let musidb = serde_json::from_str::<MusimanagerDB>(&buf);
    dbg!(&musidb);
}
