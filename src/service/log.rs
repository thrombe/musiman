use fern;
use log;
use anyhow::Result;

pub use log::{
    debug,
    error,
};



/// dbg macro but eprintln replaced with log::debug
/// https://github.com/rust-lang/rust/blob/3bcce82d14b85996c134420ac3c6790a410f7842/library/std/src/macros.rs#L287-L309
#[macro_export]
macro_rules! dbg {
    () => {
        // log::debug!("[{}:{}]", file!(), line!());
        log::debug!();
    };
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                // log::debug!("[{}:{}] {} = {:#?}", file!(), line!(), stringify!($val), &tmp);
                log::debug!("{} = {:#?}", stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}


pub fn init_logger() -> Result<()> {
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
                "[{}] [{}:{}] [{}] {}",
                record.level(),
                record.file().unwrap(),
                record.line().unwrap(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
                message,
            ))
        })
        .chain(fern::log_file(log_file)?);

    
    base_config
        .chain(file_config)
        .apply()?;

    Ok(())
}

