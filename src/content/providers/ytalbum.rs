
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use anyhow::Result;
use std::borrow::Cow;
use tui::{
    style::{
        Color,
        Style
    },
    text::Span,
};
use serde::{Serialize, Deserialize};

use crate::{
    content::{
        manager::action::ContentManagerAction,
        providers::{
            traits::{
                impliment_content_provider,
                SongProvider,
                Provider,
                Loadable,
                ContentProviderTrait,
            },
        },
        register::{
            SongID,
            ContentProviderID,
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
            Line,
            SelectedText,
            Item,
        },
    },
    service::{
        yt::{
            ytdl::YTDLPlaylist,
            ytmusic::YTMusicAlbum,
        },
        python::{
            action::PyAction,
            code::PyCodeBuilder,
            item::{
                YtMusic,
                Ytdl,
                Json,
            },
        },
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTAlbum {
    songs: Vec<SongID>,
    loaded: bool,
    id: YTAlbumID,
    name: Cow<'static, str>,
    #[serde(skip_serializing, skip_deserializing, default = "Default::default")]
    index: SelectedIndex,
}
impl YTAlbum {
    pub fn new_playlist_id<T: Into<Cow<'static, str>>>(name: T, playlist_id: T) -> Self {
        Self {
            songs: Default::default(),
            loaded: false,
            id: YTAlbumID::PlaylistID(playlist_id.into()),
            name: name.into(),
            index: Default::default(),
        }
    }
    pub fn new_browse_id<T: Into<Cow<'static, str>>>(name: T, browse_id: T) -> Self {
        Self {
            songs: Default::default(),
            loaded: false,
            id: YTAlbumID::BrowseID(browse_id.into()),
            name: name.into(),
            index: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum YTAlbumID {
    PlaylistID(Cow<'static, str>),
    BrowseID(Cow<'static, str>),
}

impl SongProvider for YTAlbum {
    fn add_song(&mut self, id: SongID) {
        self.songs.push(id)
    }
    fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a> {
        Box::new(self.songs.iter())
    }
}

impl Loadable for YTAlbum {
    fn is_loaded(&self) -> bool {
        self.loaded
    }
    fn load(&mut self, self_id: ContentProviderID) -> Result<ContentManagerAction> {
        let action = match &self.id {
            YTAlbumID::BrowseID(browse_id) => {
                vec![
                    PyAction::ExecCode {
                        code: PyCodeBuilder::new()
                        .threaded()
                        .set_dbg_status(false)
                        .func(
                            format!("
                                album_data = ytmusic.get_album('{browse_id}')
                                data = json.dumps(album_data, indent=4)
                                return data
                            "),
                            Some(vec![
                                Json::new("json").into(),
                                YtMusic::new("ytmusic").into(),
                            ]),
                        )
                        .dbg_func(
                            "
                                with open('config/temp/get_album_playlist_id.log', 'r') as f:
                                    data = f.read()
                                return data
                            ",
                            None,
                        )
                        .build()?,
                        callback: Box::new(move |res: String| -> Result<ContentManagerAction> {
                            // the data we get from here have songs not necessarily the music videos
                            // but the data we get from the playlistId has the music videos
                            // (music videos being the songs with album art rather than the ones with dances and stuff)
                            // debug!("{res}");
                            let ytm_album = serde_json::from_str::<YTMusicAlbum>(&res)?;
                            let action = ContentManagerAction::ReplaceContentProvider {
                                old_id: self_id,
                                cp: ytm_album.into(),
                            };
                            Ok(action)
                        })
                    }.into(),
                ].into()
            }
            YTAlbumID::PlaylistID(playlist_id) => {
                self.loaded = true;
                vec![
                    PyAction::ExecCode {
                        code: PyCodeBuilder::new()
                        .threaded()
                        .set_dbg_status(false)
                        .func(
                            format!("
                                data = ytdl.extract_info('{playlist_id}', download=False)
                                data = json.dumps(data, indent=4)
                                return data
                            "),
                            Some(vec![
                                Json::new("json").into(),
                                Ytdl::new("ytdl").into(),
                            ]),
                        )
                        .dbg_func(
                            "
                                with open('config/temp/get_playlist.log', 'r') as f:
                                    data = f.read()
                                return data
                            ",
                            None,
                        )
                        .build()?,
                        callback: Box::new(move |res: String| -> Result<ContentManagerAction> {
                            // debug!("{res}");
                            let playlist = serde_json::from_str::<YTDLPlaylist>(&res)?;
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
                    }.into(),
                ].into()
            }
        };
        Ok(action)
    }
}

impl Provider for YTAlbum {
    fn get_selected_index(&self) -> &SelectedIndex {
        &self.index
    }
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
        &mut self.index
    }
}

impl<'b> Display<'b> for YTAlbum {
    type DisplayContext = DisplayContext<'b>;
    fn display(&self, context: Self::DisplayContext) -> ListBuilder<'static> {
        let mut lb = ListBuilder::default();
        lb.title(Span::raw(self.get_name()));

        lb.items = match context.state {
            DisplayState::Normal => { // BAD: partially copied from yt_explorer
                self.songs
                .iter()
                .map(|id| context.songs.get(*id).unwrap())
                .map(|s| s.as_display().title())
                .map(String::from)
                .map(Span::from)
                .map(Line::new)
                .map(|line| Item {
                    text: vec![line],
                    selected_text: SelectedText::Style(Style::default().fg(Color::Rgb(200, 200, 0)))
                })
                .collect()
            }
            DisplayState::Menu(_) => unreachable!(),
            DisplayState::Edit(_) => unreachable!(),
        };

        lb
    }
    fn get_name(&self) -> Cow<'static, str> {
        self.name.clone()
    }
}

#[typetag::serde]
impl ContentProviderTrait for YTAlbum {
    impliment_content_provider!(YTAlbum, SongProvider, Loadable, Provider, Display);
}
