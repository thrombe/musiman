

mod printer;
mod config;
mod utils;

use self::{
    config::Config,
    printer::{
        PrinterType,
    },
};

enum UnprocessedImage {
    Path(String),
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

enum ProcessedImage {
    None,
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

pub struct ImageHandler {
    config: Config,
    processed_image: ProcessedImage,
    printer: PrinterType,
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
            printer: PrinterType::Block,
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

    pub fn maybe_print(&mut self) {
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

        use self::printer::Printer;
        let (_w, _h) = self
        .printer
        .print_from_file(&mut stdout, "/home/issac/Pictures/Screenshot_20211122_221759.png", &self.config)
        .unwrap();

        if self.config.restore_cursor {
            execute!(&mut stdout, RestorePosition).unwrap();
        };

    }
}
