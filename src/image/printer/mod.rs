
use crossterm::{
    cursor::{
        MoveRight,
        MoveTo,
        MoveToPreviousLine,
    },
    execute,
};
use std::io::Write;
use anyhow::Result;

use crate::image::config::Config;

mod block;
pub use block::Block;

mod sixel;
pub use self::sixel::{
    is_sixel_supported,
    Sixel,
};

pub enum Printer {
    Block,
    Sixel,
}


// Move the cursor to a location from where it should start printing. Calculations are based on
// offsets from the config.
fn adjust_offset(stdout: &mut impl Write, config: &Config) -> Result<()> {
    if config.absolute_offset {
        if config.y >= 0 {
            // If absolute_offset, move to (x,y).
            execute!(stdout, MoveTo(config.x, config.y as u16))?;
        } else {
            //Negative values do not make sense.
            return Err(anyhow::anyhow!("absolute_offset is true but y offset is negative"));
        }
    } else if config.y < 0 {
        // MoveUp if negative
        execute!(stdout, MoveToPreviousLine(-config.y as u16))?;
        execute!(stdout, MoveRight(config.x))?;
    } else {
        // Move down y lines
        for _ in 0..config.y {
            // writeln! is used instead of MoveDown to force scrolldown
            // observed when config.y > 0 and cursor is on the last terminal line
            writeln!(stdout)?;
        }
        execute!(stdout, MoveRight(config.x))?;
    }
    Ok(())
}

