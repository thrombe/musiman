
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};


use std::{
    io,
    panic::{
        set_hook,
        take_hook,
    },
};
use tui::{
    backend::{
        CrosstermBackend,
        Backend,
    },
    Terminal,
};
use crossterm::{
    execute,
    event::{
        DisableMouseCapture,
        // EnableMouseCapture,
    },
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use anyhow::Result;
// use tokio;

use crate::{
    app::app::App,
    service::log::init_logger,
};


// #[tokio::main]
// async fn main() -> Result<()> {
pub fn run() -> Result<()> {
    init_logger().expect("failed to init logger");

    // yt_manager::test().unwrap();
    // return Ok(());

    let hook = take_hook();
    set_hook(Box::new(move |info| {
        // create new Terminal if panic
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend).unwrap();
    
        // restore terminal
        let _ = restore_terminal(&mut terminal); // ignore errors in panic hook
        hook(info)
    }));

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::load()?;
    app.run_app(&mut terminal)?;

    restore_terminal(&mut terminal)?;

    Ok(())
}

fn restore_terminal<B>(terminal: &mut Terminal<B>) -> Result<()> 
where B: Backend + std::io::Write,
{
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
