
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

mod printer;
mod config;
mod utils;

use anyhow::Result;
use image::DynamicImage;
use reqwest;

use self::{
    config::Config,
    printer::{
        Printer, BlockPrinter, SixelPrinter, SixelOutput,
    },
};

pub enum UnprocessedImage {
    Path(String),
    Url(String),
    Image(image::DynamicImage),
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
        Self::Image(o)
    }
}
impl UnprocessedImage {
    pub fn needs_preparing(&self) -> bool {
        match self {
            Self::Image(..) => false,
            Self::None | Self::Url(..) | Self::Path(..) => true,
        }
    }

    pub fn prepare_image(&mut self) -> Result<()> {
        match &self {
            Self::Path(path) => {
                let img = image::io::Reader::open(path)?.with_guessed_format()?.decode()?;
                *self = Self::Image(img);
            }
            Self::Url(url) => {
                let res = reqwest::blocking::get(url)?;
                let img = image::load_from_memory(&res.bytes()?)?;
                *self = Self::Image(img);
            }
            Self::Image(..) => (),
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
            Self::Image(img) => Some(img),
            _ => None,
        }
    }
}

pub enum ProcessedImage {
    None,
    Block {
        img: termcolor::Buffer,
        width: u32,
        height: u32,
    },
    SixelEncoder {
        img: Vec<u8>,
        img_width: u32,
        img_height: u32,
        width: u32,
        height: u32,
    },
    SixelOutput {
        img: SixelOutput,
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
            Self::SixelEncoder {width, height, ..} => {
                (width, height)
            }
            Self::SixelOutput { width, height , ..} => {
                (width, height)
            }
            Self::None => return true,
        };
        !(*width == config.width.unwrap() && *height == config.height.unwrap())
    }

    pub fn process(&mut self, image: &DynamicImage, config: &Config, printer: &Printer) {
        match printer {
            Printer::Block => {
                let mut buf = termcolor::Buffer::ansi();
                BlockPrinter.print(&mut buf, image, config).unwrap();
                *self = Self::Block {
                    img: buf,
                    width: config.width.unwrap(),
                    height: config.height.unwrap(),
                };
            }
            Printer::SixelEncoder => {
                let (width, height, img) = SixelPrinter.get_quickframe(image, &Config {x: 0, y: 0, ..*config});
                *self = Self::SixelEncoder {
                    img,
                    img_width: width,
                    img_height: height,
                    width: config.width.unwrap(),
                    height: config.height.unwrap(),
                }
            }
            Printer::SixelOutput => {
                let out = SixelOutput::new(image, config).unwrap();
                *self = Self::SixelOutput {
                    img: out,
                    width: config.width.unwrap(),
                    height: config.height.unwrap(),
                }
            }
        }
    }

    pub fn print(&mut self, config: &Config) {
        match self {
            Self::Block {img, ..} => {
                use std::io::Write;
                std::io::BufWriter::new(std::io::stdout()).write_all(img.as_slice()).unwrap();
            }
            Self::SixelEncoder {img, img_width, img_height, ..} => {
                SixelPrinter.print_quickframe(img, *img_width, *img_height, config).unwrap();
            }
            Self::SixelOutput {img, ..} => {
                img.print(config).unwrap();
            }
            Self::None => (),
        }
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
                transparent: false,
                absolute_offset: true,
                x: 0,
                y: 0,
                restore_cursor: true,
                width: None,
                height: None,
                truecolor: utils::truecolor_available(),
                use_sixel: true,
                alignment: Default::default(),
            },
            printer: Printer::SixelEncoder,
            processed_image: Default::default(),
            unprocessed_image: Default::default(),
            dimensions_changed: false,
        }
    }
}

impl ImageHandler {
    pub fn set_offset(&mut self, x: u16, y: i16) {
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

    pub fn maybe_print(&mut self) {
        dbg!("maybe printing");
        if self.dimensions_changed { // TODO:

        } else {

        }

        use crossterm::{
            execute,
            cursor::{
                SavePosition,
                RestorePosition,
            },
        };
        let mut stdout = std::io::stdout();
        if self.config.restore_cursor {
            execute!(&mut stdout, SavePosition).unwrap();
        }

        self.prepare_image();
        self.processed_image.print(&self.config);

        if self.config.restore_cursor {
            execute!(&mut stdout, RestorePosition).unwrap();
        };

    }
}
