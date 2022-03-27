// #![allow(unused_imports)]
#![allow(dead_code)]

mod song;
mod content_providers;
mod content_handler;
mod image_handler;
mod ui;

use ui::{
    App,
};

use std::{io};
use tui::{
    backend::{CrosstermBackend},
    Terminal,
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use anyhow::Result;


fn main() -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // terminal.draw(|f| {
    //     let size = f.size();
    //     let block = Block::default()
    //         .title("Block")
    //         .borders(Borders::ALL)
    //         .border_style(Style::default().fg(Color::White))
    //         .style(Style::default().bg(Color::Black))
    //         ;
    //     f.render_widget(block, size);
    // })?;

    // create app and run it
    let app = App::load();
    let res = app.run_app(&mut terminal);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;


    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

