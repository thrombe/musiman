
use std::fmt::{Debug, Display};

use crate::{
    song::Song,
    content_handler::{ContentType, ContentID, ID, ContentManager, LoadEntry, GetNames, MenuOptions, ActionEntry, ContentProviderID},
};


#[derive(Clone, Debug)]
pub struct ContentProvider {
    pub content: Vec<ID>,
    name: String,
    cp_type: ContentProviderType,
    pub selected_index: usize,
    loaded: bool,
}

#[derive(Clone, Copy, Debug)]
#[derive(PartialEq, Eq)]
pub enum ContentProviderContentType {
    Menu,
    Normal,
}
impl Default for ContentProviderContentType {
    fn default() -> Self {
        Self::Normal
    }
}

impl ContentProvider {
    pub fn get_content_names(&self, t: ContentProviderContentType) -> GetNames {
        match t {
            ContentProviderContentType::Menu => {
                GetNames::Names(
                    self.get_menu_options()
                    .into_iter()
                    .map(|o| {
                        o.get_name()
                        .replace("_", " ")
                        .to_lowercase()
                    })
                    .collect()
                )
            }
            ContentProviderContentType::Normal => {
                GetNames::IDS(&self.content)
            }
        }
    }

    pub fn has_menu(&self) -> bool {
        match self.cp_type {
            ContentProviderType::MainProvider => true,
            _ => todo!(),
        }
    }

    pub fn apply_option(&mut self, opt: ContentProviderMenuOptions, self_id: ContentProviderID) -> Option<ActionEntry> {
        match opt {
            ContentProviderMenuOptions::Main(o) => {
                match o {
                    MainContentProviderMenuOptions::ADD_FILE_EXPLORER => {
                        Some(ActionEntry::AddCPToCP { id: self_id, cp: Self::new_file_explorer("/home/issac/daata/phon-data/.musi/IsBac/".to_owned(), "File Explorer: ".to_owned()) })
                    }
                    _ => todo!()
                }
            }
        }
    }

    pub fn new_main_provider() -> Self {
        Self {
            content: vec![],
            name: "Main Provider".to_owned(),
            cp_type: ContentProviderType::MainProvider,
            selected_index: 0,
            loaded: true
        }
    }

    pub fn new_file_explorer(path: String, pre_name: String) -> Self {
        Self {
            content: vec![],
            name: pre_name + &path.rsplit_terminator("/").next().unwrap().to_owned(),
            cp_type: ContentProviderType::FileExplorer {path},
            selected_index: 0,
            loaded: false,
        }
    }

    // TODO: maybe return a list of songs/sp/spp so that the parent function can add? or is this better?
    /// can load from various sources like yt/local storage while being able to add stuff to s/sp/spp
    pub fn load(&mut self, id: ContentProviderID) -> Option<ActionEntry> {
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
                        sp.push(ContentProvider::new_file_explorer(e, "".to_owned()));
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
                Some(ActionEntry::LoadContentManager(LoadEntry {s, sp, loader: id}))
            }
            _ => panic!()
        }
    }

    pub fn get_menu_options(&self) -> Vec<ContentProviderMenuOptions> {
        match self.cp_type {
            ContentProviderType::MainProvider => {
                [
                    MainContentProviderMenuOptions::ADD_ARTIST_PROVIDER,
                    MainContentProviderMenuOptions::ADD_PLAYLIST_PROVIDER,
                    MainContentProviderMenuOptions::ADD_QUEUE_PROVIDER,
                    MainContentProviderMenuOptions::ADD_FILE_EXPLORER
                ].into_iter()
                .map(|o| ContentProviderMenuOptions::Main(o))
                .collect()
            }
            _ => panic!()
        }
    }
}


#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContentProviderMenuOptions {
    Main(MainContentProviderMenuOptions),
}
impl ContentProviderMenuOptions {
    fn get_name(self) -> String {
        match self {
            Self::Main(o) => {
                format!("{o:#?}")
            }
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainContentProviderMenuOptions {
    ADD_ARTIST_PROVIDER,
    ADD_PLAYLIST_PROVIDER,
    ADD_QUEUE_PROVIDER,
    ADD_FILE_EXPLORER,
}

#[derive(Clone, Debug)]
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
    Loading, // load the content manager in another thread and use this as placeholder and apply it when ready
}

impl ContentProvider {
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl ContentProvider {
    pub fn provide(&self) -> &Vec<ID> {
        &self.content
    }

    pub fn provide_mut(&mut self) -> &mut Vec<ID> {
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
    pub fn add(&mut self, content_identifier: ID) {
        self.provide_mut().push(content_identifier);
    }
    /// panics if out of bounds
    pub fn remove_using_index(&mut self, index: usize) -> ID {
        self.provide_mut().remove(index)
    }
    pub fn remove(&mut self, cid: ID) {
        self.provide_mut().iter().position(|&e| e == cid);
    }

}
