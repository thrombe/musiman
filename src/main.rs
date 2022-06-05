#![allow(dead_code)]

// #![allow(unused_variables)]
// #![allow(unused_imports)]

mod app;
mod content;
mod image;
mod service;

pub use service::log::{
    // dbg macro is defined at the root of the crate automatically for some reason
    debug,
    error,
};


use anyhow::Result;

fn main() -> Result<()> {
    app::run::run()
}

