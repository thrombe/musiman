
use serde::{Serialize, Deserialize};

use crate::{
    content_handler::{Content, ContentType},
};


#[derive(Debug, Deserialize, Serialize)]
pub struct Song {
    metadata: SongMetadata,
    stype: SongType,
}

#[derive(Debug, Deserialize, Serialize)]
enum SongMetadata {
    YTMetadata {
        url: String,
    },
    YTFileMetadata {
        path: String,
    },
    FileMetadata {
        path: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SongType {
    YTOnline,
    YTOnDisk,
    UnknownOnDisk,
    Seperator,
}

impl Content for Song {
    fn get_content_type() -> ContentType {
        ContentType::Song
    }
}

impl Song {
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
}
