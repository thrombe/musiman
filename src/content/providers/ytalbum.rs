
use anyhow::Result;

use crate::{
    content::{
        action::ContentHandlerAction,
        providers::{
            traits::{
                impliment_content_provider,
                SongProvider,
                Provider,
                Loadable,
                ContentProvider,
            },
        },
        manager::{
            SongID,
            ContentProviderID,
        },
    },
    app::app::SelectedIndex,
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

#[derive(Debug, Clone)]
pub struct YTAlbum {
    songs: Vec<SongID>,
    loaded: bool,
    id: YTAlbumID,
    name: String,
    index: SelectedIndex,
}
impl YTAlbum {
    pub fn new_playlist_id(name: String, playlist_id: String) -> Self {
        Self {
            songs: Default::default(),
            loaded: false,
            id: YTAlbumID::PlaylistID(playlist_id),
            name,
            index: Default::default(),
        }
    }
    pub fn new_browse_id(name: String, browse_id: String) -> Self {
        Self {
            songs: Default::default(),
            loaded: false,
            id: YTAlbumID::BrowseID(browse_id),
            name,
            index: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
enum YTAlbumID {
    PlaylistID(String),
    BrowseID(String),
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
    fn load(&mut self, self_id: ContentProviderID) -> ContentHandlerAction {
        match &self.id {
            YTAlbumID::BrowseID(browse_id) => {
                vec![
                    PyAction::ExecCode {
                        code: PyCodeBuilder::new()
                        .threaded()
                        .set_dbg_status(false)
                        .get_data_func(
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
                        .build().unwrap(),
                        id: self_id,
                        callback: Box::new(move |res: String| -> Result<ContentHandlerAction> {
                            // the data we get from here have songs not necessarily the music videos
                            // but the data we get from the playlistId has the music videos
                            // (music videos being the songs with album art rather than the ones with dances and stuff)
                            // debug!("{res}");
                            let ytm_album = serde_json::from_str::<YTMusicAlbum>(&res)?;
                            let action = ContentHandlerAction::ReplaceContentProvider {
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
                        .get_data_func(
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
                        .build().unwrap(),
                        id: self_id,
                        callback: Box::new(move |res: String| -> Result<ContentHandlerAction> {
                            // debug!("{res}");
                            let playlist = serde_json::from_str::<YTDLPlaylist>(&res)?;
                            let action = vec![
                                ContentHandlerAction::LoadContentProvider {
                                    loader_id: self_id,
                                    songs: playlist.songs.into_iter().map(Into::into).collect(),
                                    content_providers: Default::default(),
                                },
                                ContentHandlerAction::RefreshDisplayContent,
                            ].into();
                            Ok(action)
                        })
                    }.into(),
                ].into()
            }
        }
    }
}

impl Provider for YTAlbum {
    fn get_name(&self) -> &str {
        &self.name
    }
    fn get_selected_index(&self) -> &SelectedIndex {
        &self.index
    }
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
        &mut self.index
    }
}



impl ContentProvider for YTAlbum {
    impliment_content_provider!(YTAlbum, SongProvider, Loadable, Provider);
}
