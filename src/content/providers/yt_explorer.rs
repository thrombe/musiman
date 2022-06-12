

use crate::{
    content::{
        stack::StateContext,
        action::ContentHandlerAction,
        manager::{
            SongID,
            ContentProviderID,
        },
        providers::{
            self,
            FriendlyID,
            traits::{
                impliment_content_provider,
                ContentProvider,
                Provider,
                Editable,
                SongProvider,
                CPProvider,
                Loadable,
            },
        },
    },
    app::app::SelectedIndex,
    service::python::action::PyAction,
};

#[derive(Debug, Clone)]
pub struct YTExplorer {
    songs: Vec<SongID>,
    providers: Vec<ContentProviderID>,
    name: String,
    selected: SelectedIndex,
    loaded: bool,
    search_term: String,
    search_type: YTSearchType,
}

impl Default for YTExplorer {
    fn default() -> Self {
        Self {
            songs: Default::default(),
            providers: Default::default(),
            selected: Default::default(),
            search_term: Default::default(),
            search_type: YTSearchType::Album,
            loaded: false,
            name: "Youtube".into(),
        }
    }
}

impl YTExplorer {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    // TODO: impliment this recursively for ease
    fn editables(&self, ctx: &StateContext) -> Box<dyn Iterator<Item = Editables>> {
        match ctx.len()-1 {
            0 => {
                Box::new(YTEEditables::iter().into_iter().cloned().map(Into::into))
            }
            1 => {
                let i = ctx.get(0).unwrap().selected_index();
                let opt = YTEEditables::iter()[i];
                match opt {
                    YTEEditables::SEARCH_TYPE => {
                        Box::new(YTSearchType::iter().into_iter().cloned().map(Into::into))
                    }
                    YTEEditables::SEARCH_TERM => {
                        Box::new(YTEEditables::iter().into_iter().cloned().map(Into::into))
                    }
                }
            }
            _ => unreachable!()
        }
    }
}

impl Provider for YTExplorer {
    fn get_name(&self) -> &str {
        &self.name
    }
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
        &mut self.selected
    }
    fn get_selected_index(&self) -> &SelectedIndex {
        &self.selected
    }

}
impl SongProvider for YTExplorer {
    fn add_song(&mut self, id: SongID) {
        self.songs.push(id);
    }
    fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a> {
        Box::new(self.songs.iter())
    }

}
impl CPProvider for YTExplorer {
    fn add_provider(&mut self, id: ContentProviderID) {
        self.providers.push(id);
    }
    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
        Box::new(self.providers.iter())
    }

}
impl Editable for YTExplorer {
    fn get_editables<'a>(&'a self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID> + 'a> {
        Box::new(self.editables(ctx).map(|e| {
            match e {
                Editables::Main(e) => {
                    match e {
                        YTEEditables::SEARCH_TERM => {
                            FriendlyID::String(e.to_string() + ": " + &self.search_term)
                        }
                        YTEEditables::SEARCH_TYPE => {
                            FriendlyID::String(e.to_string() + ": " + &self.search_type.to_string())
                        }
                    }
                }
                Editables::SType(e) => e.into(),
            }
        }))
    }

    fn select_editable(&mut self, ctx: &mut StateContext, self_id: ContentProviderID) -> ContentHandlerAction {
        let i = ctx.last().selected_index();
        let opt = self.editables(ctx).skip(i).next().unwrap();
        match opt {
            Editables::Main(e) => {
                match e {
                    YTEEditables::SEARCH_TERM => {
                        let mut index = SelectedIndex::default();
                        index.select(i);
                        ctx.push(index);
                        let callback = move |me: &mut providers::ContentProvider, content: String| -> ContentHandlerAction {
                            let cp = me.as_any_mut().downcast_mut::<Self>().unwrap();
                            cp.loaded = true;
                            cp.search_term = content;
                            cp.songs.clear();
                            cp.providers.clear();
                            cp.selected.select(0);
                            return vec![
                                ContentHandlerAction::PopContentStack, // typing
                                ContentHandlerAction::PopContentStack, // edit
                                match cp.search_type {
                                    YTSearchType::Album => {
                                        PyAction::AlbumSearch {
                                            term: cp.search_term.clone(),
                                            loader: self_id,
                                        }.into()
                                    }
                                    YTSearchType::Playlist => {
                                        todo!()
                                    }
                                    YTSearchType::Song => {
                                        PyAction::SongSearch {
                                            term: cp.search_term.clone(),
                                            loader: self_id,
                                        }.into()
                                    }
                                    YTSearchType::Video => {
                                        PyAction::VideoSearch {
                                            term: cp.search_term.clone(),
                                            loader: self_id,
                                        }.into()
                                    }
                                }
                            ].into();
                        };
                        vec![
                            ContentHandlerAction::EnableTyping {
                                content: self.search_term.clone(),
                                callback: Box::new(callback),
                                loader: self_id.into(),
                            },
                        ].into()
                    }
                    YTEEditables::SEARCH_TYPE => {
                        let mut index = SelectedIndex::default();
                        index.select(i);
                        ctx.push(index);
                        ContentHandlerAction::None
                    }
                }
            }
            Editables::SType(e) => {
                self.search_type = e;
                ContentHandlerAction::PopContentStack
            }
        }
    }
}

impl Loadable for YTExplorer {
    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn load(&mut self, _: ContentProviderID) -> ContentHandlerAction {
        ContentHandlerAction::OpenEditForCurrent
    }
}

impl ContentProvider for YTExplorer {
    impliment_content_provider!(YTExplorer, Provider, Loadable, Editable, SongProvider, CPProvider);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug,)]
enum Editables {
    Main(YTEEditables),
    SType(YTSearchType),
}
impl From<YTEEditables> for Editables {
    fn from(e: YTEEditables) -> Self {
        Self::Main(e)
    }
}
impl From<YTSearchType> for Editables {
    fn from(e: YTSearchType) -> Self {
        Self::SType(e)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum YTSearchType {
    Album,
    Song,
    Video,
    Playlist,
}
impl YTSearchType {
    fn iter() -> &'static [Self] {
        &[
            Self::Album,
            Self::Song,
            Self::Video,
            Self::Playlist,
        ]
    }
}
impl Into<FriendlyID> for YTSearchType {
    fn into(self) -> FriendlyID {
        FriendlyID::String(format!("{self:#?}"))
    }
}
impl ToString for YTSearchType {
    fn to_string(&self) -> String {
        format!("{self:#?}")
    }
}


#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum YTEEditables {
    SEARCH_TERM,
    SEARCH_TYPE,
}
impl ToString for YTEEditables {
    fn to_string(&self) -> String {
        format!("{self:#?}")
        .replace("_", " ")
        .to_lowercase()        
    }
}
impl YTEEditables {
    fn iter() -> &'static [Self] {
        &[
            Self::SEARCH_TYPE,
            Self::SEARCH_TERM,
        ]
    }
}

