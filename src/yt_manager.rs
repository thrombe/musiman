
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use crate::{
    content_handler::{
        ContentHandlerAction,
    },
    content_manager::ContentProviderID,
    content_providers::{
        ContentProvider,
        // ContentProviderType,
    },
    song::{
        Song,
        SongMetadata,
    },
};

use pyo3::{
    Python,
    types::{
        IntoPyDict,
        PyAny,
    },
    Py,
};
use serde::{self, Serialize, Deserialize};
use serde_json;
use derivative::Derivative;
use anyhow::{Result, Context};
use std::{
    thread::{
        self,
        JoinHandle,
    },
    sync::{
        mpsc::{
            self,
            Receiver,
            Sender,
        },
    }
};

struct PyHandel {
    ytdl: Py<PyAny>,
    ytmusic: Py<PyAny>,
    thread: Py<PyAny>,
    json: Py<PyAny>,
    time: Py<PyAny>,
}
impl PyHandel {
    fn new(py: Python) -> Result<Self> {
        let headers_path = "/home/issac/0Git/musimanager/db/headers_auth.json";
        let ytmusic = py
        .import("ytmusicapi")?
        .getattr("YTMusic")?
        .call1((headers_path,))?
        .extract()?;
        let ext = "m4a";
        let path = "/home/issac/Music/";
        let code = format!("{{
            'format': 'bestaudio',
            'postprocessors': [{{
                'key': 'FFmpegExtractAudio',
                #'preferredquality': '160', # youtube caps at 160
                'preferredcodec': '{ext}',
            }}],
            'noplaylist': True,
            'quiet': True,
            'outtmpl': '{path}' + '%(id)s.%(ext)s',
            'verbose': False,
            'no_warnings': True,
            'noprogress': True,
            'geo_bypass': True,
            # 'skip_playlist_after_errors': 5,
            'extract_flat': 'in_playlist', # dont recursively seek for every video when in playlist
            # 'concurrent_fragment_downloads': 100,
        }}");
        // println!("{}", &code);
        let ytdl = py.import("yt_dlp")?
        .getattr("YoutubeDL")?
        .call1((py.eval(&code, None, None)?,))?
        .extract()?;
        let json = py
        .import("json")?
        .extract()?;
        let thread = py.import("threading")?
        .getattr("Thread")?
        .extract()?;
        let time = py.import("time")?
        .extract()?;
        Ok(Self {
            ytmusic,
            thread,
            json,
            ytdl,
            time,
        })
    }
}

// pyo3 cant do python in multiple rust threads at a time. so gotta make sure only one is active at a time
#[derive(Derivative)]
#[derivative(Debug)]
pub enum YTAction { // TODO: use cow for strings in actions?
    GetSong {
        url: String,
        #[derivative(Debug="ignore")]
        callback: Box<dyn Fn(String, String) -> ContentHandlerAction + Send + Sync>,
    },
    // TODO: more search actions
    // https://ytmusicapi.readthedocs.io/en/latest/reference.html#ytmusicapi.YTMusic.search
    AlbumSearch {
        term: String,
        loader: ContentProviderID,
    },
    VideoSearch {
        term: String,
        loader: ContentProviderID,
    },
    SongSearch {
        term: String,
        loader: ContentProviderID,
    },
    GetAlbumPlaylistId {
        browse_id: String,
        loader: ContentProviderID,
    },
    GetPlaylist {
        playlist_id: String,
        loader: ContentProviderID,
    },
}
impl YTAction {
    fn run(&mut self, py: Python, pyd: &Py<PyAny>, pyh: &mut PyHandel) -> Result<()> {
        dbg!("running ytaction", &self);
        let globals = [
            ("res", &*pyd),
            ("thread", &pyh.thread),
            ("ytdl", &pyh.ytdl),
            ("ytmusic", &pyh.ytmusic),
            ("json", &pyh.json),
        ].into_py_dict(py);
        " #// ? template
        def dbg_data():
            with open('config/temp/{file_name}.log', 'r') as f:
                data = f.read()
            return data
        def get_data():
            {code_here}
            data = json.dumps(data, indent=4)
            return data
        get_data = dbg_data # // dbg:
        ";
        let code = match self {
            Self::GetSong {url, ..} => {
                format!("
                    def dbg_data():
                        with open('config/temp/ytdl_song.log', 'r') as f:
                            data = f.read()
                        return data
                    def get_data():
                        ytdl_data = ytdl.extract_info(url='{url}', download=False)
                        data = json.dumps(ytdl_data, indent=4)
                        return data
                    #get_data = dbg_data # // dbg:
                ")
            }
            Self::AlbumSearch {term, ..} => { // TODO: allow to choose limit and ignore_spelling from ui too
                format!("
                    def dbg_data():
                        with open('config/temp/album_search.log', 'r') as f:
                            data = f.read()
                        return data
                    def get_data():
                        data = ytmusic.search('{term}', filter='albums', limit=75, ignore_spelling=True)
                        data = json.dumps(data, indent=4)
                        return data
                    #get_data = dbg_data # // dbg:
                ")
            }
            Self::VideoSearch {term, ..} => {
                format!("
                    def dbg_data():
                        with open('config/temp/video_search.log', 'r') as f:
                            data = f.read()
                        return data
                    def get_data():
                        data = ytmusic.search('{term}', filter='videos', limit=75, ignore_spelling=True)
                        data = json.dumps(data, indent=4)
                        return data
                    #get_data = dbg_data # // dbg:
                ")
            }
            Self::SongSearch {term, ..} => {
                format!("
                    def dbg_data():
                        with open('config/temp/song_search.log', 'r') as f:
                            data = f.read()
                        return data
                    def get_data():
                        data = ytmusic.search('{term}', filter='songs', limit=75, ignore_spelling=True)
                        data = json.dumps(data, indent=4)
                        return data
                    #get_data = dbg_data # // dbg:
                ")
            }
            Self::GetAlbumPlaylistId {browse_id, ..} => {
                format!("
                    def dbg_data():
                        with open('config/temp/get_album_playlist_id.log', 'r') as f:
                            data = f.read()
                        return data
                    def get_data():
                        album_data = ytmusic.get_album('{browse_id}')
                        data = json.dumps(album_data, indent=4)
                        return data
                    #get_data = dbg_data # // dbg:
                ")
            }
            Self::GetPlaylist {playlist_id, ..} => {
                format!("
                    def dbg_data():
                        with open('config/temp/get_playlist.log', 'r') as f:
                            data = f.read()
                        return data
                    def get_data():
                        data = ytdl.extract_info('{playlist_id}', download=False)
                        data = json.dumps(data, indent=4)
                        return data
                    #get_data = dbg_data # // dbg:
                ")
            }
        };
        let try_catch = fix_python_indentation("
            def try_catch(f):
                try:
                    res['data'] = f()
                except Exception as e:
                    import traceback
                    res['error'] = traceback.format_exc()
                res['found'] = True
            handle = thread(target=try_catch, args=[get_data])
            handle.start()
            #try_catch(get_data)
        ");
        let code = fix_python_indentation(&code);
        let code = append_python_code(code, try_catch);
        debug!("{code}");
        py.run(&code, Some(globals), None)?;
        Ok(())
    }

    fn resolve(&mut self, py: Python, pyd: &Py<PyAny>, _pyh: &mut PyHandel) -> Result<ContentHandlerAction> {
        dbg!("resolving YTAction", &self);
        let globals = [("res", pyd)].into_py_dict(py);
        let pyd = py.eval("res['data']", Some(globals), None)?.extract::<Py<PyAny>>()?;
        if py.eval("res['error'] != None", Some(globals), None)?.extract::<bool>()? {
            let err = py.eval("res['error']", Some(globals), None)?.extract::<String>()?;
            error!("{err}");
            return Ok(ContentHandlerAction::None); // ?
        }
        let action = match self {
            Self::GetSong {callback, ..} => {
                let res = pyd.extract::<String>(py)?;
                // debug!("{res}");
                let song = serde_json::from_str::<YtdlSong>(&res)?;
                // dbg!(&song);
                let best_thumbnail_url = song
                .thumbnails
                .context("")?
                .into_iter()
                .filter(|e| e.preference.is_some() && e.url.is_some())
                .reduce(|a, b| {
                    if a.preference.unwrap() > b.preference.unwrap() {
                        a
                    } else {
                        b
                    }
                }).context("")?.url.unwrap();
                
                // yanked and translated code from ytdlp github readme
                // https://github.com/yt-dlp/yt-dlp#use-a-custom-format-selector
                let best_video_ext = song
                .formats
                .as_ref()
                .context("")?
                .iter()
                .rev()
                .filter(|f| {
                    f.vcodec.is_some() &&
                    f.vcodec.as_ref().unwrap() != "none" &&
                    f.acodec.is_some() &&
                    f.acodec.as_ref().unwrap() == "none"
                })
                .next()
                .context("")?
                .ext
                .as_ref()
                .context("")?;
                let best_audio_url = song
                .formats
                .as_ref()
                .context("")?
                .iter()
                .rev()
                .filter(|f| {
                    f.acodec.is_some() &&
                    f.acodec.as_ref().unwrap() != "none" &&
                    f.vcodec.is_some() &&
                    f.vcodec.as_ref().unwrap() == "none" &&
                    f.ext.is_some() &&
                    f.ext.as_ref().unwrap() == best_video_ext
                })
                .next()
                .context("")?
                .url
                .as_ref()
                .context("")?
                .clone();
                callback(best_audio_url, best_thumbnail_url)
            }
            Self::AlbumSearch {loader, ..} => {
                let res = pyd.extract::<String>(py)?;
                // debug!("{res}");
                let albums = serde_json::from_str::<Vec<YTMusicSearchAlbum>>(&res);
                // dbg!(&albums);
                // let content_providers = albums?.into_iter().map(Into::into).collect();
                // dbg!(&content_providers);
                vec![
                    // ContentHandlerAction::LoadContentProvider {
                    //     songs: Default::default(),
                    //     content_providers,
                    //     loader_id: *loader,
                    // },
                    // ContentHandlerAction::RefreshDisplayContent,
                ].into()
            }
            Self::VideoSearch {loader, ..} => {
                let res = pyd.extract::<String>(py)?;
                // debug!("{res}");
                let videos = serde_json::from_str::<Vec<YTMusicSearchVideo>>(&res)?;
                let songs = videos.into_iter().map(Into::into).collect();
                vec![
                    ContentHandlerAction::LoadContentProvider {
                        songs,
                        content_providers: Default::default(),
                        loader_id: *loader,
                    },
                    ContentHandlerAction::RefreshDisplayContent,
                ].into()
            }
            Self::SongSearch {loader, ..} => {
                let res = pyd.extract::<String>(py)?;
                // debug!("{res}");
                let songs = serde_json::from_str::<Vec<YTMusicSearchSong>>(&res)?;
                // dbg!(&songs);
                let songs = songs.into_iter().map(Into::into).collect();
                vec![
                    ContentHandlerAction::LoadContentProvider {
                        songs,
                        content_providers: Default::default(),
                        loader_id: *loader,
                    },
                    ContentHandlerAction::RefreshDisplayContent,
                ].into()
            }
            Self::GetAlbumPlaylistId {loader, ..} => {
                // the data we get from here have songs not necessarily the music videos
                // but the data we get from the playlistId has the music videos
                // (music videos being the songs with album art rather than the ones with dances and stuff)
                let res = pyd.extract::<String>(py)?;
                // debug!("{res}");
                let ytm_album = serde_json::from_str::<YTMusicAlbum>(&res)?;
                // ContentHandlerAction::ReplaceContentProvider {
                //     old_id: *loader,
                //     cp: ytm_album.into(),
                // }
                None.into()
            }
            Self::GetPlaylist {loader, ..} => {
                let res = pyd.extract::<String>(py)?;
                // debug!("{res}");
                let playlist = serde_json::from_str::<YTDLPlaylist>(&res)?;
                vec![
                    ContentHandlerAction::LoadContentProvider {
                        loader_id: *loader,
                        songs: playlist.songs.into_iter().map(Into::into).collect(),
                        content_providers: Default::default(),
                    },
                    ContentHandlerAction::RefreshDisplayContent,
                ].into()
            }
        };
        Ok(action)
    }
}

// https://serde.rs/attributes.html
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct YTMusicSearchAlbum {
    title: Option<String>,
    browse_id: Option<String>,
    artists: Option<Vec<Option<YTMusicSearchArtist>>>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct YTMusicSearchArtist {
    name: Option<String>,
    id: Option<String>,
}
// impl Into<ContentProvider> for YTMusicSearchAlbum {
//     fn into(self) -> ContentProvider {
//         let loaded = self.browse_id.is_none();
//         let t = if self.browse_id.is_some() {
//             ContentProviderType::YTAlbum {
//                 browse_id: self.browse_id.unwrap(),
//             }
//         } else {
//             error!("borked data: {self:#?}");
//             ContentProviderType::Borked
//         };

//         ContentProvider::new(
//             self.title.unwrap().into(),
//             t,
//             loaded,
//         )
//     }
// }

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct YTMusicAlbum {
    title: Option<String>,
    artists: Option<Vec<Option<YTMusicSearchArtist>>>,
    audio_playlist_id: Option<String>,
    playlist_id: Option<String>,
    // tracks from here are not as useful as the ones from the playlist_id
}
// impl Into<ContentProvider> for YTMusicAlbum {
//     fn into(self) -> ContentProvider {
//         let mut loaded = false;
//         let t = if self.audio_playlist_id.is_some() {
//             ContentProviderType::YTAudioPlaylist {
//                 playlist_id: self.audio_playlist_id.unwrap(),
//             }
//         } else if self.playlist_id.is_some() {
//             ContentProviderType::YTPlaylist {
//                 playlist_id: self.playlist_id.unwrap(),
//             }
//         } else {
//             error!("borked data: {self:#?}");
//             loaded = true;
//             ContentProviderType::Borked
//         };

//         ContentProvider::new(
//             self.title.unwrap().into(),
//             t,
//             loaded,
//         )
//     }
// }

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct YTDLPlaylist {
    title: Option<String>,
    #[serde(rename(deserialize = "entries"))]
    songs: Vec<YTDLPlaylistSong>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct YTDLPlaylistSong {
    id: Option<String>,
    title: Option<String>,
    uploader: Option<String>,
    channel_id: Option<String>,
}
impl Into<Song> for YTDLPlaylistSong {
    fn into(self) -> Song {
        Song {
            metadata: SongMetadata::YT {
                title: self.title.unwrap(),
                id: self.id.unwrap(),
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct YTMusicSearchVideo {
    title: Option<String>,
    video_id: Option<String>,
    artists: Vec<YTMusicSearchArtist>,
}
impl Into<Song> for YTMusicSearchVideo {
    fn into(self) -> Song {
        Song {
            metadata: SongMetadata::YT {
                title: self.title.unwrap(),
                id: self.video_id.unwrap(),
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct YtdlAndYTMusicSong {
    ytdl: YtdlSong,
    ytmusic: YTMusicSong,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct YTMusicSong {
    video_details: Option<YTMusicSongVideoDetails>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct YTMusicSongVideoDetails {
    video_id: Option<String>,
    title: Option<String>,
    channel_id: Option<String>,
    thumbnail: Option<YTMusicSongThumbnails>,
    author: Option<String>,
    microformat: Option<YTMusicSongVideoDetailsMicroformat>,
}
#[derive(Serialize, Deserialize, Debug)]
struct YTMusicSongThumbnails {
    thumbnails: Option<Vec<YTMusicSongThumbnail>>,
}
#[derive(Serialize, Deserialize, Debug)]
struct YTMusicSongThumbnail {
    url: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct YTMusicSongVideoDetailsMicroformat { // eh

}

#[derive(Serialize, Deserialize, Debug)]
struct YtdlSong {
    id: Option<String>,
    title: Option<String>,
    thumbnails: Option<Vec<YtdlSongThumbnail>>,
    uploader: Option<String>,
    uploader_id: Option<String>,
    channel_id: Option<String>,
    tags: Option<Vec<String>>,
    album: Option<String>,
    artist: Option<String>,
    track: Option<String>,
    channel: Option<String>,
    creator: Option<String>,
    alt_title: Option<String>,
    availability: Option<String>,
    fulltitle: Option<String>,
    formats: Option<Vec<YtdlSongFormat>>,
}
#[derive(Serialize, Deserialize, Debug)]
struct YtdlSongThumbnail { // the fields always seem to be there, but just to be sure
    preference: Option<i32>,
    url: Option<String>,
}
#[derive(Serialize, Deserialize, Debug)]
struct YtdlSongFormat {
    ext: Option<String>,
    vcodec: Option<String>,
    acodec: Option<String>,
    url: Option<String>,
    audio_ext: Option<String>,
    video_ext: Option<String>,
    format: Option<String>,
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct YTMusicSearchSong {
    title: Option<String>,
    album: Option<YTMusicSongSearchAlbum>,
    video_id: Option<String>,
    artists: Option<Vec<YTMusicSearchArtist>>,
    thumbnails: Option<Vec<YTMusicSongThumbnail>>,
}
impl Into<Song> for YTMusicSearchSong {
    fn into(self) -> Song {
        Song {
            metadata: SongMetadata::YT {
                title: self.title.unwrap(),
                id: self.video_id.unwrap(),
            }
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
struct YTMusicSongSearchAlbum {
    name: Option<String>,
    id: Option<String>,
}


pub struct YTActionEntry {
    action: YTAction,
    pyd: Py<PyAny>,
}

#[derive(Debug)]
pub struct YTManager {
    sender: Sender<YTAction>,
    receiver: Receiver<ContentHandlerAction>,
    thread: JoinHandle<Result<()>>, // FIX: a crash in this thread silently kills the thread without communication
}

impl YTManager {
    pub fn new() -> Result<Self> {
        let (a_sender, a_receiver) = mpsc::channel();
        let (yt_sender, yt_receiver) = mpsc::channel();

        let thread = Self::init_thread(a_sender, yt_receiver);

        Ok(Self {
            sender: yt_sender,
            receiver: a_receiver,
            thread,
        })
    }

    pub fn poll(&mut self) -> ContentHandlerAction {
        match self.receiver.try_recv().ok() {
            Some(a) => {
                dbg!("action received");
                a
            },
            None => ContentHandlerAction::None
        }
    }

    pub fn run(&mut self, action: YTAction) -> Result<()> {
        dbg!(&action);
        self.sender.send(action).ok().context("")
    }

    fn init_thread(sender: Sender<ContentHandlerAction>, receiver: Receiver<YTAction>) -> JoinHandle<Result<()>> {
        let thread = thread::spawn(move || -> Result<()> {
            pyo3::prepare_freethreaded_python();
            let p = pyo3::Python::acquire_gil(); 
            let py = p.python();

            let pyh = &mut PyHandel::new(py)?;
            let mut actions = vec![];

            loop {
                // sleeping in python seems to not ruin speed. sleeping in rust somehow destroys it
                py.run("time.sleep(0.2)", Some([("time", &pyh.time)].into_py_dict(py)), None)?;
                match receiver.try_recv() {
                    Ok(a) => {
                        // choosing the default value of a dict so that the new data can be inserted into this dict, and
                        // the memory location does not change. res = data changes the memory location something something
                        // but res['data'] = data does what i want
                        let pyd = py.eval("{'data': None, 'found': False, 'error': None}", None, None)?.extract()?;
                        let entry = YTActionEntry {action: a, pyd };
                        actions.push(entry);
                        let a = actions.last_mut().unwrap();
                        a.action.run(py, &a.pyd, pyh)?;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        loop {
                            match actions
                            .iter()
                            .enumerate()
                            .map(|(i, a)|
                                Ok::<_, pyo3::PyErr>((i, py
                                .eval("a['found']", Some([("a", &a.pyd),].into_py_dict(py)), None)?
                                .extract::<bool>()?))
                            )
                            .map(Result::unwrap) // ? how do i pass this along
                            .filter(|(_, a)| *a)
                            .map(|(i, _)| i)
                            .next() {
                                Some(i) => {
                                    let mut a = actions.swap_remove(i);
                                    let action = a.action.resolve(py, &a.pyd, pyh)?;
                                    dbg!("sending action");
                                    sender.send(action)?;
                                    dbg!("action sent");
                                    }
                                None => break,
                            }
                        }
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        break;
                    }
                }
            };
             Ok(())
        });
        thread
    }
}

/// assumes all lines have consistent exclusive spaces/tabs
fn fix_python_indentation(code: &str) -> String {
    let line = match code.lines().find(|line| !line.trim().is_empty()) {
        Some(line) => line,
        None => return "".to_owned(),
    };
    let whitespace_chars = line.len() - line.trim_start().len();
    code
    .lines()
    .map(|line| 
        line
        .chars()
        .skip(whitespace_chars)
        .collect::<String>()
    )
    .map(|line| String::from(line) + "\n")
    .collect()
}

fn append_python_code(a: String, b: String) -> String {
    a.lines().chain(b.lines()).collect::<Vec<_>>().join("\n")
}

// pub fn test() -> Result<()> {
//     wierd_threading_test()?;
//     Ok(())
// }
// fn wierd_threading_test() -> Result<()> {
//     pyo3::prepare_freethreaded_python();
//     let p = pyo3::Python::acquire_gil(); 
//     let py = p.python();
//     let thread = py.import("threading")?
//     .getattr("Thread")?
//     .extract()?;
//     let enu = py.None();
//     let globals = [("thread", &thread), ("enu", &enu)].into_py_dict(py);
//     let code = "
// print(hex(id(enu)))
// def f():
//     global enu
//     print('ininnu')
//     print(hex(id(enu)))
//     import time
//     time.sleep(2)
//     enu = 42
// handle = thread(target=f, args=())
// handle.start()
// thread = handle
// print('enu', enu)
// print(hex(id(enu)))
//     ";
//     py.run(code, Some(globals), None)?;
//     let globals = [("thread", py.eval("thread", Some(globals), None)?.extract::<Py<PyAny>>()?),].into_py_dict(py);
//     let code = "
// #print(hex(id(enu)))
// print(thread)
// thread.join()
// #print('from other run', enu)
//     ";
//     py.run(code, Some(globals), None)?;
//     Ok(())
// }


// https://pyo3.rs/latest/memory.html
// https://pyo3.rs/main/memory.html#gil-bound-memory

// fn main1() -> Result<()> {
//     pyo3::prepare_freethreaded_python();
//     let p = pyo3::Python::acquire_gil();
//     let py = p.python();
//     let ytm = py.import("ytmusicapi")?;
//     let headers_path = "/home/issac/0Git/musimanager/db/headers_auth.json";
//     // let ytmusic = ytm.getattr("YTMusic")?.call1(<pyo3::types::PyTuple as PyTryFrom>::try_from(((headers_path)).to_object(py).as_ref(py)).unwrap())?;
//     let ytmusic = ytm.getattr("YTMusic")?.call1((headers_path,))?; // rust tuples with single object need a "," at the end
//     let py_json = py.import("json")?;
//     // get the Python object using py() or directly use Python object to create a new pool, when pool drops, all objects after the pool also drop
//     // make sure everything created after the pool does not have a refrence that lives longer
//     let _scope = unsafe{ytmusic.py().new_pool()};
//     // let py = scope.python();
//     let s = ytmusic.call_method1("get_song", ("AjesoBGztF8",))?;
//     let s = py_json.call_method1("dumps", (s,))?;
//     let mut s = serde_json::from_str::<serde_json::Value>(&s.to_string())?;
//     s.as_object_mut().context("NoneError")?.remove("playabilityStatus");
//     s.as_object_mut().context("NoneError")?.remove("streamingData");
//     s.as_object_mut().context("NoneError")?.remove("microformat");
//     dbg!(&s);
//     Ok(())
// }

