
use anyhow::Result;
use std::{
    io::Stdout,
    fmt::Debug,
};

pub trait Printer: Send + Sync + Debug {
    fn print(&self, stdout: &mut Stdout) -> Result<()>;
}

impl<T> From<T> for crate::image::Printer
    where T: Printer + 'static
{
    fn from(t: T) -> Self {
        crate::image::Printer::new(Box::new(t) as Box<dyn Printer>)
    }
}
