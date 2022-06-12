
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use pyo3::{
    Python,
    types::{
        PyAny,
    },
    Py,
};
use anyhow::{Result};

pub mod manager;
pub mod action;

pub struct PyHandle {
    pub ytdl: Py<PyAny>,
    pub ytmusic: Py<PyAny>,
    pub thread: Py<PyAny>,
    pub json: Py<PyAny>,
    pub time: Py<PyAny>,
}
impl PyHandle {
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

/// assumes all lines have consistent exclusive spaces/tabs
pub fn fix_code_indentation(code: &str) -> String {
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

pub fn append_code(a: String, b: String) -> String {
    a.lines().chain(b.lines()).collect::<Vec<_>>().join("\n")
}
