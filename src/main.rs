#![allow(dead_code)]

// #![allow(unused_variables)]
// #![allow(unused_imports)]

mod song;
mod content_providers;
mod content_handler;
mod content_manager;
mod image_handler;
mod ui;
mod editors;
mod db_handler;
mod notifier;
mod yt_manager;

use crate::ui::{
    App,
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
        EnableMouseCapture,
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


// #[tokio::main]
// async fn main() -> Result<()> {
fn main() -> Result<()> {

    // let ytm = yt_manager::YTManager::new()?;

    // use std::thread;
    // let t_handle = thread::spawn(|| {
    //     ytm.search_song().unwrap();
    // });
    // t_handle.join().unwrap();

    // use tokio::runtime::Runtime;
    // let rt = Runtime::new().unwrap();
    // let handle = rt.handle();
    // let t_handle = handle.spawn_blocking(|| {
    //     println!("now running on a worker thread");
    // });

    // use tokio::task;
    // let j_handle = task::spawn_blocking(|| -> Result<()>{
    //     println!("now running on a worker thread");
    //     ytm.search_song()
    // });
    
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
    app.run_app(&mut terminal).unwrap();

    restore_terminal(&mut terminal).unwrap();

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

