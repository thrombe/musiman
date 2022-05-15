
use crate::{
    song::Song,
    content_handler::{ContentType, Content, ContentID, ContentManager, LoadEntry},
};


#[derive(Clone)]
pub struct ContentProvider {
    pub content: Vec<ContentID>,
    name: String,
    cp_type: ContentProviderType,
    pub selected_index: usize,
    loaded: bool,
}

impl ContentProvider {
    pub fn new_main_provider() -> Self {
        Self {
            content: vec![],
            name: "Main Provider".to_owned(),
            cp_type: ContentProviderType::MainProvider,
            selected_index: 0,
            loaded: true
        }
    }

    pub fn new_file_explorer(path: String) -> Self {
        Self {
            content: vec![],
            name: path.rsplit_terminator("/").next().unwrap().to_owned(),
            cp_type: ContentProviderType::FileExplorer {path},
            selected_index: 0,
            loaded: false,
        }
    }

    // TODO: maybe return a list of songs/sp/spp so that the parent function can add? or is this better?
    /// can load from various sources like yt/local storage while being able to add stuff to s/sp/spp
    pub fn load(&mut self) -> Option<LoadEntry> {
        if self.loaded {return None}
        self.loaded = true;
        match &mut self.cp_type {
            ContentProviderType::FileExplorer {path} => {
                if self.content.len() != 0 {return None}
                let mut s = vec![];
                let mut sp = vec![];

                // TODO: no need to have two calls to read_dir + this has a lot of duplicated code
                std::fs::read_dir(&path).unwrap()
                .filter(|e| e.as_ref().map(|r| r.path().is_dir()).unwrap_or(false))
                .map(|res| res.map(|e| e.path()).unwrap().to_str().unwrap().to_owned())
                .for_each(|e| {
                        sp.push(ContentProvider::new_file_explorer(e));
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
                Some(LoadEntry {s, sp})
            }
            _ => panic!()
        }
    }

    pub fn get_menu_options(&self) -> Vec<MenuOptions> {
        match self.cp_type {
            ContentProviderType::MainProvider => {
                [
                    MenuOptions::ADD_ARTIST_PROVIDER,
                    MenuOptions::ADD_PLAYLIST_PROVIDER,
                    MenuOptions::ADD_QUEUE_PROVIDER,
                    MenuOptions::ADD_FILE_EXPLORER
                ].into_iter()
                .collect()
            }
            _ => panic!()
        }
    }
}


#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuOptions {
    ADD_ARTIST_PROVIDER,
    ADD_PLAYLIST_PROVIDER,
    ADD_QUEUE_PROVIDER,
    ADD_FILE_EXPLORER,
}

#[derive(Clone)]
enum ContentProviderType {
    Playlist,
    Queue,
    YTArtist,
    UnknownArtist,
    Album,
    Seperator,

    Playlists,
    Queues,
    Artists,
    Albums,
    FileExplorer {
        path: String,
    },

    MainProvider,
}

impl Content for ContentProvider {
    fn get_content_type() -> ContentType {
        ContentType::SongProvider
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

impl ContentProvider {
    pub fn provide(&self) -> &Vec<ContentID> {
        &self.content
    }

    pub fn provide_mut(&mut self) -> &mut Vec<ContentID> {
        &mut self.content
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn update_index(&mut self, i: usize) {
        self.selected_index = i;
    }

    /// panics if out of bounds
    pub fn swap(&mut self, a: usize, b:  usize) {
        self.provide_mut().swap(a, b)
    }

    // TODO: reimpliment these for all of the diff types of content providers
    pub fn add(&mut self, content_identifier: ContentID) {
        self.provide_mut().push(content_identifier);
    }
    /// panics if out of bounds
    pub fn remove_using_index(&mut self, index: usize) -> ContentID {
        self.provide_mut().remove(index)
    }
    pub fn remove(&mut self, cid: ContentID) {
        self.provide_mut().iter().position(|&e| e == cid);
    }

}
