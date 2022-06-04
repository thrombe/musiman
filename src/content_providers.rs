
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
    borrow::{Cow},
};

use crate::{
    ui::SelectedIndex,
    song::{
        Song,
    },
    content_handler::{
        self,
        DisplayContent,
        ContentHandlerAction, StateContext,
    },
    yt_manager::{
        YTAction,
    },
    content_manager::{
        ContentProviderID,
        SongID,
        ID,
    },
};

// pub struct Provider(Box<dyn ContentProvider>);

pub enum FriendlyID {
    String(String),
    ID(ID),
}

pub trait HumanReadable {
    
}

pub trait CPClone {
    fn cp_clone(&self) -> Box<dyn ContentProvider>;
}

impl<T> CPClone for T
    where T: 'static + Clone + Debug + ContentProvider
{
    fn cp_clone(&self) -> Box<dyn ContentProvider> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn ContentProvider> {
    fn clone(&self) -> Self {
        self.cp_clone()
    }
}

pub trait ContentProvider
    where
        Self: std::fmt::Debug + Send + Sync + CPClone,
{
    fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a> {
        Box::new([].into_iter())
    }
    
    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
        Box::new([].into_iter())
    }

    fn ids<'a>(&'a self) -> Box<dyn Iterator<Item = ID> + 'a> {
        Box::new(
            self.providers()
            .map(Clone::clone)
            .map(Into::into)
            .chain(
                self.songs()
                .map(Clone::clone)
                .map(Into::into)
            )
        )
    }
    
    fn get_friendly_ids<'a>(&'a self) -> Box<dyn Iterator<Item = FriendlyID> + 'a> {
        Box::new(
            self
            .ids()
            .map(FriendlyID::ID)
        )
    }

    fn menu_options<'a>(&'a self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID>> {
        Box::new([].into_iter())
    }

    fn has_menu(&self) -> bool {
        let (min, max) = self.menu_options(&StateContext::default()).size_hint();
        // an iterator has exactly 0 elements iff it has atleast 0 and atmost 0 elements
        !(min > 0 && max.is_some() && max.unwrap() == 0)
    }

    fn load(&self, id: ContentProviderID) -> ContentHandlerAction {
        None.into()
    }

    fn get_size(&self) -> usize {
        self.ids().size_hint().0
    }

    fn add_song(&mut self, id: SongID);
    fn add_provider(&mut self, id: ContentProviderID);
    fn get_name(&self) -> &str;
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex;
    fn get_selected_index(&self) -> &SelectedIndex;
    fn get_selected(&self) -> ID {
        self
        .ids()
        .skip(
            self
            .get_selected_index()
            .selected_index()
        )
        .next()
        .unwrap() // should never fail (unless the indices are not managed properly ig)
        .to_owned()
    }
    fn selection_increment(&mut self) -> bool {
        let num_items = self.get_size();
        let i = self.get_selected_index_mut();
        if i.selected_index() < num_items-1 {
            i.select(i.selected_index()+1);
            true
        } else {
            false
        }
    }

    fn selection_decrement(&mut self) -> bool {
        let i = self.get_selected_index_mut();
        if i.selected_index() > 0 {
            i.select(i.selected_index()-1);
            true
        } else {
            false
        }
    }

    fn get_editables(&self) -> Box<dyn Iterator<Item = FriendlyID>> {
        Box::new([].into_iter())
    }
    
    fn has_editables(&self) -> bool {
        // implimentation is super similar to Self::has_menu
    
        let (min, max) = self.get_editables().size_hint();
        // an iterator has exactly 0 elements iff it has atleast 0 and atmost 0 elements
        !(min > 0 && max.is_some() && max.unwrap() == 0)
    }

    fn num_editables(&self) -> usize {
        self.get_editables().size_hint().0
    }

    fn apply_typed(&mut self, _: ContentProviderID, _: String) -> ContentHandlerAction {
        // BAD: eh?? really this?
        None.into()
    }

    fn apply_option(&mut self, ctx: &StateContext, self_id: ContentProviderID) -> ContentHandlerAction {
        None.into()
    }
}

#[derive(Debug, Clone)]
pub struct FileExplorer {
    songs: Vec<SongID>,
    providers: Vec<ContentProviderID>,
    name: String,
    selected: SelectedIndex,
    path: String,
    loaded: bool,
}

impl Default for FileExplorer {
    fn default() -> Self {
        Self {
            songs: Default::default(),
            providers: Default::default(),
            name: "".into(),
            selected: Default::default(),
            path: "".into(),
            loaded: false,
        }
    }
}

impl FileExplorer {
    fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_owned() + path.rsplit_terminator("/").next().unwrap(),
            path: path.into(),
            ..Default::default()
        }
    }
}
enum FileExplorerMenuOption {
    Reset,
}
impl Into<FriendlyID> for FileExplorerMenuOption {
    fn into(self) -> FriendlyID {
        match self {
            Self::Reset => FriendlyID::String(String::from("reset"))
        }
    }
}

// simple implimentations can be yeeted away in a macro
impl<'b> ContentProvider for FileExplorer {
    fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a> {
        Box::new(self.songs.iter())
    }

    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
        Box::new(self.providers.iter())
    }

    fn menu_options<'a>(&'a self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID>> {
        Box::new([
            FileExplorerMenuOption::Reset.into(),
        ].into_iter())
    }

    fn add_song(&mut self, id: SongID) {
        self.songs.push(id);
    }

    fn add_provider(&mut self, id: ContentProviderID) {
        self.providers.push(id);
    }

    fn get_name(&self) -> &str {
        let a = self.name.as_ref();
        a
    }

    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
        &mut self.selected
    }
    fn get_selected_index(&self) -> &SelectedIndex {
        &self.selected
    }

    fn load(&self, id: ContentProviderID) -> ContentHandlerAction {
        let mut s = vec![];
        let mut sp = vec![];
        std::fs::read_dir(&self.path)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .for_each(|e| {
            if e.is_dir() {
                let dir = e.to_str().unwrap().to_owned();
                sp.push(FileExplorer::new("", &dir).into());
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
        ContentHandlerAction::LoadContentProvider {songs: s, content_providers: sp, loader_id: id}
    }
}



#[derive(Debug, Clone)]
pub struct MainProvider {
    providers: Vec<ContentProviderID>,
    name: Cow<'static, str>,
    selected: SelectedIndex,
}

impl Default for MainProvider {
    fn default() -> Self {
        Self {
            providers: Default::default(),
            name: Cow::from("main"),
            selected: Default::default(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MainProviderMenuOption {
    ADD_ARTIST_PROVIDER,
    ADD_PLAYLIST_PROVIDER,
    ADD_QUEUE_PROVIDER,
    ADD_FILE_EXPLORER,
    ADD_YT_EXPLORER,
}
impl Into<FriendlyID> for MainProviderMenuOption {
    fn into(self) -> FriendlyID {
        FriendlyID::String(
            String::from(
                format!("{self:#?}")
                .replace("_", " ")
                .to_lowercase()
            )
        )
        // match self {
        //     Self::ADD_ARTIST_PROVIDER => FriendlyID::Cow(Cow::from(stringify!(Self::ADD_ARTIST_PROVIDER))),
        //     Self::ADD_PLAYLIST_PROVIDER => todo!(),
        //     Self::ADD_QUEUE_PROVIDER => todo!(),
        //     Self::ADD_FILE_EXPLORER => todo!(),
        //     Self::ADD_YT_EXPLORER => todo!(),
        // }
    }
}

impl ContentProvider for MainProvider {
    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
        Box::new(self.providers.iter())
    }

    fn menu_options<'a>(&'a self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID>> {
        Box::new(self.menu(ctx).map(Into::into))
    }

    fn add_song(&mut self, id: SongID) {
        unreachable!()
        // BAD: eh?
    }

    fn add_provider(&mut self, id: ContentProviderID) {
        self.providers.push(id);
    }

    fn get_name(&self) -> &str {
        self.name.as_ref()
    }

    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
        &mut self.selected
    }
    fn get_selected_index(&self) -> &SelectedIndex {
        &self.selected
    }
    fn apply_option(&mut self, ctx: &StateContext, self_id: ContentProviderID) -> ContentHandlerAction {
        let option = self.menu(ctx).skip(ctx.last().selected_index()).next().unwrap();
        match option {
            MainProviderMenuOption::ADD_ARTIST_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_PLAYLIST_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_QUEUE_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_FILE_EXPLORER => {
                vec![
                    ContentHandlerAction::PopContentStack,
                    ContentHandlerAction::AddCPToCPAndContentStack {
                        id: self_id,
                        cp: FileExplorer::new(
                            "File Explorer: ",
                            "/home/issac/daata/phon-data/.musi/IsBac/",
                        ).into(),
                        // new_cp_content_type: ContentProviderContentType::Normal,
                    },
                ].into()
            }
            MainProviderMenuOption::ADD_YT_EXPLORER => todo!(),
        }
    }
}

impl MainProvider {
    fn menu(&self, ctx: &StateContext) -> Box<dyn Iterator<Item = MainProviderMenuOption>> {
        Box::new([
            MainProviderMenuOption::ADD_ARTIST_PROVIDER,
            MainProviderMenuOption::ADD_PLAYLIST_PROVIDER,
            MainProviderMenuOption::ADD_FILE_EXPLORER,
            MainProviderMenuOption::ADD_YT_EXPLORER,
            MainProviderMenuOption::ADD_QUEUE_PROVIDER,
        ].into_iter())
    }
}

impl<T> From<T> for content_handler::ContentProvider
    where T: ContentProvider + 'static
{
    fn from(t: T) -> Self {
        content_handler::ContentProvider::new(Box::new(t) as Box<dyn ContentProvider>)
    }
}
