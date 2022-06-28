
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

use crossterm::{
    cursor::{
        MoveTo,
    },
    execute,
};
use std::io::Write;
use anyhow::Result;
use image::DynamicImage;

use crate::{
    image::config::Config,
};

mod traits;
mod block;
#[cfg(feature = "sixel")]
pub mod sixel;


#[derive(Debug, Clone, Copy)]
pub enum PrinterChooser {
    Block,
    Ansi256,
    #[cfg(feature = "sixel")]
    Sixel,
    Default,
    None,
}
impl Default for PrinterChooser {
    fn default() -> Self {
        Self::None
    }
}
impl PrinterChooser {
    pub fn printer(&self, img: &DynamicImage, config: &Config) -> Result<Printer> {
        let printer = match self {
            PrinterChooser::Block => block::Block::new(img, config, true)?.into(),
            PrinterChooser::Ansi256 => block::Block::new(img, config, false)?.into(),
            #[cfg(feature = "sixel")]
            PrinterChooser::Sixel => sixel::Sixel::new(img, config)?.into(),
            PrinterChooser::Default => {
                let mut printer: Option<Printer> = None;
                #[cfg(feature = "sixel")]
                {
                    if sixel::is_sixel_supported() {
                        printer = Some(sixel::Sixel::new(img, config)?.into())
                    }
                }
                {
                    let truecolor = block::truecolor_available();
                    if printer.is_none() {
                        printer = Some(block::Block::new(img, config, truecolor)?.into());
                    }
                }
                printer.unwrap_or(NonePrinter.into())
            }
            PrinterChooser::None => NonePrinter.into(),
        };
        Ok(printer)
    }
}

#[derive(Debug)]
struct NonePrinter;
impl traits::Printer for NonePrinter {
    fn print(&self, _: &mut std::io::Stdout) -> Result<()> {
        Ok(())
    }
}



#[derive(Debug)]
pub struct Printer(Box<dyn traits::Printer>);
impl Default for Printer {
    fn default() -> Self {
        NonePrinter.into()
    }
}
impl Printer {
    pub fn new(p: Box<dyn traits::Printer>) -> Self {
        Self(p)
    }
    pub fn new_from_img(&mut self, img: &image::DynamicImage, config: &super::Config) -> Result<()> {
        *self = config.printer_chooser.printer(img, config)?;
        Ok(())
    }
}
impl std::ops::Deref for Printer {
    type Target = Box<dyn traits::Printer>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Printer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}


// Move the cursor to a location from where it should start printing
fn adjust_offset(stdout: &mut impl Write, x_off: u16, y_off: u16) -> Result<()> {
    // If absolute_offset, move to (x,y).
    execute!(stdout, MoveTo(x_off, y_off))?;
    Ok(())
}

