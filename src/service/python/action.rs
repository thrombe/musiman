
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
            },
            code::{
                PyCode,
            },
        },
    },
};


// pyo3 cant do python in multiple rust threads at a time. so gotta make sure only one is active at a time
#[derive(Derivative)]
#[derivative(Debug)]
pub enum PyAction {
    ExecCode {
        code: PyCode,
        #[derivative(Debug="ignore")]
        callback: PyCallback,
    },
}
impl PyAction {
    pub fn run(&mut self, py: Python, pyd: &Py<PyAny>, pyh: &mut PyHandle) -> Result<()> {
        dbg!("running ytaction", &self);
        let code = match self {
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
        py.run(&code.code, dict, None)?; // android has problems creating threads from python it seems
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
            Self::ExecCode {callback, ..} => {
                let res = pyd.extract::<String>(py)?;
                callback(res)?
            }
        };
        Ok(action)
    }
}

pub type PyCallback = Box<dyn FnOnce(String) -> Result<ContentManagerAction> + Send + Sync>;
