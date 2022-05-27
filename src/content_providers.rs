
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

use std::{
    collections::HashMap,
    fmt::{
        Debug,
    },
};

use crate::{
    ui::SelectedIndex,
    song::{
        Song,
    },
    content_handler::{
        DisplayContent,
        ContentHandlerAction,
        ParallelAction,
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
    selected: HashMap<ContentProviderContentType, SelectedIndex>,
    loaded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContentProviderContentType {
    Menu,
    Normal,
    Edit(ContentProviderEditables),
}
impl From<ContentProviderEditables> for ContentProviderContentType {
    fn from(e: ContentProviderEditables) -> Self {
        Self::Edit(e)
    }
}
impl Default for ContentProviderContentType {
    fn default() -> Self {
        Self::Normal
    }
}

impl ContentProvider {
    pub fn new(name: String, t: ContentProviderType, loaded: bool) -> Self {
        Self {
            providers: Default::default(),
            songs: Default::default(),
            name,
            cp_type: t,
            selected: Default::default(),
            loaded,
        }
    }

    pub fn get_content_names(&self, t: ContentProviderContentType) -> DisplayContent {
        match t {
            ContentProviderContentType::Menu => {
                match self.cp_type {
                    _ => {
                        DisplayContent::Names( // TODO: maybe GetNames should have different types like GetNames::MenuOptions and stuff. else there is code repetition
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
                        DisplayContent::IDS(
                            self.providers
                            .iter().cloned()
                            .map(Into::into)
                            .rev()
                            .collect()
                        )
                    }
                    _ => {
                        DisplayContent::IDS(
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
                                DisplayContent::Names(
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
                                        DisplayContent::Names(
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
                                        DisplayContent::Names(
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
                                DisplayContent::Names(
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
                        DisplayContent::Names(
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
        dbg!(&self);
        match self.cp_type {
            ContentProviderType::YTExplorer {..} => true,
            _ => todo!(),
        }
    }

    pub fn apply_selected_option(&mut self, self_id: ContentProviderID) -> ContentHandlerAction {
        let t = self_id.get_content_type();
        let index = self.get_raw_selected_index(t);
        let opts = self.get_menu_options();
        let opt = opts[index];
        match opt {
            ContentProviderMenuOptions::Main(o) => {
                match o {
                    MainContentProviderMenuOptions::ADD_FILE_EXPLORER => {
                        vec![
                            ContentHandlerAction::PopContentStack,
                            ContentHandlerAction::AddCPToCPAndContentStack {
                                id: self_id,
                                cp: Self::new_file_explorer(
                                    "/home/issac/daata/phon-data/.musi/IsBac/".to_owned(),
                                    "File Explorer: ".to_owned()
                                ),
                                new_cp_content_type: ContentProviderContentType::Normal,
                            },
                        ].into()
                    }
                    MainContentProviderMenuOptions::ADD_QUEUE_PROVIDER => {
                        vec![
                            ContentHandlerAction::PopContentStack,
                            ContentHandlerAction::AddCPToCPAndContentStack {
                                id: self_id,
                                cp: Self::new_queue_provider(),
                                new_cp_content_type: ContentProviderContentType::Normal,
                            },
                        ].into()
                    }
                    MainContentProviderMenuOptions::ADD_YT_EXPLORER => {
                        vec![
                            ContentHandlerAction::PopContentStack,
                            ContentHandlerAction::AddCPToCPAndContentStack {
                                id: self_id,
                                cp: Self::new_yt_explorer(),
                                new_cp_content_type: ContentProviderContentType::Normal,
                            },
                        ].into()
                    }
                    _ => todo!()
                }
            }
        }
    }

    pub fn choose_selected_editable(&mut self, self_id: ContentProviderID, e: ContentProviderEditables) -> ContentHandlerAction {
        dbg!(e);
        let index = self.get_raw_selected_index(e.into());
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
                        ContentHandlerAction::PushToContentStack { id }
                    }
                    YTExplorerEditables::SEARCH_TERM => {
                        self.set_selected_index(id.get_content_type(), index);
                        match &mut self.cp_type {
                            ContentProviderType::YTExplorer { search_term , ..} => {
                                vec![
                                    ContentHandlerAction::PushToContentStack { id }, // TODO: check if i called back() after coming out of typing mode
                                    ContentHandlerAction::EnableTyping { content: search_term.clone()},
                                ].into()
                            }
                            _ => panic!(),
                        }
                    }
                }
            }
            ContentProviderEditables::YTSearchType(e) => {
                match &mut self.cp_type {
                    ContentProviderType::YTExplorer { search_type, .. } => {
                        *search_type = e;
                        ContentHandlerAction::PopContentStack
                    }
                    _ => panic!("bad path"),
                }
            }
        }
    }

    // BAD: wow. this looks aweful
    pub fn apply_typed(&mut self, self_id: ContentProviderID, content: String) -> ContentHandlerAction {
        let t = self_id.get_content_type();
        match t {
            ContentProviderContentType::Edit(e) => {
                match e {
                    ContentProviderEditables::YTExplorer(e) => {
                        match e {
                            YTExplorerEditables::SEARCH_TERM => {
                                match &mut self.cp_type {
                                    ContentProviderType::YTExplorer { search_term, search_type } => {
                                        *search_term = content;
                                        let mut id = self_id;
                                        // self.loaded = false;
                                        id.set_content_type(ContentProviderContentType::Normal);
                                        return vec![
                                            ContentHandlerAction::PopContentStack, // typing
                                            ContentHandlerAction::PopContentStack, // edit
                                            match search_type {
                                                YTSearchType::Album => {
                                                    ParallelAction::YTAlbumSearch {
                                                        term: search_term.clone(),
                                                        add_to: id,
                                                    }.into()
                                                }
                                                YTSearchType::Playlist => {
                                                    todo!()
                                                }
                                                YTSearchType::Song => {
                                                    todo!()
                                                }
                                                YTSearchType::Video => {
                                                    todo!()
                                                }
                                            }
                                        ].into();
                                    }
                                    _ => (),
                                }
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        }
        panic!(); // should not happen
    }

    pub fn new_main_provider() -> Self {
        Self {
            providers: vec![],
            songs: vec![],
            name: "Main Provider".to_owned(),
            cp_type: ContentProviderType::MainProvider,
            selected: Default::default(),
            loaded: true
        }
    }

    pub fn new_file_explorer(path: String, pre_name: String) -> Self {
        Self {
            providers: vec![],
            songs: vec![],
            name: pre_name + path.rsplit_terminator("/").next().unwrap(),
            cp_type: ContentProviderType::FileExplorer {path},
            selected: Default::default(),
            loaded: false,
        }
    }

    pub fn new_queue_provider() -> Self {
        Self {
            providers: vec![],
            songs: vec![],
            name: "Queues".into(),
            selected: Default::default(),
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
            selected: Default::default(),
            loaded: false,
        }
    }

    /// can load from various sources like yt/local storage while being able to add stuff to s/sp/spp
    pub fn load(&mut self, id: ContentProviderID) -> ContentHandlerAction {
        if self.loaded {return ContentHandlerAction::None}
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

                ContentHandlerAction::LoadContentManager {songs: s, content_providers: sp, loader_id: id}
            }
            ContentProviderType::YTExplorer { .. } => {
                let mut id = id;
                id.set_content_type(ContentProviderContentType::Edit(ContentProviderEditables::None));
                vec![
                    ContentHandlerAction::PushToContentStack { id },
                ].into()
            }
            ContentProviderType::Album {browse_id} => {
                vec![
                    ParallelAction::YTGetAlbum {
                        browse_id: browse_id.clone(),
                        add_to: id,
                    }.into(),
                ].into()
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

    pub fn get_selected_index(&mut self, t: ContentProviderContentType) -> &mut SelectedIndex {
        if !self.selected.contains_key(&t) {
            let i = SelectedIndex::new();
            self.selected.insert(t, i);
        }
        self.selected.get_mut(&t).unwrap()
    }

    pub fn selection_increment(&mut self, t: ContentProviderContentType) -> bool {
        let i = self.get_raw_selected_index(t);
        if i < self.get_size(t)-1 {
            self.set_selected_index(t, i+1);
            true
        } else {
            false
        }
    }

    pub fn selection_decrement(&mut self, t: ContentProviderContentType) -> bool {
        let i = self.get_raw_selected_index(t);
        if i > 0 {
            self.set_selected_index(t, i-1);
            true
        } else {
            false
        }
    }

    /// ensure that index is within limits
    fn set_selected_index(&mut self, t: ContentProviderContentType, index: usize) {
        let i = self.get_selected_index(t);
        i.select(index);
    }

    fn get_raw_selected_index(&self, t: ContentProviderContentType) -> usize {
        let i = if !self.selected.contains_key(&t) {
            SelectedIndex::new().selected_index()
        } else {
            self.selected.get(&t).unwrap().selected_index()
        };
        i
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
    pub fn get_selected(&self) -> ID {
        let index = self.get_raw_selected_index(ContentProviderContentType::Normal);
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
    pub fn get_size(&self, t: ContentProviderContentType) -> usize {
        match t {
            ContentProviderContentType::Normal => {
                self.providers.len() + self.songs.len()
            }
            ContentProviderContentType::Menu => {
                self.get_menu_options().len()
            }
            ContentProviderContentType::Edit(e) => {
                self.get_editables(e).len()
            }
        }
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum YTExplorerEditables {
    SEARCH_TYPE,
    SEARCH_TERM,
}
impl Into<ContentProviderEditables> for YTExplorerEditables {
    fn into(self) -> ContentProviderEditables {
        ContentProviderEditables::YTExplorer(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
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
    Album {
        browse_id: String,
    },
    
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
    Borked,
}

