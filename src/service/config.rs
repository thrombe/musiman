
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use toml;
use serde::{
    Deserialize,
    Serialize,
};
use once_cell::sync::OnceCell;
use dirs;
use std::{
    path::PathBuf,
    io::Read,
};

pub fn config() -> &'static Config {
    static CONFIG: OnceCell<Config> = OnceCell::new();
    CONFIG.get_or_init(|| {
        let dev_config_path = "./config/config.toml";
        let mut dev_config_file = std::fs::File::open(dev_config_path).unwrap();
        let mut buf = String::new();
        dev_config_file.read_to_string(&mut buf).unwrap();
        let config = toml::from_str::<ConfigBuilder>(&buf).unwrap();
        dbg!(&config, toml::to_string(&config), Config::from(config.clone()));
        config.into()
    })
}

type MaybeString = Option<String>;
type MaybePath = Option<PathBuf>;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ConfigBuilder {
    file_explorer_default_path: MaybeString,
    ytmusic_cookies_path: MaybePath,
    prefered_song_ext: MaybeString,
    music_path: MaybePath,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub file_explorer_default_path: PathBuf,
    pub ytmusic_cookies_path: MaybePath,
    pub prefered_song_ext: String,
    pub music_path: PathBuf,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            file_explorer_default_path: expand_path("~/"),
            ytmusic_cookies_path: None,
            prefered_song_ext: "mp3".into(),
            music_path: dirs::audio_dir().unwrap(), // FIX: not available in termux ??
        }
    }
}

impl From<ConfigBuilder> for Config {
    fn from(cb: ConfigBuilder) -> Self {
        let def = Self::default();
        Self {
            file_explorer_default_path: cb.file_explorer_default_path
            .map(expand_path)
            .unwrap_or(def.file_explorer_default_path),
            
            ytmusic_cookies_path: cb.ytmusic_cookies_path
            .map(expand_path)
            .map(|e| Some(e))
            .unwrap_or(def.ytmusic_cookies_path),

            prefered_song_ext: cb.prefered_song_ext
            .unwrap_or(def.prefered_song_ext),

            music_path: cb.music_path
            .map(expand_path)
            .unwrap_or(def.music_path)
        }
    }
}

fn expand_path<T: Into<PathBuf>>(path: T) -> PathBuf {
    let path: PathBuf = path.into();
    let path = if path.starts_with("~/") {
        dirs::home_dir().unwrap().join(path.components().skip(1).collect::<PathBuf>())
    } else {
        PathBuf::from(path)
    };
    let _ = path.to_str().unwrap(); // extra check for early crash
    path.canonicalize().unwrap()
}
