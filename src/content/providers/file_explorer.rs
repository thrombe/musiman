

use crate::{
    content::{
        song::tagged_file_song::TaggedFileSong,
        manager::action::ContentManagerAction,
        register::{
            SongID,
            ContentProviderID,
        },
        providers::{
            traits::{
                impliment_content_provider,
                ContentProviderTrait,
                Loadable,
                Provider,
                SongProvider,
                CPProvider,
            },
        },
        display::DisplayContext,
    },
    app::{
        app::SelectedIndex,
        display::{
            Display,
            ListBuilder,
        },
    },
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

    fn load(&mut self, id: ContentProviderID) -> ContentManagerAction {
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
                let file = e.to_str().unwrap();
                if file.ends_with(".m4a") {
                    match TaggedFileSong::from_file(file.into()).unwrap() { // BAD: unwrap
                        Some(song) => s.push(song.into()),
                        None => (),
                    }
                }
            }
        });
        ContentManagerAction::LoadContentProvider {songs: s, content_providers: sp, loader_id: id}
    }
}

impl<'b> Display<'b> for FileExplorer {
    type DisplayContext = DisplayContext<'b>;
    fn display(&self, _context: Self::DisplayContext) -> ListBuilder<'static> {
        todo!()
    }
    fn get_name<'a>(&self) -> std::borrow::Cow<'a, str> {
        todo!()
    }
}

impl ContentProviderTrait for FileExplorer {
    impliment_content_provider!(FileExplorer, Provider, SongProvider, CPProvider, Loadable, Display);
}
