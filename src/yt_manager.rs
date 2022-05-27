
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

use pyo3::{
    types::{IntoPyDict, PyAny},
    Py,
};
use serde;
use serde_json;
use anyhow::{Result, Context};

#[derive(Clone)]
pub struct YTManager {
    ytmusic: Py<PyAny>,
    ytdl: Py<PyAny>,
    json: Py<PyAny>,
    thread: Py<PyAny>,
}

impl YTManager {
    pub fn new(py: pyo3::Python) -> Result<Self> {
        // pyo3::prepare_freethreaded_python();
        // let p = pyo3::Python::acquire_gil(); 
        // let py = p.python();
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

        Ok(Self {
            ytmusic,
            ytdl,
            json,
            thread,
        })
    }

    // pub fn search_song(&self, py: pyo3::Python, key: &str) -> Result<()> {
    //     // let p = pyo3::Python::acquire_gil();
    //     // let py = p.python();
    //     let s = self.ytdl
    //     .getattr(py, "extract_info")?
    //     .call1(py, (key,))?
    //     .extract::<Py<PyAny>>(py)?;
    //     let dumps = self.json
    //     .getattr(py, "dumps")?;
    //     let locals = [("s", s), ("dumps", dumps)].into_py_dict(py);
    //     let s = py.eval("dumps(list(s.keys()), indent=4)", None, Some(locals))?
    //     .extract::<String>()?;
    //     // println!("{}", &s);
    //     // let s = serde_json::from_str::<serde_json::Value>(&s);
    //     debug!("{s}");
    //     Ok(())
    // }

    pub fn search_song(&self, py: pyo3::Python, key: &str) -> Result<Py<PyAny>> {
        // let p = pyo3::Python::acquire_gil();
        // let py = p.python();
        let f = py.None();
        let extract_info = self.ytdl
        .getattr(py, "extract_info")?
        // .call1(py, (key,))?
        .extract::<Py<PyAny>>(py)?;
        let dumps = self.json
        .getattr(py, "dumps")?;
        let globals = [
            (stringify!(extract_info), extract_info),
            (stringify!(dumps), dumps),
            (stringify!(f), f.clone()),
            (stringify!(key), pyo3::types::PyString::new(py, key).into()),
            ].into_py_dict(py);
        // let s = py.eval("dumps(list(s.keys()), indent=4)", Some(globals), None)?
        // .extract::<String>()?;
        // println!("{}", &s);
        // let s = serde_json::from_str::<serde_json::Value>(&s);
        let code = "
def func():
    return dumps(extract_info(key), indent=4)
f = func
        ";
        py.run(code, Some(globals), None)?;
        Ok(f)
    }

    // pub fn search_song(&self) -> Result<()> {
    //     pyo3::Python::with_gil(|py| -> Result<()> {
    //         let s = self.ytdl
    //         .getattr(py, "extract_info")?
    //         .call1(py, ("AjesoBGztF8",))?
    //         .extract::<Py<PyAny>>(py)?;
    //         let dumps = self.json
    //         .getattr(py, "dumps")?;
    //         let locals = [("s", s), ("dumps", dumps)].into_py_dict(py);
    //         let s = py.eval("dumps(list(s.keys()), indent=4)", None, Some(locals))?
    //         .extract::<String>()?;
    //         // println!("{}", &s);
    //         // let s = serde_json::from_str::<serde_json::Value>(&s);
    //         debug!("{s}");
    //         Ok(())
    //     })
    // }
}

enum Faction {
    GetSong {
        key: String,
    }
}

fn testing_python_threading_action() -> Result<()> {
    let (sender, receiver) = std::sync::mpsc::channel();

    std::thread::spawn(move || -> Result<()> {
        pyo3::prepare_freethreaded_python();
        let p = pyo3::Python::acquire_gil(); 
        let py = p.python();
        let ytman = YTManager::new(py)?;
        let mut actions = vec![];
        loop {
            std::thread::sleep(std::time::Duration::from_secs_f64(0.1));
            match receiver.try_recv() {
                Ok(a) => {
                    actions.push(a);
                    let a = actions.last().unwrap();
                    match a {
                        Faction::GetSong { key } => { // "AjesoBGztF8"
                            let f = ytman.search_song(py, key)?;
                        }
                    }
                }
                Err(_) => {
                    for a in &actions {
                        match a {
                            Faction::GetSong { key } => {

                            }
                        }
                    }
                }
            }
        }
    });


    Ok(())
}

fn wierd_threading_test() -> Result<()> {
    pyo3::prepare_freethreaded_python();
    let p = pyo3::Python::acquire_gil(); 
    let py = p.python();
    let thread = py.import("threading")?
    .getattr("Thread")?
    .extract()?;
    let enu = py.None();
    let globals = [("thread", thread), ("enu", enu)].into_py_dict(py);
    let code = "
enu = None
def f():
    global enu
    print('ininnu')
    import time
    time.sleep(2)
    enu = 42
handle = thread(target=f, args=())
handle.start()
thread = handle
print('enu', enu)
    ";
    py.run(code, Some(globals), None)?;
    let code = "
thread.join()
print('from other run', enu)
    ";
    py.run(code, Some(globals), None)?;
    




    Ok(())
}

pub fn test() -> Result<()> {
    return wierd_threading_test();

    // use std::thread;
    // let t_handle1 = thread::spawn(move || -> Result<()> {
    //     let ytm = YTManager::new()?;
    //     dbg!("t1");
    //     ytm.search_song().unwrap();
    //     Ok(())
    // });
    // let t_handle2 = thread::spawn(move || -> Result<()> {
    //     let ytm = YTManager::new()?;
    //     dbg!("t2");
    //     ytm.search_song().unwrap();
    //     Ok(())
    // });
    // dbg!("main");
    // t_handle1.join().unwrap().unwrap();
    // t_handle2.join().unwrap().unwrap();

    // use tokio::runtime::Runtime;
    // let rt = Runtime::new().unwrap();
    // let handle = rt.handle();
    // let t_handle = handle.spawn_blocking(|| {
    //     println!("now running on a worker thread");
    // });

    // use tokio::task;
    // let j_handle = task::spawn_blocking(|| -> Result<()>{
    //     println!("now running on a worker thread");
    //     ytm.search_song()
    // });
    
    return Ok(());
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

