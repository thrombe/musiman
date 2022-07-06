
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use serde::{Deserialize, Serialize};
use serde_yaml;
use std::{
    fs::File,
    io::{
        BufReader,
        BufWriter, Read, Write,
    },
};
use anyhow::Result;

use crate::{
    content::{
        providers::ContentProvider,
        song::Song,
        register::{
            ContentRegister,
            ContentProviderID,
            SongID,
        },
    },
    service::config::config,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct DBHandler {
    pub songs: ContentRegister<Song, SongID>,
    pub content_providers: ContentRegister<ContentProvider, ContentProviderID>,
    pub main_provider: ContentProviderID,
}

impl DBHandler {
    pub fn try_load() -> Result<Option<Self>> {
        let db_path = config().db_path.as_path();
        let file = match File::open(db_path) {
            Ok(file) => file,
            Err(_) => return Ok(None), // no problem is file does not exist
        };
        let mut red = BufReader::new(file);
        let mut buf = String::new();
        red.read_to_string(&mut buf)?;
        let dbh = serde_yaml::from_str(&buf)?;
        Ok(dbh)
    }

    pub fn save(&self) -> Result<()> {
        let db_path = config().db_path.as_path();
        let mut w = BufWriter::new(File::create(db_path)?);
        let yaml = serde_yaml::to_string(self)?;
        write!(w, "{yaml}")?;
        Ok(())
    }
}