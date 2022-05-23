
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

use std::fmt::{
    Debug,
};

use crate::{
    song::{
        Song,
    },
    content_handler::{
        GetNames,
        Action,
    },
    content_manager::{
        ContentProviderID,
        SongID,
        ID,
    },
};


#[derive(Clone, Debug)]
pub struct ContentProvider {
    pub providers: Vec<ContentProviderID>,
    pub songs: Vec<SongID>,
    name: String,
    pub cp_type: ContentProviderType,
    pub selected_index: usize,
    loaded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentProviderContentType {
    Menu,
    Normal,
    Edit(ContentProviderEditables),
}
impl Default for ContentProviderContentType {
    fn default() -> Self {
        Self::Normal
    }
}
impl ContentProviderContentType {
    /// panic if not edit
    pub fn edit(self) -> ContentProviderEditables {
        match self {
            ContentProviderContentType::Edit(e) => {
                e
            }
            _ => panic!("type was not edit")
        }
    }
}

impl ContentProvider {
    pub fn get_content_names(&self, t: ContentProviderContentType) -> GetNames {
        match t {
            ContentProviderContentType::Menu => {
                match self.cp_type {
                    _ => {
                        GetNames::Names( // TODO: maybe GetNames should have different types like GetNames::MenuOptions and stuff. else there is code repetition
                            self.get_menu_options()
                            .into_iter()
                            .map(|o| {
                                o.to_string()
                                .replace("_", " ")
                                .to_lowercase()
                            })
                            .collect()
                        )
                    }
                }
            }
            ContentProviderContentType::Normal => {
                match self.cp_type {
                    ContentProviderType::Queues => {
                        GetNames::IDS(
                            self.providers
                            .iter().cloned()
                            .map(Into::into)
                            .rev()
                            .collect()
                        )
                    }
                    _ => {
                        GetNames::IDS(
                            self.providers
                            .iter().cloned()
                            .map(Into::into)
                            .chain(
                                self.songs
                                .iter().cloned()
                                .map(Into::into)
                            )
                            .collect()
                        )
                    }
                }
            }
            ContentProviderContentType::Edit(i) => {
                match &self.cp_type {
                    ContentProviderType::YTExplorer { search_type, search_term } => {
                        match i {
                            ContentProviderEditables::None => {
                                GetNames::Names(
                                    vec![
                                        ContentProviderEditables::YTExplorer(YTExplorerEditables::SEARCH_TYPE)
                                        .to_string()
                                        .replace("_", " ")
                                        .to_lowercase()
                                        + &format!(": {search_type:#?}"),
                                        ContentProviderEditables::YTExplorer(YTExplorerEditables::SEARCH_TERM)
                                        .to_string()
                                        .replace("_", " ")
                                        .to_lowercase()
                                        + ": " + search_term,
                                    ]
                                )
                            }
                            ContentProviderEditables::YTExplorer(e) => {
                                match e {
                                    YTExplorerEditables::SEARCH_TERM => {
                                        GetNames::Names(
                                            vec![
                                                ContentProviderEditables::YTExplorer(YTExplorerEditables::SEARCH_TYPE)
                                                .to_string()
                                                .replace("_", " ")
                                                .to_lowercase()
                                                + &format!(": {search_type:#?}"),
                                                search_term.into(),
                                            ]
                                        )
                                    }
                                    YTExplorerEditables::SEARCH_TYPE => {
                                        GetNames::Names(
                                            self.get_editables(i)
                                            .into_iter()
                                            .map(|o| {
                                                o.to_string()
                                                .replace("_", " ")
                                                .to_lowercase()
                                            })
                                            .collect()
                                        )
                                    }
                                }
                            }
                            _ => { // BAD: i can be not related to YTExplorer and no compile errors
                                GetNames::Names(
                                    self.get_editables(i)
                                    .into_iter()
                                    .map(|o| {
                                        o.to_string()
                                        .replace("_", " ")
                                        .to_lowercase()
                                    })
                                    .collect()
                                )    
                            }
                        }
                    }
                    _ => {
                        GetNames::Names(
                            self.get_editables(i)
                            .into_iter()
                            .map(|o| {
                                o.to_string()
                                .replace("_", " ")
                                .to_lowercase()
                            })
                            .collect()
                        )
                    }
                }
            }
        }
    }

    pub fn has_menu(&self) -> bool {
        match self.cp_type {
            ContentProviderType::MainProvider => true,
            _ => todo!(),
        }
    }

    pub fn has_editables(&self) -> bool {
        match self.cp_type {
            ContentProviderType::YTExplorer {..} => true,
            _ => todo!(),
        }
    }

    pub fn apply_option(&mut self, opt: ContentProviderMenuOptions, self_id: ContentProviderID) -> Action {
        match opt {
            ContentProviderMenuOptions::Main(o) => {
                match o {
                    MainContentProviderMenuOptions::ADD_FILE_EXPLORER => {
                        Action::AddCPToCP {
                            id: self_id,
                            cp: Self::new_file_explorer(
                                "/home/issac/daata/phon-data/.musi/IsBac/".to_owned(),
                                "File Explorer: ".to_owned()
                            ),
                            new_cp_content_type: ContentProviderContentType::Normal,
                        }
                    }
                    MainContentProviderMenuOptions::ADD_QUEUE_PROVIDER => {
                        Action::AddCPToCP {
                            id: self_id,
                            cp: Self::new_queue_provider(),
                            new_cp_content_type: ContentProviderContentType::Normal,
                        }
                    }
                    MainContentProviderMenuOptions::ADD_YT_EXPLORER => {
                        Action::AddCPToCP {
                            id: self_id,
                            cp: Self::new_yt_explorer(),
                            new_cp_content_type: ContentProviderContentType::Edit(ContentProviderEditables::None),
                        }
                    }
                    _ => todo!()
                }
            }
        }
    }

    pub fn choose_editable(&mut self, index: usize, self_id: ContentProviderID, e: ContentProviderEditables) -> Action {
        dbg!(e);
        let editables = self.get_editables(e);
        match editables[index] {
            ContentProviderEditables::None => {
                panic!("nothing to choose")
            }
            ContentProviderEditables::YTExplorer(e) => {
                let mut id = self_id;
                id.set_content_type(ContentProviderContentType::Edit(e.into()));
                match e {
                    YTExplorerEditables::SEARCH_TYPE => {
                        Action::PushToContentStack { id }
                    }
                    YTExplorerEditables::SEARCH_TERM => {
                        vec![
                            Action::PushToContentStack { id },
                            Action::SetSelectedIndex { index },
                            Action::EnableTyping,
                        ].into()
                    }
                }
            }
            ContentProviderEditables::YTSearchType(e) => {
                match &mut self.cp_type {
                    ContentProviderType::YTExplorer { search_type, .. } => {
                        *search_type = e;
                        Action::PopContentStack
                    }
                    _ => panic!("bad path"),
                }
            }
        }
    }

    pub fn new_main_provider() -> Self {
        Self {
            providers: vec![],
            songs: vec![],
            name: "Main Provider".to_owned(),
            cp_type: ContentProviderType::MainProvider,
            selected_index: 0,
            loaded: true
        }
    }

    pub fn new_file_explorer(path: String, pre_name: String) -> Self {
        Self {
            providers: vec![],
            songs: vec![],
            name: pre_name + path.rsplit_terminator("/").next().unwrap(),
            cp_type: ContentProviderType::FileExplorer {path},
            selected_index: 0,
            loaded: false,
        }
    }

    pub fn new_queue_provider() -> Self {
        Self {
            providers: vec![],
            songs: vec![],
            name: "Queues".into(),
            selected_index: 0,
            loaded: true,
            cp_type: ContentProviderType::Queues,
        }
    }

    pub fn new_yt_explorer() -> Self {
        Self {
            providers: vec![],
            songs: vec![],
            name: "youtube".into(),
            cp_type: ContentProviderType::YTExplorer {
                search_term: "".into(),
                search_type: YTSearchType::Album,
            },
            selected_index: 0,
            loaded: true,
        }
    }

    /// can load from various sources like yt/local storage while being able to add stuff to s/sp/spp
    pub fn load(&mut self, id: ContentProviderID) -> Action {
        if self.loaded {return Action::None}
        self.loaded = true;
        match &mut self.cp_type {
            ContentProviderType::FileExplorer {path} => {
                let mut s = vec![];
                let mut sp = vec![];

                std::fs::read_dir(&path)
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .for_each(|e| {
                    if e.is_dir() {
                        let dir = e.to_str().unwrap().to_owned();
                        sp.push(ContentProvider::new_file_explorer(dir, "".to_owned()));
                    } else if e.is_file() {
                        let file = e.to_str().unwrap().to_owned();
                        if file.ends_with(".m4a") {
                            match Song::from_file(file).ok() {
                                Some(song) => s.push(song),
                                None => (),
                            }
                        }
                    }
                });

                Action::LoadContentManager {songs: s, content_providers: sp, loader_id: id}
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
                    MainContentProviderMenuOptions::ADD_FILE_EXPLORER,
                    MainContentProviderMenuOptions::ADD_YT_EXPLORER,
                ].into_iter()
                .map(ContentProviderMenuOptions::Main)
                .collect()
            }
            _ => todo!()
        }
    }

    pub fn get_editables(&self, e: ContentProviderEditables) -> Vec<ContentProviderEditables> {
        dbg!(e);
        match e {
            ContentProviderEditables::None => {
                match self.cp_type {
                    ContentProviderType::YTExplorer{..} => {
                        [
                            YTExplorerEditables::SEARCH_TYPE,
                            YTExplorerEditables::SEARCH_TERM,
                        ].into_iter()
                        .map(Into::into)
                        .collect()
                    }
                    _ => todo!()
                }                        
            }
            ContentProviderEditables::YTExplorer(e) => { // BAD: if self.cp_type is not related to yt_explorer, no errors. this should not compile
                match e {
                    YTExplorerEditables::SEARCH_TYPE => {
                        return [
                            YTSearchType::Album,
                            YTSearchType::Song,
                            YTSearchType::Playlist,
                            YTSearchType::Video,
                        ].into_iter()
                        .map(Into::into)
                        .collect()
                    }
                    YTExplorerEditables::SEARCH_TERM => panic!("should never happen"),
                }
            }
            _ => todo!(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    // /// panics if out of bounds
    // pub fn swap(&mut self, a: usize, b:  usize) {
    //     self.content.swap(a, b)
    // }
    pub fn add(&mut self, id: ID) {
        match id {
            ID::Song(id) => {
                self.songs.push(id)
            }
            ID::ContentProvider(id) => {
                self.providers.push(id)
            }
        }
    }
    /// panics if out of bounds
    pub fn get(&self, index: usize) -> ID {
        let providers = self.providers.len();
        let songs = self.songs.len();
        if index < providers {
                self.providers[index].into()
        } else if index < songs+providers {
                self.songs[index-providers].into()
        } else {
            panic!()
        }
    }
    pub fn get_size(&self) -> usize {
        self.providers.len() + self.songs.len()
    }
    // /// panics if out of bounds
    // pub fn remove_using_index(&mut self, index: usize) -> ID {
    //     self.content.remove(index)
    // }
    // /// panic if not in here
    // pub fn remove(&mut self, id: ID) {
    //     let i = self.content.iter().position(|&e| e == id).unwrap();
    //     self.content.remove(i);
    // }
}


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContentProviderMenuOptions {
    Main(MainContentProviderMenuOptions),
}
impl ToString for ContentProviderMenuOptions {
    fn to_string(&self) -> String {
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
    ADD_YT_EXPLORER,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContentProviderEditables {
    YTExplorer(YTExplorerEditables),
    YTSearchType(YTSearchType),
    None,
}
impl ToString for ContentProviderEditables {
    fn to_string(&self) -> String {
        match self {
            Self::YTExplorer(o) => {
                format!("{o:#?}")
            }
            Self::YTSearchType(o) => {
                format!("{o:#?}")
            }
            Self::None => {
                "none".into()
            }
        }        
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YTExplorerEditables {
    SEARCH_TYPE,
    SEARCH_TERM,
}
impl Into<ContentProviderEditables> for YTExplorerEditables {
    fn into(self) -> ContentProviderEditables {
        ContentProviderEditables::YTExplorer(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum YTSearchType {
    Album,
    Song,
    Video,
    Playlist,
}
impl Into<ContentProviderEditables> for YTSearchType {
    fn into(self) -> ContentProviderEditables {
        ContentProviderEditables::YTSearchType(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentProviderType {
    Playlist,
    Queue,
    YTArtist,
    LocalArtist,
    Album, //??
    
    Playlists,
    Queues,
    Artists,
    Albums, //??
    FileExplorer {
        path: String,
    },
    YTExplorer {
        search_type: YTSearchType,
        search_term: String,
    },
    
    MainProvider,
    Seperator,
    Loading, // load the content manager in another thread and use this as placeholder and apply it when ready
}

