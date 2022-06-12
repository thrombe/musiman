
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

pub mod ytmusic;
pub mod ytdl;

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

