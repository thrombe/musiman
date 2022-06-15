
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
use derivative::Derivative;
use anyhow::{
    Result,
    Context,
};

use crate::{
    content::{
        manager::{
            action::{
                ContentManagerAction,
            },
        },
    },
    service::{
        python::{
            item::{
                PyHandle,
                Ytdl,
                Json,
            },
            code::{
                PyCode,
                PyCodeBuilder,
            },
        },
        yt::ytdl::YtdlSong,
    },
};


// pyo3 cant do python in multiple rust threads at a time. so gotta make sure only one is active at a time
#[derive(Derivative)]
#[derivative(Debug)]
pub enum PyAction {
    GetSong {
        url: String,
        #[derivative(Debug="ignore")]
        callback: Box<dyn Fn(String, String) -> ContentManagerAction + Send + Sync>,
    },
    ExecCode {
        code: PyCode,
        #[derivative(Debug="ignore")]
        callback: PyCallback,
    },
}
impl PyAction {
    pub fn run(&mut self, py: Python, pyd: &Py<PyAny>, pyh: &mut PyHandle) -> Result<()> {
        dbg!("running ytaction", &self);
        let mut bad_code;
        let code = match self {
            Self::GetSong {url, ..} => {
                bad_code = PyCodeBuilder::new()
                .threaded()
                .set_dbg_status(false)
                .dbg_func(
                    "
                        with open('config/temp/ytdl_song.log', 'r') as f:
                            data = f.read()
                        return data                    
                    ",
                    None,
                )
                .get_data_func(
                    format!("
                        ytdl_data = ytdl.extract_info(url='{url}', download=False)
                        data = json.dumps(ytdl_data, indent=4)
                        return data
                    "),
                    Some(vec![
                        Ytdl::new("ytdl").into(),
                        Json::new("json").into(),
                    ]),
                )
                .build()?;
                &mut bad_code
            }
            Self::ExecCode {code, ..} => {
                code
            }
        };
        debug!("{}", code.code);
        let dict = match &code.globals {
            Some(g) => Some(pyh.get_dict(py, g)?),
            None => None,
        };
        let dict = dict.map(|dict| {
            dict.set_item("res", pyd).unwrap();
            dict
        });
        py.run(&code.code, dict, None)?;
        Ok(())
    }

    pub fn resolve(self, py: Python, pyd: &Py<PyAny>, _pyh: &mut PyHandle) -> Result<ContentManagerAction> {
        dbg!("resolving YTAction", &self);
        let globals = [("res", pyd)].into_py_dict(py);
        let pyd = py.eval("res['data']", Some(globals), None)?.extract::<Py<PyAny>>()?;
        if py.eval("res['error'] != None", Some(globals), None)?.extract::<bool>()? {
            let err = py.eval("res['error']", Some(globals), None)?.extract::<String>()?;
            error!("{err}");
            return Ok(ContentManagerAction::None); // ?
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
            Self::ExecCode {callback, ..} => {
                let res = pyd.extract::<String>(py)?;
                callback(res)?
            }
        };
        Ok(action)
    }
}

pub type PyCallback = Box<dyn FnOnce(String) -> Result<ContentManagerAction> + Send + Sync>;
