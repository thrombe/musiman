
use console::{
    Key,
    Term,
};
use image::{
    imageops::FilterType,
    DynamicImage,
    GenericImageView,
};
use lazy_static::lazy_static;
use sixel::{
    encoder::{
        Encoder,
        QuickFrameBuilder,
    },
    optflags::EncodePolicy,
};
use std::io::Write;
use termion;
use anyhow::Result;

use crate::image::{
    printer::{
        adjust_offset,
        Printer,
    },
    config::Config,
};

pub struct SixelPrinter;

lazy_static! {
    static ref SIXEL_SUPPORT: bool = check_sixel_support();
}

/// Returns the terminal's support for Sixel.
pub fn is_sixel_supported() -> bool {
    *SIXEL_SUPPORT
}

impl Printer for SixelPrinter {
    fn print(
        &self,
        stdout: &mut impl Write,
        img: &DynamicImage,
        config: &Config,
    ) -> Result<(u32, u32)> {
        let (w, mut h) = get_size_pix(img, config.width, config.height);

        // https://en.wikipedia.org/wiki/Sixel
        // a sixel is 1 pixel wide
        // a sexel is 6 pixels in height
        // so we make the final image height a multiple of 6 which is less than or equal to h*char_height
        h = (h/6)*6;

        let resized_img =
            img.resize_exact(w, h, FilterType::Triangle);

        let (width, height) = resized_img.dimensions();

        let rgba = resized_img.to_rgba8();
        let raw = rgba.as_raw();

        adjust_offset(stdout, config)?;

        let encoder = Encoder::new().unwrap();

        encoder.set_encode_policy(EncodePolicy::Fast).unwrap();

        let frame = QuickFrameBuilder::new()
            .width(width as usize)
            .height(height as usize)
            .format(sixel_sys::PixelFormat::RGBA8888)
            .pixels(raw.to_vec());

        encoder.encode_bytes(frame).unwrap();

        Ok((w, h)) // returning the number of pixels rendered (not number of sixels)
    }
}

fn get_size_pix(img: &DynamicImage, width: Option<u32>, height: Option<u32>) -> (u32, u32) {
    let (img_width_pix, img_height_pix) = img.dimensions();

    let (scr_width_chars, scr_height_chars) = {
        let rc = termion::terminal_size().unwrap();
        (rc.0 as u32, rc.1 as u32)
    };

    let (char_width, char_height) = {
        let (scr_width, scr_height) = termion::terminal_size_pixels().unwrap();
        
        // terminal size in pixels can be a little bigger than the space where chars are printed.
        // so floor is needed
        (
            (scr_width as f32/scr_width_chars as f32) as u32,
            (scr_height as f32/scr_height_chars as f32) as u32
        )
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
    
    (new_width_pix, new_height_pix)
}

// Check if Sixel is within the terminal's attributes
// see https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Sixel-Graphics
// and https://vt100.net/docs/vt510-rm/DA1.html
fn check_device_attrs() -> Result<bool> {
    let mut term = Term::stdout();

    write!(&mut term, "\x1b[c")?;
    term.flush()?;

    let mut response = String::new();

    while let Ok(key) = term.read_key() {
        if let Key::Char(c) = key {
            response.push(c);
            if c == 'c' {
                break;
            }
        }
    }

    Ok(response.contains(";4;") || response.contains(";4c"))
}

// Check if Sixel protocol can be used
fn check_sixel_support() -> bool {
    if let Ok(term) = std::env::var("TERM") {
        match term.as_str() {
            "mlterm" | "yaft-256color" | "foot" | "foot-extra" => return true,
            "st-256color" | "xterm" | "xterm-256color" => {
                return check_device_attrs().unwrap_or(false)
            }
            _ => {
                if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
                    return term_program == "MacTerm";
                }
            }
        }
    }
    false
}
