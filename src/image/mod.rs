
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

mod printer;
mod config;

use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use reqwest;

use crate::image::{
    config::Config,
    printer::{
        Printer,
        PrinterChooser,
    },
};

use derivative::Derivative;
use std::{
    path::PathBuf,
};

#[derive(Derivative)]
#[derivative(Debug)]
pub enum UnprocessedImage {
    Path(PathBuf),
    Url(String), // implimenting into From<String> might be dangerous? accidental string path to Url
    Image {
        #[derivative(Debug="ignore")]
        img: image::DynamicImage,
    },
    None,
}
impl Default for UnprocessedImage {
    fn default() -> Self {
        Self::None
    }
}
impl From<Option<UnprocessedImage>> for UnprocessedImage {
    fn from(o: Option<UnprocessedImage>) -> Self {
        match o {
            Some(e) => e,
            None => Self::None,
        }
    }
}
impl From<DynamicImage> for UnprocessedImage {
    fn from(o: DynamicImage) -> Self {
        Self::Image {img: o}
    }
}
impl From<PathBuf> for UnprocessedImage {
    fn from(o: PathBuf) -> Self {
        Self::Path(o)
    }
}

impl UnprocessedImage {
    pub fn needs_preparing(&self) -> bool {
        match self {
            Self::Image {..} => false,
            Self::None | Self::Url(..) | Self::Path(..) => true,
        }
    }

    pub fn prepare_image(&mut self) -> Result<()> {
        match self {
            Self::Path(path) => {
                let img = image::io::Reader::open(path)?.with_guessed_format()?.decode()?;
                *self = Self::Image {img};
                self.prepare_image()?;
            }
            Self::Url(url) => {
                let res = reqwest::blocking::get(&*url)?;
                let img = image::load_from_memory(&res.bytes()?)?;
                *self = Self::Image {img};
                self.prepare_image()?;
            }
            Self::Image {img} => {
                let (x, y) = img.dimensions();
                if x > y {
                    *img = img.crop((x-y)/2, 0, y, y);
                }
            }
            Self::None => (),
        }
        Ok(())
    }

    fn is_none(&self) -> bool {
        match self {
            Self::None => true,
            _ => false,
        }
    }

    fn get_image(&self) -> Option<&DynamicImage> {
        match self {
            Self::Image {img} => Some(img),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ImageHandler {
    config: Config,
    printer: Printer,
    unprocessed_image: UnprocessedImage,
    dimensions_changed: bool,
}

impl Default for ImageHandler {
    fn default() -> Self {
        Self {
            config: Config {
                x: 0,
                y: 0,
                restore_cursor: true,
                width: None,
                height: None,
                printer_chooser: PrinterChooser::Default,
                alignment: Default::default(),
            },
            printer: Default::default(),
            unprocessed_image: Default::default(),
            dimensions_changed: false,
        }
    }
}

impl ImageHandler {
    pub fn set_offset(&mut self, x: u16, y: u16) {
        self.config.x = x;
        self.config.y = y;
    }

    pub fn set_size(&mut self, width: Option<u32>, height: Option<u32>) {
        self.config.width = width;
        self.config.height = height;
    }

    pub fn set_image<T>(&mut self, img: T)
        where
            T: Into<UnprocessedImage>
    {
        self.unprocessed_image = img.into();
        self.dimensions_changed();
    }

    pub fn clear_image(&mut self) {
        self.printer = Default::default();
    }

    fn prepare_image(&mut self) -> bool {
        if self.dimensions_changed {
            match self.unprocessed_image.get_image() {
                Some(img) => {
                    self.printer.new_from_img(img, &self.config).unwrap();
                    self.dimensions_changed = false;
                    return true;
                }
                None => (),
            }
        }
        false
    }

    pub fn dimensions_changed(&mut self) {
        self.dimensions_changed = true;
    }

    pub fn maybe_print(&mut self) -> Result<()> {
        use crossterm::{
            execute,
            cursor::{
                SavePosition,
                RestorePosition,
            },
        };
        let mut stdout = std::io::stdout();
        if self.config.restore_cursor {
            execute!(&mut stdout, SavePosition)?;
        }

        if self.prepare_image() {
            dbg!("image printed");
            self.printer.print(&mut stdout)?;
        }

        if self.config.restore_cursor {
            execute!(&mut stdout, RestorePosition)?;
        };
        Ok(())
    }
}
