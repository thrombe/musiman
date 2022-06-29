
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use pyo3::{
    Python,
    Py,
    PyAny,
    types::{
        IntoPyDict,
        PyDict,
    },
};
use anyhow::{
    Result,
};

use std::{
    borrow::Cow,
    fmt::Debug,
    any::TypeId,
    collections::HashMap,
};

use crate::service::config::config;


pub struct PyHandle {
    map: HashMap<TypeId, Py<PyAny>>,
}
impl PyHandle {
    pub fn get_dict<'py: 'a, 'a>(&'a mut self, py: Python<'py>, items: &[Box<dyn PyItem>]) -> Result<&'a PyDict> {
        for i in items {
            if self.map.get(&i.type_id()).is_none() {
                self.map.insert(i.type_id(), i.get_item(py)?);
            };
        }
        let dict = items
        .iter()
        .map(|i| {
            (i.get_name(), self.map.get(&i.type_id()).unwrap())
        })
        .collect::<Vec<_>>()
        .into_py_dict(py);
        Ok(dict)
    }

    pub fn new() -> Result<Self> {
        Ok(Self {
            map: Default::default(),
        })
    }
}



pub type Items = Vec<Box<dyn PyItem>>;


// TODO: make a macro for this boilerplate
pub trait PyItem: Send + Sync + Debug {
    fn get_item(&self, py: Python) -> Result<Py<PyAny>>;
    fn type_id(&self) -> TypeId;
    fn get_name(&self) -> Cow<'static, str>;
}

impl<T: PyItem + 'static> From<T> for Box<dyn PyItem> {
    fn from(o: T) -> Self {
        Box::new(o)
    }
}


#[derive(Clone, Debug)]
pub struct YtMusic {
    name: Cow<'static, str>,
}
impl YtMusic {
    pub fn new<T: Into<Cow<'static, str>>>(name: T) -> Self {
        Self { name: name.into() }
    }
}
impl PyItem for YtMusic {
    fn get_item(&self, py: Python) -> Result<Py<PyAny>> {
        let headers_path = config().ytmusic_cookies_path.as_ref().unwrap().to_str().unwrap();
        let ytmusic = py
        .import("ytmusicapi")?
        .getattr("YTMusic")?
        .call1((headers_path,))?
        .extract()?;
        Ok(ytmusic)
    }
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    fn get_name(&self) -> Cow<'static, str> {
        self.name.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Ytdl {
    name: Cow<'static, str>,
}
impl Ytdl {
    pub fn new<T: Into<Cow<'static, str>>>(name: T) -> Self {
        Self { name: name.into() }
    }
}
impl PyItem for Ytdl {
    fn get_item(&self, py: Python) -> Result<Py<PyAny>> {
        let ext = config().prefered_song_ext.as_str();
        let path = config().music_path.to_str().unwrap();
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
        let ytdl = py.import("yt_dlp")?
        .getattr("YoutubeDL")?
        .call1((py.eval(&code, None, None)?,))?
        .extract()?;
        Ok(ytdl)
    }
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    fn get_name(&self) -> Cow<'static, str> {
        self.name.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Json {
    name: Cow<'static, str>,
}
impl Json {
    pub fn new<T: Into<Cow<'static, str>>>(name: T) -> Self {
        Self { name: name.into() }
    }
}
impl PyItem for Json {
    fn get_item(&self, py: Python) -> Result<Py<PyAny>> {
        let json = py
        .import("json")?
        .extract()?;
        Ok(json)
    }
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    fn get_name(&self) -> Cow<'static, str> {
        self.name.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Thread {
    name: Cow<'static, str>,
}
impl Thread {
    pub fn new<T: Into<Cow<'static, str>>>(name: T) -> Self {
        Self { name: name.into() }
    }
}
impl PyItem for Thread {
    fn get_item(&self, py: Python) -> Result<Py<PyAny>> {
        let thread = py.import("threading")?
        .getattr("Thread")?
        .extract()?;
        Ok(thread)
    }
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    fn get_name(&self) -> Cow<'static, str> {
        self.name.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Time {
    name: Cow<'static, str>,
}
impl Time {
    pub fn new<T: Into<Cow<'static, str>>>(name: T) -> Self {
        Self { name: name.into() }
    }
}
impl PyItem for Time {
    fn get_item(&self, py: Python) -> Result<Py<PyAny>> {
        let time = py.import("time")?
        .extract()?;
        Ok(time)
    }
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    fn get_name(&self) -> Cow<'static, str> {
        self.name.clone()
    }
}
