
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use anyhow::Result;

use crate::{
    content::{
        stack::StateContext,
        action::ContentManagerAction,
        register::{
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
    service::{
        python::{
            action::PyAction,
            code::PyCodeBuilder,
            item::{
                YtMusic,
                Json,
            },
        },
        yt::ytmusic::{
            YTMusicSearchVideo,
            YTMusicSearchSong,
            YTMusicSearchAlbum,
        },
    },
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
            search_type: YTSearchType::Albums,
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

    fn get_search_action(&self, self_id: ContentProviderID) -> ContentManagerAction {
        // TODO: more search actions
        // https://ytmusicapi.readthedocs.io/en/latest/reference.html#ytmusicapi.YTMusic.search
        let code = PyCodeBuilder::new()
        .threaded()
        .get_data_func(
            format!(
                "
                    data = ytmusic.search('{}', filter='{}', limit=75, ignore_spelling=True)
                    data = json.dumps(data, indent=4)
                    return data
                ",
                self.search_term,
                self.search_type.ytmusic_filter(),
            ),
            Some(vec![
                YtMusic::new("ytmusic").into(),
                Json::new("json").into(),
            ]),
        )
        .dbg_func(
            "
                with open('config/temp/video_search.log', 'r') as f:
                    data = f.read()
                return data
            ",
            None,
        )
        .set_dbg_status(false)
        .build().unwrap();
        let callback: Box<dyn Fn(String) -> Result<ContentManagerAction> + Send + Sync> = match self.search_type {
            YTSearchType::Albums => {
                Box::new(move |res: String| -> Result<ContentManagerAction> {
                    // debug!("{res}");
                    let albums = serde_json::from_str::<Vec<YTMusicSearchAlbum>>(&res);
                    // dbg!(&albums);
                    let content_providers = albums?.into_iter().map(Into::into).collect();
                    // dbg!(&content_providers);
                    let action = vec![
                        ContentManagerAction::LoadContentProvider {
                            songs: Default::default(),
                            content_providers,
                            loader_id: self_id,
                        },
                        ContentManagerAction::RefreshDisplayContent,
                    ].into();
                    Ok(action)
                })
            }
            YTSearchType::Playlists => {
                todo!()
            }
            YTSearchType::Songs => {
                Box::new(move |res: String| -> Result<ContentManagerAction> {
                    // debug!("{res}");
                    let songs = serde_json::from_str::<Vec<YTMusicSearchSong>>(&res)?;
                    // dbg!(&songs);
                    let songs = songs.into_iter().map(Into::into).collect();
                    let action = vec![
                        ContentManagerAction::LoadContentProvider {
                            songs,
                            content_providers: Default::default(),
                            loader_id: self_id,
                        },
                        ContentManagerAction::RefreshDisplayContent,
                    ].into();
                    Ok(action)
                })
            }
            YTSearchType::Videos => {
                Box::new(move |res: String| -> Result<ContentManagerAction> {
                    // debug!("{res}");
                    let videos = serde_json::from_str::<Vec<YTMusicSearchVideo>>(&res)?;
                    let songs = videos.into_iter().map(Into::into).collect();
                    let action = vec![
                        ContentManagerAction::LoadContentProvider {
                            songs,
                            content_providers: Default::default(),
                            loader_id: self_id,
                        },
                        ContentManagerAction::RefreshDisplayContent,
                    ].into();
                    Ok(action)
                })
            }
        };
        PyAction::ExecCode {
            callback,
            code,
            id: self_id,
        }.into()
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

    fn select_editable(&mut self, ctx: &mut StateContext, self_id: ContentProviderID) -> ContentManagerAction {
        let i = ctx.last().selected_index();
        let opt = self.editables(ctx).skip(i).next().unwrap();
        match opt {
            Editables::Main(e) => {
                match e {
                    YTEEditables::SEARCH_TERM => {
                        let mut index = SelectedIndex::default();
                        index.select(i);
                        ctx.push(index);
                        let callback = move |me: &mut providers::ContentProvider, content: String| -> ContentManagerAction {
                            let cp = me.as_any_mut().downcast_mut::<Self>().unwrap();
                            cp.loaded = true;
                            cp.search_term = content;
                            cp.songs.clear();
                            cp.providers.clear();
                            cp.selected.select(0);
                            return vec![
                                ContentManagerAction::PopContentStack, // typing
                                ContentManagerAction::PopContentStack, // edit
                                cp.get_search_action(self_id)
                            ].into();
                        };
                        vec![
                            ContentManagerAction::EnableTyping {
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
                        ContentManagerAction::None
                    }
                }
            }
            Editables::SType(e) => {
                self.search_type = e;
                ContentManagerAction::PopContentStack
            }
        }
    }
}

impl Loadable for YTExplorer {
    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn load(&mut self, _: ContentProviderID) -> ContentManagerAction {
        ContentManagerAction::OpenEditForCurrent
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
    Albums,
    Songs,
    Videos,
    Playlists,
}
impl YTSearchType {
    fn iter() -> &'static [Self] {
        &[
            Self::Albums,
            Self::Songs,
            Self::Videos,
            Self::Playlists,
        ]
    }
    fn ytmusic_filter(&self) -> &'static str {
        // https://ytmusicapi.readthedocs.io/en/latest/reference.html#ytmusicapi.YTMusic.search
        match self {
            Self::Albums => "albums",
            Self::Songs => "songs",
            Self::Videos => "videos",
            Self::Playlists => "playlists",
        }
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

