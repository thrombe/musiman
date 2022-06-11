
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

mod printer;
mod config;
mod utils;

use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use reqwest;

use self::{
    config::Config,
    printer::{
        Printer,
        Block,
        Sixel,
    },
};

use derivative::Derivative;
use std::{
    path::PathBuf,
    io::Stdout,
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

pub enum ProcessedImage {
    None,
    Block {
        img: Block,
        width: u32,
        height: u32,
    },
    Sixel {
        img: Sixel,
        width: u32,
        height: u32,
    },
}
impl Default for ProcessedImage {
    fn default() -> Self {
        Self::None
    }
}
impl From<Option<ProcessedImage>> for ProcessedImage {
    fn from(o: Option<ProcessedImage>) -> Self {
        match o {
            Some(e) => e,
            None => Self::None,
        }
    }
}

impl ProcessedImage {
    pub fn needs_processing(&self, config: &Config) -> bool {
        let (width, height) = match self {
            Self::Block {width, height, ..} => {
                (width, height)
            }
            Self::Sixel { width, height , ..} => {
                (width, height)
            }
            Self::None => return true,
        };
        !(*width == config.width.unwrap() && *height == config.height.unwrap())
    }

    pub fn process(&mut self, image: &DynamicImage, config: &Config, printer: &Printer) {
        match printer {
            Printer::Block => {
                let block = Block::new(image, config).unwrap();
                *self = Self::Block {
                    img: block,
                    width: config.width.unwrap(),
                    height: config.height.unwrap(),
                };
            }
            Printer::Sixel => {
                let out = Sixel::new(image, config).unwrap();
                *self = Self::Sixel {
                    img: out,
                    width: config.width.unwrap(),
                    height: config.height.unwrap(),
                }
            }
        }
    }

    pub fn print(&mut self, stdout: &mut Stdout) -> Result<()> {
        match self {
            Self::Block {img, ..} => {
                img.print(stdout)?;
            }
            Self::Sixel {img, ..} => {
                img.print(stdout)?;
            }
            Self::None => (),
        }
        Ok(())
    }
}

pub struct ImageHandler {
    config: Config,
    processed_image: ProcessedImage,
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
                truecolor: utils::truecolor_available(),
                use_sixel: true,
                alignment: Default::default(),
            },
            printer: Printer::Block,
            processed_image: Default::default(),
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
        if width != self.config.width || height != self.config.height {
            self.dimensions_changed = true;
            self.config.width = width;
            self.config.height = height;
        }
    }

    pub fn set_image<T>(&mut self, img: T)
        where
            T: Into<UnprocessedImage>
    {
        self.unprocessed_image = img.into();
        self.processed_image = None.into();
    }

    fn prepare_image(&mut self) {
        self.unprocessed_image.prepare_image().unwrap();
        if self.processed_image.needs_processing(&self.config) {
            match self.unprocessed_image.get_image() {
                Some(img) => {
                    self.processed_image.process(img, &self.config, &self.printer);
                }
                None => (),
            }
        }
    }

    pub fn maybe_print(&mut self) -> Result<()> {
        // dbg!("maybe printing");
        // if self.dimensions_changed { // TODO:

        // } else {

        // }

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

        self.prepare_image();
        self.processed_image.print(&mut stdout)?;

        if self.config.restore_cursor {
            execute!(&mut stdout, RestorePosition)?;
        };
        Ok(())
    }
}
