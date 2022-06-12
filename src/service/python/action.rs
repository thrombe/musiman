
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use pyo3::{
    Python,
    types::{
        IntoPyDict,
        PyAny,
    },
    Py,
};
use serde_json;
use derivative::Derivative;
use anyhow::{Result, Context};

use crate::{
    content::{
        action::ContentHandlerAction,
        manager::ContentProviderID,
    },
    service::{
        python::{
            fix_python_indentation,
            append_python_code,
            PyHandel,
        },
        yt::{
            ytdl::*,
            ytmusic::*,
        },
    },
};


// BAD: rename tp Py***

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
    pub fn run(&mut self, py: Python, pyd: &Py<PyAny>, pyh: &mut PyHandel) -> Result<()> {
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

    pub fn resolve(&mut self, py: Python, pyd: &Py<PyAny>, _pyh: &mut PyHandel) -> Result<ContentHandlerAction> {
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
                let content_providers = albums?.into_iter().map(Into::into).collect();
                // dbg!(&content_providers);
                vec![
                    ContentHandlerAction::LoadContentProvider {
                        songs: Default::default(),
                        content_providers,
                        loader_id: *loader,
                    },
                    ContentHandlerAction::RefreshDisplayContent,
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
                ContentHandlerAction::ReplaceContentProvider {
                    old_id: *loader,
                    cp: ytm_album.into(),
                }
                // None.into()
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
