
use console::{
    Key,
    Term,
};
use image::{
    imageops::FilterType,
    DynamicImage,
    GenericImageView,
    Pixel,
};
use lazy_static::lazy_static;
use std::io::Write;
use termion;
use anyhow::Result;

use crate::image::{
    printer::{
        adjust_offset,
    },
    config::Config,
};

use std::{
    os::raw::{
        c_uchar,
        c_char,
        c_int,
        c_void,
    },
    ptr,
    slice
};

#[allow(unused_imports)]
use sixel_sys::{
    sixel_output_new,
    sixel_dither_get,
    sixel_encode,
    sixel_output_set_palette_type,
    sixel_output_set_encode_policy,
    sixel_dither_create,
    sixel_dither_new,
    sixel_dither_initialize,
    sixel_dither_set_diffusion_type,
    sixel_dither_set_palette,
    sixel_dither_set_optimize_palette,
    sixel_dither_set_body_only,

    BuiltinDither,
    PixelFormat,
    MethodForLargest,
    MethodForRepColor,
    QualityMode,
    DiffusionMethod,
    PaletteType,
    EncodePolicy,
};



lazy_static! {
    static ref SIXEL_SUPPORT: bool = check_sixel_support();
}

/// Returns the terminal's support for Sixel.
pub fn is_sixel_supported() -> bool {
    *SIXEL_SUPPORT
}


extern "C" fn write_fn(data: *mut c_char, len: c_int, userdata: *mut c_void) -> c_int {
    unsafe {
        let output: &mut Vec<u8> = &mut *(userdata as *mut Vec<u8>);
        output.write_all(slice::from_raw_parts(data as *mut c_uchar, len as usize)).unwrap();
        0
    }
}

pub struct Sixel {
    output: Vec<u8>,
}

impl Sixel {
    pub fn new(img: &DynamicImage, config: &Config) -> Result<Self> {
        let (w, mut h) = get_size_pix(img, config.width, config.height);

        // https://en.wikipedia.org/wiki/Sixel
        // a sixel is 1 pixel wide
        // a sexel is 6 pixels in height
        // so we make the final image height a multiple of 6 which is less than or equal to h*char_height
        h = (h/6)*6;

        let img = img.resize_exact(w, h, FilterType::Triangle);

        let mut data = img.to_rgb8().to_vec();
        let data: *mut c_uchar = data.as_mut_ptr() as *mut c_uchar;
        let mut output: Vec<u8> = Vec::new();
        let mut sixel_output = ptr::null_mut();
        if unsafe {
            sixel_output_new(
                &mut sixel_output,
                Some(write_fn),
                &mut output as *mut _ as *mut c_void,
                ptr::null_mut()
            )
        } != 0 {
            return Err(anyhow::anyhow!("sixel_output_new error"));
        }
        let dither = unsafe {
            // sixel_dither_new(*mut dither, )
            // let dither = unsafe { sixel_dither_get(BuiltinDither::XTerm256) };
            let dither = sixel_dither_create(256); // cap of 256 ig
            let res = sixel_dither_initialize(
                dither,
                data,
                img.width() as i32,
                img.height() as i32,
                PixelFormat::RGB888,
                MethodForLargest::Auto,
                MethodForRepColor::AveragePixels,
                QualityMode::High, // histogram processing quality
            );
            if res != 0 {
                panic!();
            }
            // sixel_dither_set_palette(dither, ) // a new pallet is being created for the image anyway ig
            sixel_dither_set_diffusion_type(dither, DiffusionMethod::None);
            sixel_output_set_palette_type(sixel_output, PaletteType::RGB);
            sixel_output_set_encode_policy(sixel_output, EncodePolicy::Size);
            sixel_dither_set_optimize_palette(dither, 0); // 0 for do, 1 for don't
            dither
        };

        let result = unsafe {
            sixel_encode(data, img.width() as i32, img.height() as i32, 0, dither, sixel_output)
        };
        if result == 0 {
            Ok(Self {
                output,
            })
        } else {
            Err(anyhow::anyhow!("sixel_encode error"))
        }
    }

    pub fn print(&self, config: &Config) -> Result<()> {
        let mut stdout = std::io::stdout();
        adjust_offset(&mut stdout, config)?;
        write!(stdout, "{}", std::str::from_utf8(&self.output)?)?;
        Ok(())
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
