
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};


use anyhow::Result;
use ansi_colours::ansi256_from_rgb;
use image::{
    DynamicImage,
    GenericImageView,
    Rgba,
    imageops::FilterType,
};
use termcolor::{
    Color,
    ColorSpec,
    WriteColor,
    Buffer,
};
use crossterm::{
    cursor::MoveRight,
    execute,
};
use std::io::{
    Write,
    BufWriter,
    Stdout,
};
use crate::image::{
    printer::adjust_offset,
    config::Config,
};

const UPPER_HALF_BLOCK: &str = "\u{2580}";
const LOWER_HALF_BLOCK: &str = "\u{2584}";

const CHECKERBOARD_BACKGROUND_LIGHT: (u8, u8, u8) = (153, 153, 153);
const CHECKERBOARD_BACKGROUND_DARK: (u8, u8, u8) = (102, 102, 102);

pub struct Block {
    img: Buffer,
}

impl Block {
    pub fn new(img: &DynamicImage, config: &Config) -> Result<Self> {
        let mut buff = Buffer::ansi();
        // adjust with x=0 and handle horizontal offset entirely below
        adjust_offset(&mut buff, 0, config.y)?;
        
        // resize the image so that it fits in the constraints, if any
        let (w, h) = get_size_block(img, config.width, config.height);

        let img = img.resize_exact(w, h, FilterType::Triangle);
        let (width, height) = img.dimensions();
        
        let mut row_color_buffer = vec![ColorSpec::new(); width as usize];
        let img_buffer = img.to_rgba8();
        
        for (curr_row, img_row) in img_buffer.enumerate_rows() {
            let is_even_row = curr_row % 2 == 0;
            let is_last_row = curr_row == height - 1;
            
            // move right if x offset is specified
            if config.x > 0 && (!is_even_row || is_last_row) {
                execute!(buff, MoveRight(config.x))?;
            }
            
            for pixel in img_row {
                // choose the half block's color
                let color = if is_pixel_transparent(pixel) {
                    None
                } else {
                    Some(get_color_from_pixel(pixel, config.truecolor))
                };
                
                // Even rows modify the background, odd rows the foreground
                // because lower half blocks are used by default
                let colorspec = &mut row_color_buffer[pixel.0 as usize];
                if is_even_row {
                    colorspec.set_bg(color);
                    if is_last_row {
                        write_character(&mut buff, colorspec, true)?;
                    }
                } else {
                    colorspec.set_fg(color);
                    write_character(&mut buff, colorspec, false)?;
                }
            }
    
            if !is_even_row && !is_last_row {
                buff.reset()?;
                writeln!(&mut buff as &mut dyn WriteColor, "\r")?;
            }    
        }
        buff.reset()?;
        writeln!(&mut buff as &mut dyn WriteColor)?;
        (&mut buff as &mut dyn WriteColor).flush()?;
        
        Ok(Self {
            img: buff,
        })
    }    
    
    pub fn print(&self, stdout: &mut Stdout) -> Result<()> {
        BufWriter::new(stdout).write_all(self.img.as_slice())?;
        Ok(())
    }
}

fn write_character(stdout: &mut impl WriteColor, c: &ColorSpec, is_last_row: bool) -> Result<()> {
    let out_color;
    let out_char;
    let mut new_color;

    // On the last row use upper blocks and leave the bottom half empty (transparent)
    if is_last_row {
        new_color = ColorSpec::new();
        if let Some(bg) = c.bg() {
            new_color.set_fg(Some(*bg));
            out_char = UPPER_HALF_BLOCK;
        } else {
            execute!(stdout, MoveRight(1))?;
            return Ok(());
        }
        out_color = &new_color;
    } else {
        match (c.fg(), c.bg()) {
            (None, None) => {
                // completely transparent
                execute!(stdout, MoveRight(1))?;
                return Ok(());
            }
            (Some(bottom), None) => {
                // only top transparent
                new_color = ColorSpec::new();
                new_color.set_fg(Some(*bottom));
                out_color = &new_color;
                out_char = LOWER_HALF_BLOCK;
            }
            (None, Some(top)) => {
                // only bottom transparent
                new_color = ColorSpec::new();
                new_color.set_fg(Some(*top));
                out_color = &new_color;
                out_char = UPPER_HALF_BLOCK;
            }
            (Some(_top), Some(_bottom)) => {
                // both parts have a color
                out_color = c;
                out_char = LOWER_HALF_BLOCK;
            }
        }
    }
    stdout.set_color(out_color)?;
    write!(stdout, "{}", out_char)?;

    Ok(())
}

fn is_pixel_transparent(pixel: (u32, u32, &Rgba<u8>)) -> bool {
    pixel.2[3] == 0
}

fn get_color_from_pixel(pixel: (u32, u32, &Rgba<u8>), truecolor: bool) -> Color {
    let (_x, _y, data) = pixel;
    let rgb = (data[0], data[1], data[2]);
    if truecolor {
        Color::Rgb(rgb.0, rgb.1, rgb.2)
    } else {
        Color::Ansi256(ansi256_from_rgb(rgb))
    }
}

fn get_size_block(img: &DynamicImage, width: Option<u32>, height: Option<u32>) -> (u32, u32) {
    let (img_width_pix, img_height_pix) = img.dimensions();

    let (scr_width_chars, scr_height_chars) = {
        let rc = termion::terminal_size().unwrap();
        (rc.0 as u32, rc.1 as u32)
    };

    let (char_width, char_height) = {
        let (scr_width, scr_height) = termion::terminal_size_pixels().unwrap();

        // terminal size in pixels can be a little bigger than the space where chars are printed.
        // so floor is needed
        let (mut char_width, mut char_height) = (
            (scr_width as f32/scr_width_chars as f32) as u32,
            (scr_height as f32/scr_height_chars as f32) as u32
        );
        if scr_width == 0 && scr_height == 0 {
            char_width = 12;
            char_height = 24;
        }
        (char_width, char_height)
    };

    let (_scr_width_pix, _scr_height_pix) = {
        (
            scr_width_chars*char_width,
            scr_height_chars*char_height
        )
    };

    let (bound_width_chars, bound_height_chars) = {
        (
            width.unwrap_or(scr_width_chars),
            height.unwrap_or(scr_height_chars)
        )
    };

    let (bound_width_pix, bound_height_pix) = {
        (
            bound_width_chars*char_width,
            bound_height_chars*char_height
        )
    };

    let img_ratio = img_height_pix as f32/img_width_pix as f32;
    let bound_ratio = bound_height_pix as f32/bound_width_pix as f32;

    let (new_width_pix, new_height_pix);

    if img_ratio > bound_ratio { // if image is skinnier than bound box
        new_height_pix = bound_height_pix;
        new_width_pix = img_width_pix*bound_height_pix/img_height_pix;
    } else {
        new_width_pix = bound_width_pix;
        new_height_pix = img_height_pix*bound_width_pix/img_width_pix;
    }
    
    
    (
        new_width_pix/char_width,
        2*new_height_pix/char_height,
    )
}

