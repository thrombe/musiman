
use crate::{
    // song::Song,
    content_handler::{ContentType, Content},
};

pub struct SongProvider {}

pub struct SPProvider {}

enum SongProviderType {
    Playlist,
    Queue,
    YTArtist,
    UnknownArtist,
    Album,
    Seperator,
}

enum SPProviderType {
    Playlists,
    Queues,
    Artists,
    Albums,
    FileExplorer,
}

impl Content for SongProvider {
    fn get_content_type() -> ContentType {
        ContentType::SongProvider
    }
}

impl Content for SPProvider {
    fn get_content_type() -> ContentType {
        ContentType::SPProvider
    }
}

