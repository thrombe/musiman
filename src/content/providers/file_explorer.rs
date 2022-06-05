

use crate::{
    content::{
        song::Song,
        stack::StateContext,
        action::ContentHandlerAction,
        manager::{
            SongID,
            ContentProviderID,
        },
        providers::{
            FriendlyID,
            traits::ContentProvider,
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
