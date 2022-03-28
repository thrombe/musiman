
use crate::{
    // song::Song,
    content_handler::{ContentType, Content, ContentProvider, ContentIdentifier},
};

pub struct SongProvider {
    songs: Vec<ContentIdentifier>,
    name: String,
}

pub struct SPProvider {
    sp_providers: Vec<ContentIdentifier>,
    name: String,
}

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

    fn get_name(&self) -> &str {
        &self.name
    }
}

impl Content for SPProvider {
    fn get_content_type() -> ContentType {
        ContentType::SPProvider
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}


impl ContentProvider for SongProvider {
    fn provide(&self) -> &Vec<ContentIdentifier> {
        &self.songs
    }

    fn provide_mut(&mut self) -> &mut Vec<ContentIdentifier> {
        &mut self.songs
    }
}

impl ContentProvider for SPProvider {
    fn provide(&self) -> &Vec<ContentIdentifier> {
        &self.sp_providers
    }

    fn provide_mut(&mut self) -> &mut Vec<ContentIdentifier> {
        &mut self.sp_providers
    }
}
