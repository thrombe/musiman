
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use std::borrow::Cow;
use anyhow::Result;
use tui::{
    text::Span,
    style::{
        Color,
        Style,
    },
};
use serde::{Serialize, Deserialize};

use crate::{
    content::{
        stack::StateContext,
        manager::action::ContentManagerAction,
        register::{
            SongID,
            ContentProviderID,
            ID,
        },
        providers::{
            ContentProvider,
            traits::{
                impliment_content_provider,
                ContentProviderTrait,
                Provider,
                Editable,
                SongProvider,
                CPProvider,
                Loadable,
            },
        },
        display::{
            DisplayContext,
            DisplayState,
        },
    },
    app::{
        app::SelectedIndex,
        display::{
            Display,
            ListBuilder,
            Item,
            Line,
            SelectedText,
        },
    },
    service::{
        python::{
            action::{
                PyAction,
                PyCallback,
            },
            code::PyCodeBuilder,
            item::{
                YtMusic,
                Ytdl,
                Json,
            },
        },
        yt::{
            ytmusic::{
                YTMusicSearchVideo,
                YTMusicSearchSong,
                YTMusicSearchAlbum,
                YTMusicSearchPlaylist,
            },
            ytdl::{
                YTDLPlaylist,
                YtdlSong,
            },
        },
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTExplorer {
    songs: Vec<SongID>,
    providers: Vec<ContentProviderID>,
    name: Cow<'static, str>,
    selected: SelectedIndex,
    loaded: bool,
    search_term: String,
    filter: YTSearchFilter,
}

impl Default for YTExplorer {
    fn default() -> Self {
        Self {
            songs: Default::default(),
            providers: Default::default(),
            selected: Default::default(),
            search_term: Default::default(),
            filter: YTSearchFilter::Albums,
            loaded: false,
            name: "Youtube".into(),
        }
    }
}

impl<'b> Display<'b> for YTExplorer {
    type DisplayContext = DisplayContext<'b>;
    fn display(&self, context: Self::DisplayContext) -> ListBuilder<'static> {
        let mut lb = ListBuilder::default();
        lb.title(Span::raw(self.get_name()));

        lb.items = match context.state {
            DisplayState::Normal => {
                let items = self.songs
                .iter()
                .map(|id| context.songs.get(*id).unwrap())
                .map(|s| s.as_display().title())
                .map(String::from)
                .map(Span::from);
                
                let more_items = self.providers
                .iter()
                .map(|id| {
                    context.providers
                    .get(*id)
                    .unwrap()
                    .as_display()
                    .get_name()
                })
                .map(|c| Span {
                    content: c,
                    style: Default::default(),
                });

                items.chain(more_items)
                .map(Line::new)
                .map(|line| Item {
                    text: vec![line],
                    selected_text: SelectedText::Style(Style::default().fg(Color::Rgb(200, 200, 0)))
                })
                .collect()
            }
            
            DisplayState::Edit(ctx) => {
                self.editables(ctx)
                .map(|e| {
                    match e {
                        Editables::Main(e) => {
                            match e {
                                YTEEditables::SEARCH_TERM => e.to_string() + ": " + &self.search_term,
                                YTEEditables::FILTER => e.to_string() + ": " + &self.filter.to_string(),
                                YTEEditables::URL => e.to_string(),
                            }
                        }
                        Editables::SFilter(e) => {
                            e.to_string()
                        }
                        Editables::Url(e) => {
                            format!("{e:#?}")
                        }
                    }
                })
                .map(Span::from)
                .map(Line::new)
                .map(|line| Item {
                    text: vec![line],
                    selected_text: SelectedText::Style(Style::default().fg(Color::Rgb(200, 200, 0))),
                })
                .collect()
            }
            
            DisplayState::Menu(_) => unreachable!(),
        };

        lb
    }

    fn get_name(&self) -> Cow<'static, str> {
        self.name.clone()
    }
}

impl YTExplorer {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn pop_all_ids(&mut self) -> Vec<ID> {
        let songs = std::mem::replace(&mut self.songs, Default::default());
        let providers = std::mem::replace(&mut self.providers, Default::default());
        songs
        .into_iter()
        .map(Into::into)
        .chain(
            providers
            .into_iter()
            .map(Into::into)
        )
        .collect()
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
                match opt { // TODO: put filter and search_term inside SEARCH (as it is seperate from the url stuff)
                    YTEEditables::FILTER => {
                        Box::new(YTSearchFilter::iter().into_iter().cloned().map(Into::into))
                    }
                    YTEEditables::SEARCH_TERM => {
                        Box::new(YTEEditables::iter().into_iter().cloned().map(Into::into))
                    }
                    YTEEditables::URL => {
                        Box::new(YTUrl::iter().into_iter().cloned().map(Into::into))
                    }
                }
            }
            2 => {
                Box::new(YTUrl::iter().into_iter().cloned().map(Into::into))
            }
            _ => unreachable!()
        }
    }

    fn get_url_action(&self, self_id: ContentProviderID, url_type: YTUrl, url: String) -> ContentManagerAction {
        let code = PyCodeBuilder::new()
        .threaded()
        .func(
            format!(
                "
                    data = ytdl.extract_info('{url}', download=False)
                    data = json.dumps(data, indent=4)
                    return data
                ",
            ),
            Some(vec![
                Ytdl::new("ytdl").into(),
                Json::new("json").into(),
            ]),
        )
        .set_dbg_status(false)
        .build().unwrap();

        let callback: PyCallback = match url_type {
            YTUrl::Playlist => {
                Box::new(move |res: String| {
                    // debug!("{res}");
                    let playlist = serde_json::from_str::<YTDLPlaylist>(&res)?;
                    // dbg!(&playlist);

                    let action = vec![
                        ContentManagerAction::LoadContentProvider {
                            loader_id: self_id,
                            songs: playlist.songs.into_iter().map(Into::into).collect(),
                            content_providers: Default::default(),
                        },
                        ContentManagerAction::RefreshDisplayContent,
                    ].into();
                    Ok(action)
                })
            }
            YTUrl::Video => {
                Box::new(move |res: String| {
                    // debug!("{res}");
                    let song = serde_json::from_str::<YtdlSong>(&res)?;
                    // dbg!(&song);

                    let action = vec![
                        ContentManagerAction::LoadContentProvider {
                            loader_id: self_id,
                            songs: vec![song.into()],
                            content_providers: Default::default(),
                        },
                        ContentManagerAction::RefreshDisplayContent,
                    ].into();
                    Ok(action)
                })
            }
        };

        PyAction::ExecCode {
            code,
            callback,
        }.into()
    }

    fn get_search_action(&self, self_id: ContentProviderID) -> ContentManagerAction {
        // TODO: more search actions
        // https://ytmusicapi.readthedocs.io/en/latest/reference.html#ytmusicapi.YTMusic.search
        let code = PyCodeBuilder::new()
        .threaded()
        .func(
            format!(
                "
                    data = ytmusic.search('{}', filter='{}', limit=75, ignore_spelling=True)
                    data = json.dumps(data, indent=4)
                    return data
                ",
                self.search_term,
                self.filter.ytmusic_filter(),
            ),
            Some(vec![
                YtMusic::new("ytmusic").into(),
                Json::new("json").into(),
            ]),
        )
        .dbg_func(
            format!(
                "
                    with open('config/temp/{filter}_search.log', 'r') as f:
                        data = f.read()
                    return data
                ",
                filter = self.filter.ytmusic_filter(),
            ),
            None,
        )
        .set_dbg_status(false)
        .build().unwrap();
        let callback: PyCallback = match self.filter {
            YTSearchFilter::Albums => {
                Box::new(move |res: String| {
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
            YTSearchFilter::Playlists => {
                Box::new(move |res: String| {
                    // debug!("{res}");
                    let playlists = serde_json::from_str::<Vec<YTMusicSearchPlaylist>>(&res);
                    // dbg!(&playlists);
                    let content_providers = playlists?.into_iter().map(Into::into).collect();
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
            YTSearchFilter::Songs => {
                Box::new(move |res: String| {
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
            YTSearchFilter::Videos => {
                Box::new(move |res: String| {
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
        }.into()
    }
}

impl Provider for YTExplorer {
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
    fn num_editables(&self, ctx: &StateContext) -> usize {
        self.editables(ctx).count()
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
                        vec![
                            ContentManagerAction::EnableTyping {
                                content: self.search_term.clone(),
                                loader: self_id.into(),
                                callback: Box::new(move |me: &mut ContentProvider, content: String| {
                                    let cp = me.as_any_mut().downcast_mut::<Self>().unwrap();
                                    cp.loaded = true;
                                    cp.name = Cow::from(format!("Youtube: {content}"));
                                    cp.search_term = content;
                                    cp.selected.select(0);
                                    vec![
                                        ContentManagerAction::Unregister {
                                            ids: cp.pop_all_ids(),
                                        },
                                        ContentManagerAction::PopContentStack, // typing
                                        ContentManagerAction::PopContentStack, // edit
                                        ContentManagerAction::MaybePushToContentStack {id: self_id.into()},
                                        cp.get_search_action(self_id),
                                    ].into()
                                }),
                            },
                        ].into()
                    }
                    YTEEditables::FILTER => {
                        let index = SelectedIndex::default();
                        ctx.push(index);
                        ContentManagerAction::None
                    }
                    YTEEditables::URL => {
                        let index = SelectedIndex::default();
                        ctx.push(index);
                        ContentManagerAction::None
                    }
                }
            }
            Editables::SFilter(e) => {
                self.filter = e;
                ContentManagerAction::PopContentStack
            }
            Editables::Url(e) => {
                let mut index = SelectedIndex::default();
                index.select(i);
                ctx.push(index);

                ContentManagerAction::EnableTyping {
                    content: "".into(),
                    loader: self_id.into(),
                    callback: Box::new(move |me: &mut ContentProvider, content: String| {
                        let cp = me.as_any_mut().downcast_mut::<Self>().unwrap();
                        cp.loaded = true;
                        cp.name = Cow::from(format!("Youtube: {e:#?} Url"));
                        cp.selected.select(0);
                        vec![
                            ContentManagerAction::Unregister { ids: cp.pop_all_ids() },
                            ContentManagerAction::PopContentStack, // typing
                            ContentManagerAction::PopContentStack, // edit
                            ContentManagerAction::PopContentStack, // edit
                            ContentManagerAction::MaybePushToContentStack {id: self_id.into()},
                            cp.get_url_action(self_id, e, content),
                        ].into()
                    }),
                }
            }
        }
    }
}

impl Loadable for YTExplorer {
    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn load(&mut self, self_id: ContentProviderID) -> Result<ContentManagerAction> {
        Ok(vec![
            ContentManagerAction::PopContentStack,
            ContentManagerAction::OpenEditFor {id: self_id.into()},
        ].into())
    }
}

#[typetag::serde]
impl ContentProviderTrait for YTExplorer {
    impliment_content_provider!(YTExplorer, Provider, Loadable, Editable, SongProvider, CPProvider, Display);
}

#[derive(Clone, Copy, Debug)]
enum Editables {
    Main(YTEEditables),
    SFilter(YTSearchFilter),
    Url(YTUrl),
}
impl From<YTEEditables> for Editables {
    fn from(e: YTEEditables) -> Self {
        Self::Main(e)
    }
}
impl From<YTSearchFilter> for Editables {
    fn from(e: YTSearchFilter) -> Self {
        Self::SFilter(e)
    }
}
impl From<YTUrl> for Editables {
    fn from(e: YTUrl) -> Self {
        Self::Url(e)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum YTSearchFilter {
    Albums,
    Songs,
    Videos,
    Playlists,
}
impl YTSearchFilter {
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
impl ToString for YTSearchFilter {
    fn to_string(&self) -> String {
        format!("{self:#?}")
    }
}


#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum YTEEditables {
    SEARCH_TERM,
    FILTER,
    URL,
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
            Self::FILTER,
            Self::SEARCH_TERM,
            Self::URL,
        ]
    }
}

#[derive(Clone, Copy, Debug)]
enum YTUrl {
    Playlist,
    Video,
    // Channel,
}
impl YTUrl {
    fn iter() -> &'static [Self] {
        &[
            Self::Playlist,
            Self::Video,
        ]
    }
}
