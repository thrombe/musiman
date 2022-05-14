
use crate::{
    song::Song,
    content_handler::{ContentType, Content, ContentProvider, ContentIdentifier, ContentManager, LoadEntry},
};

pub struct SongProvider {
    pub songs: Vec<ContentIdentifier>,
    name: String,
    sp_type: SongProviderType,
    pub selected_index: usize,
}

#[derive(Clone)]
pub struct SPProvider {
    pub sp_providers: Vec<ContentIdentifier>,
    name: String,
    spp_type: SPProviderType,
    pub selected_index: usize,
    loaded: bool,
}

impl SPProvider {
    pub fn new_file_explorer(path: String) -> Self {
        Self {
            sp_providers: vec![],
            name: path.rsplit_terminator("/").next().unwrap().to_owned(),
            spp_type: SPProviderType::FileExplorer {path},
            selected_index: 0,
            loaded: false,
        }
    }

    // TODO: maybe return a list of songs/sp/spp so that the parent function can add? or is this better?
    /// can load from various sources like yt/local storage while being able to add stuff to s/sp/spp
    pub fn load(&mut self) -> Option<LoadEntry> {
        if self.loaded {return None}
        self.loaded = true;
        match &mut self.spp_type {
            SPProviderType::FileExplorer {path} => {
                if self.sp_providers.len() != 0 {return None}
                let mut s = vec![];
                let mut spp = vec![];

                // TODO: no need to have two calls to read_dir + this has a lot of duplicated code
                std::fs::read_dir(&path).unwrap()
                .filter(|e| e.as_ref().map(|r| r.path().is_dir()).unwrap_or(false))
                .map(|res| res.map(|e| e.path()).unwrap().to_str().unwrap().to_owned())
                .for_each(|e| {
                        spp.push(SPProvider::new_file_explorer(e));
                    }
                );

                std::fs::read_dir(&path).unwrap()
                .filter(|e| e.as_ref().map(|r| r.path().is_file()).unwrap_or(false))
                .map(|res| res.map(|e| e.path()).unwrap().to_str().unwrap().to_owned())
                .filter(|s| s.ends_with(".m4a"))
                .for_each(|e| {
                        s.push(Song::from_file(e));
                    }
                );
                Some(LoadEntry {s, spp, sp: vec![]})
            }
            _ => panic!()
        }
    }
}

enum SongProviderType {
    Playlist,
    Queue,
    YTArtist,
    UnknownArtist,
    Album,
    Seperator,
}

#[derive(Clone)]
enum SPProviderType {
    Playlists,
    Queues,
    Artists,
    Albums,
    FileExplorer {
        path: String,
    },
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

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn update_index(&mut self, i: usize) {
        self.selected_index = i;
    }
}

impl ContentProvider for SPProvider {
    fn provide(&self) -> &Vec<ContentIdentifier> {
        &self.sp_providers
    }

    fn provide_mut(&mut self) -> &mut Vec<ContentIdentifier> {
        &mut self.sp_providers
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn update_index(&mut self, i: usize) {
        self.selected_index = i;
    }
}
