
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use anyhow::Result;
use std::borrow::Cow;
use tui::{
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
        },
    },
    service::{
        yt::{
            // ytdl::YTDLPlaylist,
            ytmusic::YTMusicPlaylist,
        },
        python::{
            action::PyAction,
            code::PyCodeBuilder,
            item::{
                YtMusic,
                // Ytdl,
                Json,
            },
        },
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTPlaylist {
    songs: Vec<SongID>,
    loaded: bool,
    id: YTPlaylistID,
    name: Cow<'static, str>,
    #[serde(skip_serializing, skip_deserializing, default = "Default::default")]
    index: SelectedIndex,
}
impl YTPlaylist {
    // pub fn new_playlist_id<T: Into<Cow<'static, str>>>(name: T, playlist_id: T) -> Self {
    //     Self {
    //         songs: Default::default(),
    //         loaded: false,
    //         id: YTPlaylistID::PlaylistID(playlist_id.into()),
    //         name: name.into(),
    //         index: Default::default(),
    //     }
    // }
    pub fn new_browse_id<T: Into<Cow<'static, str>>>(name: T, browse_id: T) -> Self {
        Self {
            songs: Default::default(),
            loaded: false,
            id: YTPlaylistID::BrowseID(browse_id.into()),
            name: name.into(),
            index: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum YTPlaylistID {
    // PlaylistID(Cow<'static, str>),
    BrowseID(Cow<'static, str>),
}

impl SongProvider for YTPlaylist {
    fn add_song(&mut self, id: SongID) {
        self.songs.push(id)
    }
    fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a> {
        Box::new(self.songs.iter())
    }
    fn songs_mut(&mut self) -> &mut Vec<SongID> {
        &mut self.songs
    }
}

impl Loadable for YTPlaylist {
    fn is_loaded(&self) -> bool {
        self.loaded
    }
    fn load(&mut self, self_id: ContentProviderID) -> Result<ContentManagerAction> {
        let action = match &self.id {
            YTPlaylistID::BrowseID(browse_id) => {
                self.loaded = true;
                vec![
                    PyAction::ExecCode {
                        code: PyCodeBuilder::new()
                        .threaded()
                        .set_dbg_status(false)
                        .func(
                            format!("
                                playlist_data = ytmusic.get_playlist('{browse_id}')
                                data = json.dumps(playlist_data, indent=4)
                                return data
                            "),
                            Some(vec![
                                Json::new("json").into(),
                                YtMusic::new("ytmusic").into(),
                            ]),
                        )
                        .dbg_func(
                            "
                                with open('config/temp/get_playlist_from_browse_id.log', 'r') as f:
                                    data = f.read()
                                return data
                            ",
                            None,
                        )
                        .build().unwrap(),
                        callback: Box::new(move |res: String| -> Result<ContentManagerAction> {
                            // debug!("{res}");
                            let ytm_playlist = serde_json::from_str::<YTMusicPlaylist>(&res)?;
                            dbg!(&ytm_playlist);
                            let action = vec![
                                ContentManagerAction::LoadContentProvider {
                                    songs: ytm_playlist.tracks.into_iter().map(Into::into).collect(),
                                    content_providers: Default::default(),
                                    loader_id: self_id,
                                },
                                ContentManagerAction::RefreshDisplayContent,
                            ];
                            Ok(action.into())
                        })
                    }.into(),
                ].into()
            }
            // YTPlaylistID::PlaylistID(playlist_id) => {
            //     self.loaded = true;
            //     vec![
            //         PyAction::ExecCode {
            //             code: PyCodeBuilder::new()
            //             .threaded()
            //             .set_dbg_status(false)
            //             .func(
            //                 format!("
            //                     data = ytdl.extract_info('{playlist_id}', download=False)
            //                     data = json.dumps(data, indent=4)
            //                     return data
            //                 "),
            //                 Some(vec![
            //                     Json::new("json").into(),
            //                     Ytdl::new("ytdl").into(),
            //                 ]),
            //             )
            //             .dbg_func(
            //                 "
            //                     with open('config/temp/?.log', 'r') as f:
            //                         data = f.read()
            //                     return data
            //                 ",
            //                 None,
            //             )
            //             .build().unwrap(),
            //             callback: Box::new(move |res: String| -> Result<ContentManagerAction> {
            //                 // debug!("{res}");
            //                 let playlist = serde_json::from_str::<YTDLPlaylist>(&res)?;
            //                 let action = vec![
            //                     ContentManagerAction::LoadContentProvider {
            //                         loader_id: self_id,
            //                         songs: playlist.songs.into_iter().map(Into::into).collect(),
            //                         content_providers: Default::default(),
            //                     },
            //                     ContentManagerAction::RefreshDisplayContent,
            //                 ].into();
            //                 Ok(action)
            //             })
            //         }.into(),
            //     ].into()
            // }
        };
        Ok(action)
    }
}

impl Provider for YTPlaylist {
    fn get_selected_index(&self) -> &SelectedIndex {
        &self.index
    }
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
        &mut self.index
    }
}

impl<'b> Display<'b> for YTPlaylist {
    type DisplayContext = DisplayContext<'b>;
    fn display(&self, context: Self::DisplayContext) -> ListBuilder<'static> {
        let mut lb = ListBuilder::default();
        lb.title(Span::raw(self.get_name()));

        lb.items = match context.state {
            DisplayState::Normal => {
                self.ids()
                .map(|id| context.display_item(id))
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
impl ContentProviderTrait for YTPlaylist {
    impliment_content_provider!(YTPlaylist, SongProvider, Loadable, Provider, Display);
}
