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
        // EnableMouseCapture,
    },
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use fern;
use log;
use anyhow::Result;
// use tokio;

pub use log::debug;

/// dbg macro but eprintln replaced with log::debug
/// https://github.com/rust-lang/rust/blob/3bcce82d14b85996c134420ac3c6790a410f7842/library/std/src/macros.rs#L287-L309
#[macro_export]
macro_rules! dbg {
    () => {
        log::debug!("[{}:{}]", $crate::file!(), $crate::line!());
    };
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                log::debug!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}


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

    init_logger().expect("failed to init logger");


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

fn init_logger() -> Result<()> {
    let mut base_config = fern::Dispatch::new();

    base_config = match 2 {
        0 => {
            // Let's say we depend on something which whose "info" level messages are too
            // verbose to include in end-user output. If we don't need them,
            // let's not include them.
            base_config
                .level(log::LevelFilter::Info)
                .level_for("overly-verbose-target", log::LevelFilter::Warn)
        }
        1 => base_config
            .level(log::LevelFilter::Debug)
            .level_for("overly-verbose-target", log::LevelFilter::Info),
        2 => base_config.level(log::LevelFilter::Debug),
        _3_or_more => base_config.level(log::LevelFilter::Trace),
    };

    let log_file = "config/temp/log.log";
    let _ = std::fs::remove_file(log_file);
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] {}",
                record.level(),
                message
            ))
        })
        .chain(fern::log_file(log_file)?);

    
    base_config
        .chain(file_config)
        .apply()?;

    Ok(())
}

