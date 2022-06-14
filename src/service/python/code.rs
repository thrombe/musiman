
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use std::borrow::Cow;
use anyhow::{
    Result,
    Context,
};

use crate::service::python::{
    item::{
        Items,
        Thread,
    },
};


#[derive(Default)]
pub struct PyCodeBuilder {
    threaded: bool,
    dbg_stat: bool,
    dbg_code: Option<Cow<'static, str>>,
    dbg_globals: Option<Items>,
    code: Option<Cow<'static, str>>,
    globals: Option<Items>,
}
impl PyCodeBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn dbg_func<T: Into<Cow<'static, str>>>(mut self, code: T, globals: Option<Items>) -> Self {
        self.dbg_code = Some(code.into());
        self.dbg_globals = globals;
        self
    }
    pub fn set_dbg_status(mut self, stat: bool) -> Self {
        self.dbg_stat = stat;
        self
    }
    pub fn get_data_func<T: Into<Cow<'static, str>>>(mut self, code: T, globals: Option<Items>) -> Self {
        self.code = Some(code.into());
        self.globals = globals;
        self
    }
    pub fn threaded(mut self) -> Self {
        self.threaded = true;
        self
    }
    pub fn build(self) -> Result<PyCode> {
        let mut code = self.code.context("code not provided")?;
        let mut globals = self.globals;
        if self.dbg_stat {
            code = self.dbg_code.context("dbg code not provided")?;
            globals = self.dbg_globals;
        }

        code = fix_code_indentation(code.as_ref()).into();
        code = code.lines().map(|line| "    ".to_owned() + line + "\n").collect();
        code = append_code(
            &fix_code_indentation("
                def try_catch(f):
                    try:
                        res['data'] = f()
                    except Exception as e:
                        import traceback
                        res['error'] = traceback.format_exc()
                    res['found'] = True
                def function():
            "),
            &code,
        ).into();

        if self.threaded {
            globals = globals.map(|mut g| {
                g.push(Thread::new("thread").into());
                g
            });
            code = append_code(
                &code,
                &fix_code_indentation("
                    handle = thread(target=try_catch, args=[function])
                    handle.start()    
                ")
            ).into();
        } else {
            code = append_code(
                &code,
                &fix_code_indentation("
                    try_catch(function)
                ")
            ).into();
        }

        let pycode = PyCode {
            code,
            globals,
        };
        Ok(pycode)
    }
}


#[derive(Debug)]
pub struct PyCode {
    pub code: Cow<'static, str>,
    pub globals: Option<Items>,
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
    .filter(|line| !line.trim().is_empty())
    .map(|line| 
        line
        .chars()
        .skip(whitespace_chars)
        .collect::<String>()
    )
    .map(|line| String::from(line) + "\n")
    .collect()
}

pub fn append_code(a: &str, b: &str) -> String {
    a.lines().chain(b.lines()).collect::<Vec<_>>().join("\n")
}
