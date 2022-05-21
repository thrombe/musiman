
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

// impl Song {
    // pub fn load() -> Self {
    //     // theres also track and alt_title in case of ytm
    //     titles = [ // priority acc to index (low index is better) maybe check if in english?
    //         ytdl_data.get("track", None),
    //         ytdl_data.get("alt_title", None),
    //         ytdl_data["title"], // if this isnt there, something is wrong
    //         ]
    //     titles = [name for name in titles if name != None]

    //     video_id = ytdl_data["id"]
    //     duration = float(ytdl_data["duration"])
    //     tags = ytdl_data.get("tags", [])
    //     thumbnail_url = ytdl_data["thumbnail"]
    //     album = ytdl_data.get("album", "")
        
    //     artist_names = [ // priority acc to index (low index is better) maybe check if in english?
    //         ytdl_data.get("artist", None), // aimer
    //         ytdl_data.get("uploader", None), // aimer - topic
    //         ytdl_data.get("creator", None), //???
    //         ytdl_data["channel"], // aimer official youtube
    //     ]
    //     artist_names = [name for name in artist_names if name != None]

    //     // donno whats diff here
    //     channel_id = ytdl_data["channel_id"]
    //     uploader_id = ytdl_data["uploader_id"]

    //     Self {}
    // }
// }
