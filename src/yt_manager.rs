
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

use crate::{
    content_handler::{
        ContentHandlerAction,
    },
    content_manager::ContentProviderID,
    content_providers::{
        ContentProvider,
        ContentProviderType,
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
        Ok(Self {
            ytmusic,
            thread,
            json,
            ytdl,
        })
    }
}

// pyo3 cant do python in multiple rust threads at a time. so gotta make sure only one is active at a time
#[derive(Debug)]
pub enum YTAction {
    GetSong {
        url: String,
    },
    AlbumSearch {
        term: String,
        add_to: ContentProviderID,
    },
    GetAlbum {
        browse_id: String,
        add_to: ContentProviderID,
    },
}
impl YTAction {
    fn run(&mut self, py: Python, pyd: &Py<PyAny>, pyh: &mut PyHandel) -> Result<()> {
        dbg!(&self);
        let globals = [
            ("res", &*pyd),
            ("thread", &pyh.thread),
            ("ytdl", &pyh.ytdl),
            ("ytmusic", &pyh.ytmusic),
            ("json", &pyh.json),
        ].into_py_dict(py);
        match self {
            Self::GetSong {url} => {
                let code = format!("
                    def dbg_res():
                        with open('config/temp/get_song.log', 'r') as f:
                            res['data'] = f.read()
                            res['found'] = True
                    def f():
                        ytdl_data = ytdl.extract_info(url='{url}', download=False)
                        res['data'] = json.dumps(ytdl_data, indent=4)
                        res['found'] = True
                    f = dbg_res
                    handle = thread(target=f, args=())
                    handle.start()
                ");
                let code = fix_python_indentation(code);
                py.run(&code, Some(globals), None)?;
            }
            Self::AlbumSearch {term, ..} => {
                let code = format!("
                    def dbg_res():
                        with open('config/temp/album_search.log', 'r') as f:
                            res['data'] = f.read()
                            res['found'] = True
                    def f():
                        data = ytmusic.search('{term}', filter='albums', limit=75, ignore_spelling=True)
                        res['data'] = json.dumps(data, indent=4)
                        res['found'] = True
                    f = dbg_res
                    handle = thread(target=f, args=())
                    handle.start()
                ");
                let code = fix_python_indentation(code);
                py.run(&code, Some(globals), None)?;
            }
            Self::GetAlbum {browse_id, ..} => {
                let code = format!("
                    def try_catch(f):
                        try: f()
                        except Exception as e:
                            import traceback
                            res['error'] = traceback.format_exc()
                            res['found'] = True
                    def dbg_res():
                        with open('config/temp/get_album.log', 'r') as f:
                            res['data'] = f.read()
                            res['found'] = True
                    def get_data():
                        album_data = ytmusic.get_album('{browse_id}')
                        playlist_id = album_data.get('playlistId', None)
                        if playlist_id is None: playlist_id = album_data['audioPlaylistId']
                        data = ytdl.extract_info(playlist_id, download=False) # // BAD: make another action just for playlist_id, and handle errors
                        
                        res['data'] = json.dumps(data, indent=4)
                        res['found'] = True
                    f = get_data
                    f = dbg_res # // dbg:
                    handle = thread(target=try_catch, args=[f])
                    handle.start()
                ");
                let code = fix_python_indentation(code);
                py.run(&code, Some(globals), None)?;
            }
        }
        Ok(())
    }

    fn resolve(&mut self, py: Python, pyd: &Py<PyAny>, _pyh: &mut PyHandel) -> Result<ContentHandlerAction> {
        dbg!("resolving YTAction", &self);
        let globals = [("res", pyd)].into_py_dict(py);
        let pyd = py.eval("res['data']", Some(globals), None)?.extract::<Py<PyAny>>()?;
        let err = py.eval("str(res['error'])", Some(globals), None).unwrap().extract::<String>().unwrap();
        debug!("{err}");
        let action = match self {
            Self::GetSong {..} => {
                let res = pyd.extract::<String>(py)?;
                debug!("{res}");
                todo!();
            }
            Self::AlbumSearch {add_to, ..} => {
                let res = pyd.extract::<String>(py)?;
                // debug!("{res}");
                let content_providers = serde_json::from_str::<Vec<Album>>(&res);
                // dbg!(&content_providers);
                let content_providers = content_providers?.into_iter().map(Into::into).collect();
                // dbg!(&content_providers);
                vec![
                    ContentHandlerAction::LoadContentManager {
                        songs: Default::default(),
                        content_providers,
                        loader_id: *add_to,
                    },
                    ContentHandlerAction::RefreshDisplayContent,
                ].into()
            }
            Self::GetAlbum {add_to, ..} => {
                let res = pyd.extract::<String>(py)?;
                debug!("{res}");
                ContentHandlerAction::None
            }
        };
        Ok(action)
    }
}

// https://serde.rs/attributes.html
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct Album {
    title: Option<String>,
    browse_id: Option<String>,
    artists: Option<Vec<Option<AlbumArtist>>>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct AlbumArtist {
    name: Option<String>,
    id: Option<String>,
}
impl Into<ContentProvider> for Album {
    fn into(self) -> ContentProvider {
        let loaded = self.browse_id.is_none();
        let t = if self.browse_id.is_some() {
            ContentProviderType::Album {browse_id: self.browse_id.unwrap()}
        } else {
            ContentProviderType::Borked
        };

        ContentProvider::new(
            self.title.unwrap().into(),
            t,
            loaded,
        )
    }
}

pub struct YTActionEntry {
    action: YTAction,
    pyd: Py<PyAny>,
}

#[derive(Debug)]
pub struct YTManager {
    sender: Sender<YTAction>,
    receiver: Receiver<ContentHandlerAction>,
    thread: JoinHandle<Result<()>>,
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
                std::thread::sleep(std::time::Duration::from_secs_f64(0.2));
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
fn fix_python_indentation(code: String) -> String {
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

