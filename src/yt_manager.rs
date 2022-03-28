use pyo3::{
    types::{PyAny},
    Py,
};
use anyhow::{Result};


pub struct YTManager {
    ytmusic: Py<PyAny>,
    ytdl: Py<PyAny>,
    json: Py<PyAny>,
}

impl YTManager {
    pub fn new() -> Result<Self> {
        pyo3::prepare_freethreaded_python();
        let p = pyo3::Python::acquire_gil(); 
        let py = p.python();
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
        .call1((py.eval(
            &code
            , None, None)?,))?
        .extract()?;
        let json = py
        .import("json")?
        .extract()?;

        Ok(Self {
            ytmusic,
            ytdl,
            json,
        })
    }
}

