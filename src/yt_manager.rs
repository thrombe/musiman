use pyo3::{
    types::{IntoPyDict, PyAny},
    Py,
};
use serde;
use serde_json;
use anyhow::{Result, Context};


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

    pub fn search_song(&self) -> Result<()> {
        let p = pyo3::Python::acquire_gil();
        let py = p.python();
        let s = self.ytdl
        .getattr(py, "extract_info")?
        .call1(py, ("AjesoBGztF8",))?
        .extract::<Py<PyAny>>(py)?;
        let dumps = self.json
        .getattr(py, "dumps")?;
        let locals = [("s", s), ("dumps", dumps)].into_py_dict(py);
        let s = py.eval("dumps(list(s.keys()), indent=4)", None, Some(locals))?
        .extract::<String>()?;
        println!("{}", &s);
        // let s = serde_json::from_str::<serde_json::Value>(&s);
        // dbg!(s);
        Ok(())
    }
}









fn main0() -> pyo3::PyResult<()> {
    pyo3::Python::with_gil(|py| {
        let ytmusicapi = py.import("ytmusicapi")?;
        let version: String = ytmusicapi.getattr("version")?.extract()?;

        let locals = [("os", py.import("os")?)].into_py_dict(py);
        let code = "os.getenv('USER') or os.getenv('USERNAME') or 'Unknown'";
        let user: String = py.eval(code, None, Some(locals))?.extract()?;

        println!("Hello {}, I'm Python {}", user, version);
        Ok(())
    })
}

// https://pyo3.rs/latest/memory.html

fn main1() -> Result<()> {
    pyo3::prepare_freethreaded_python();
    let p = pyo3::Python::acquire_gil();
    let py = p.python();
    let ytm = py.import("ytmusicapi")?;
    let headers_path = "/home/issac/0Git/musimanager/db/headers_auth.json";
    // let ytmusic = ytm.getattr("YTMusic")?.call1(<pyo3::types::PyTuple as PyTryFrom>::try_from(((headers_path)).to_object(py).as_ref(py)).unwrap())?;
    let ytmusic = ytm.getattr("YTMusic")?.call1((headers_path,))?; // rust tuples with single object need a "," at the end
    let py_json = py.import("json")?;

    // get the Python object using py() or directly use Python object to create a new pool, when pool drops, all objects after the pool also drop
    // make sure everything created after the pool does not have a refrence that lives longer
    let _scope = unsafe{ytmusic.py().new_pool()};
    // let py = scope.python();
    
    let s = ytmusic.call_method1("get_song", ("AjesoBGztF8",))?;
    let s = py_json.call_method1("dumps", (s,))?;
    let mut s = serde_json::from_str::<serde_json::Value>(&s.to_string())?;
    s.as_object_mut().context("NoneError")?.remove("playabilityStatus");
    s.as_object_mut().context("NoneError")?.remove("streamingData");
    s.as_object_mut().context("NoneError")?.remove("microformat");
    dbg!(&s);
    Ok(())
}

fn main2() -> Result<()> {
    pyo3::prepare_freethreaded_python();
    let p = pyo3::Python::acquire_gil();
    let py = p.python();
    let ytm = py.import("ytmusicapi")?;
    let headers_path = "/home/issac/0Git/musimanager/db/headers_auth.json";
    
    // these are refrence counted and can stay alive even after the pool is dropped, but gil must be aquired before they are accessed
    // using pyo3::Pythin::with_gil would be better with this
    let ytmusic = ytm.getattr("YTMusic")?.call1((headers_path,))?.extract::<pyo3::Py<pyo3::PyAny>>()?; // rust tuples with single object need a "," at the end
    let py_json = py.import("json")?.extract::<pyo3::Py<pyo3::PyAny>>()?;

    let s = ytmusic.as_ref(py).call_method1("get_song", ("AjesoBGztF8",))?;
    let s = py_json.as_ref(py).call_method1("dumps", (s,))?;
    let mut s = serde_json::from_str::<serde_json::Value>(&s.to_string())?;
    s.as_object_mut().context("NoneError")?.remove("playabilityStatus");
    s.as_object_mut().context("NoneError")?.remove("streamingData");
    s.as_object_mut().context("NoneError")?.remove("microformat");
    dbg!(&s);
    Ok(())
}


#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
struct Song {
    playability_status: Ps,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Ps {
    status: String,
}

