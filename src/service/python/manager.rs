
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use pyo3::{
    types::{
        IntoPyDict,
        PyAny,
    },
    Py,
};
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

use crate::{
    content::manager::{
        action::ContentManagerAction,
    },
    service::python::{
        action::PyAction,
        item::{
            PyHandle,
            Time,
        },
    },
};


pub struct PyActionEntry {
    action: PyAction,
    pyd: Py<PyAny>,
}

#[derive(Debug)]
pub struct PyManager {
    sender: Sender<PyAction>,
    receiver: Receiver<ContentManagerAction>,
    thread: JoinHandle<Result<()>>, // FIX: communicate the crash in this thread to the user
}

impl PyManager {
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

    pub fn poll(&mut self) -> ContentManagerAction {
        if self.thread.is_finished() {
            let (a_sender, a_receiver) = mpsc::channel();
            let (yt_sender, yt_receiver) = mpsc::channel();
            self.receiver = a_receiver;
            self.sender = yt_sender;
            let thread = std::mem::replace(&mut self.thread, Self::init_thread(a_sender, yt_receiver));
            let res = thread.join().unwrap();
            match res {
                Ok(_) => (),
                Err(err) => {
                    error!("{err}");
                }
            }
        }
        match self.receiver.try_recv().ok() {
            Some(a) => {
                dbg!("action received");
                a
            },
            None => ContentManagerAction::None
        }
    }

    pub fn run(&mut self, action: PyAction) -> Result<()> {
        dbg!(&action);
        self.sender.send(action).ok().context("send error")
    }

    fn init_thread(sender: Sender<ContentManagerAction>, receiver: Receiver<PyAction>) -> JoinHandle<Result<()>> {
        let thread = thread::spawn(move || -> Result<()> {
            pyo3::prepare_freethreaded_python();
            let p = pyo3::Python::acquire_gil(); 
            let py = p.python();

            let pyh = &mut PyHandle::new()?;
            let mut actions = vec![];
            let time = [Time::new("time").into()];

            loop {
                // sleeping in python seems to not ruin speed. sleeping in rust somehow destroys it
                py.run("time.sleep(0.2)", Some(pyh.get_dict(py, &time)?), None)?;
                match receiver.try_recv() {
                    Ok(a) => {
                        // choosing the default value of a dict so that the new data can be inserted into this dict, and
                        // the memory location does not change. res = data changes the memory location something something
                        // but res['data'] = data does what i want
                        let pyd = py.eval("{'data': None, 'found': False, 'error': None}", None, None)?.extract()?;
                        let entry = PyActionEntry {action: a, pyd };
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
                                    let a = actions.swap_remove(i);
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
