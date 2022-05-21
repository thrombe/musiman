
use core::panic;

use serde::{
    Serialize,
    Deserialize,
};

use crate::{
    content_handler::{
        Action,
    },
};


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Song {
    metadata: SongMetadata,
    stype: SongType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
enum SongMetadata {
    YT {
        url: String,
    },
    YTFile {
        path: String,
    },
    File {
        path: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum SongMenuOptions {

}


#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SongType {
    YTOnline,
    YTOnDisk,
    UnknownOnDisk,
    Seperator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SongContentType {
    Menu,
    Normal,
    Edit,
}
impl Default for SongContentType {
    fn default() -> Self {
        Self::Normal
    }
}

impl Song {
    pub fn has_menu(&self) -> bool {
        true
    }

    pub fn get_menu_options(&self) -> Vec<SongMenuOptions> {
        vec![]
    }
    
    pub fn apply_option(&mut self, opt: SongMenuOptions) -> Option<Action> {
        match opt {

        }
    }

    // TODO: temporary implimentation
    pub fn get_name(&self) -> &str {
        match &self.metadata {
            SongMetadata::File { path } => {
                path.rsplit_terminator("/").next().unwrap()
            }
            _ => panic!()
        }
    }

    pub fn get_content_names(&self, t: SongContentType) -> Vec<String> {
        match t {
            SongContentType::Menu => {
                self.get_menu_options()
                .into_iter()
                .map(|o| {
                    format!("{o:#?}")
                    .replace("_", " ")
                    .to_lowercase()
                })
                .collect()
            }
            SongContentType::Edit => {
                // send_vec_of_self_details
                todo!()
            }
            SongContentType::Normal => {
                panic!("no content names in song Normal mode")
            }
        } 
    }
    
    pub fn from_file(path: String) -> Self {
        Self {
            metadata: SongMetadata::File { path },
            stype: SongType::UnknownOnDisk,
        }
    }
    
    pub fn path(&self) -> &str {
        match &self.metadata {
            SongMetadata::File { path } => {
                path
            }
            _ => panic!()
        }
    }
}

