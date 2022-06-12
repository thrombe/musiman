
/// Configuration struct to customize printing behaviour.
#[derive(Debug)]
pub struct Config {
    /// X offset
    pub x: u16,
    /// Y offset
    pub y: u16,
    /// Take a note of cursor position before printing and restore it when finished.
    /// Defaults to false.
    pub restore_cursor: bool,
    /// Optional image width. Defaults to None.
    pub width: Option<u32>,
    /// Optional image height. Defaults to None.
    pub height: Option<u32>,
    pub printer_chooser: crate::image::printer::PrinterChooser,
    pub alignment: ImageAlignment,
}

#[derive(Debug, Clone, Copy)]
pub struct ImageAlignment { // TODO: actually impliment this
    horizontal: HorizontalAlignment,
    vertical: VerticalAlignment,
}

impl Default for ImageAlignment {
    fn default() -> Self {
        Self {
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Center,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HorizontalAlignment {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Copy)]
pub enum VerticalAlignment {
    Top,
    Bottom,
    Center,
}

impl std::default::Default for Config {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            restore_cursor: false,
            width: None,
            height: None,
            printer_chooser: Default::default(),
            alignment: Default::default(),
        }
    }
}
