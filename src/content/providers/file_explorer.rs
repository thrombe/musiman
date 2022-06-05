

use crate::{
    content::{
        song::Song,
        action::ContentHandlerAction,
        manager::{
            SongID,
            ContentProviderID,
        },
        providers::{
            traits::{
                impliment_content_provider,
                ContentProvider,
                Loadable,
                Provider,
                SongProvider,
                CPProvider,
            },
        },
    },
    app::app::SelectedIndex,
};


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
    pub fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_owned() + path.rsplit_terminator("/").next().unwrap(),
            path: path.into(),
            ..Default::default()
        }
    }
}

impl Provider for FileExplorer {
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

impl SongProvider for FileExplorer {
    fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a> {
        Box::new(self.songs.iter())
    }

    fn add_song(&mut self, id: SongID) {
        self.songs.push(id);
    }
}

impl CPProvider for FileExplorer {
    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
        Box::new(self.providers.iter())
    }

    fn add_provider(&mut self, id: ContentProviderID) {
        self.providers.push(id);
    }    
}

impl Loadable for FileExplorer {
    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn load(&mut self, id: ContentProviderID) -> ContentHandlerAction {
        self.loaded = true;
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

impl ContentProvider for FileExplorer {
    impliment_content_provider!(FileExplorer, Provider, SongProvider, CPProvider, Loadable);
}
