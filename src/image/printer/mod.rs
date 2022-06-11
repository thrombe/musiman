
use crossterm::{
    cursor::{
        MoveTo,
    },
    execute,
};
use std::io::Write;
use anyhow::Result;

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


// Move the cursor to a location from where it should start printing
fn adjust_offset(stdout: &mut impl Write, x_off: u16, y_off: u16) -> Result<()> {
    // If absolute_offset, move to (x,y).
    execute!(stdout, MoveTo(x_off, y_off))?;
    Ok(())
}

