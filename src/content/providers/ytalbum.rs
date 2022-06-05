

use crate::{
    content::{
        action::ContentHandlerAction,
        providers::traits::{
            impliment_content_provider,
            SongProvider,
            Provider,
            Loadable,
            ContentProvider,
        },
        manager::{
            SongID,
            ContentProviderID,
        },
    },
    app::app::SelectedIndex,
    service::yt::YTAction,
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
            YTAlbumID::BrowseID(id) => {
                vec![
                    YTAction::GetAlbumPlaylistId {
                        browse_id: id.clone(),
                        loader: self_id,
                    }.into(),
                ].into()
            }
            YTAlbumID::PlaylistID(id) => {
                vec![
                    YTAction::GetPlaylist {
                        playlist_id: id.to_owned(),
                        loader: self_id,
                    }.into()
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
