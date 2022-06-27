
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use std::borrow::Cow;
use tui::{
    text::Span,
    style::{
        Color,
        Style,
    },
};
use lofty::Probe;
use anyhow::Result;

use crate::{
    content::{
        song::{
            tagged_file_song::TaggedFileSong,
            untagged_file_song::UntaggedFileSong,
        },
        manager::action::{
            ContentManagerAction,
            RustParallelAction,
        },
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
        display::{
            DisplayContext,
            DisplayState,
        },
    },
    app::{
        app::SelectedIndex,
        display::{
            Display,
            Line,
            Item,
            SelectedText,
            ListBuilder,
        },
    },
};


#[derive(Debug, Clone)]
pub struct FileExplorer {
    songs: Vec<SongID>,
    providers: Vec<ContentProviderID>,
    pub name: Cow<'static, str>,
    selected: SelectedIndex,
    pub path: Cow<'static, str>,
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

impl Provider for FileExplorer {
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
        let path = self.path.clone();
        RustParallelAction::Callback {
            callback: Box::new(move || {
                let mut s = vec![];
                let mut sp = vec![];
                std::fs::read_dir(path.as_ref())?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .map(|e| {
                    if e.is_dir() {
                        let dir = e.to_str().unwrap();
                        sp.push(FileExplorer {
                            name: Cow::from(dir.rsplit_terminator("/").next().unwrap().to_owned()),
                            path: Cow::from(dir.to_owned()),
                            ..Default::default()
                        }.into());
                    } else if e.is_file() {
                        let file_path = e.to_str().unwrap();
                        let file = Probe::open(file_path)?; // file open error
                        match file.guess_file_type() {
                            Ok(_) => { // is some kinda song
                                match TaggedFileSong::from_file_path(file_path.into()) {
                                    Ok(Some(song)) => {
                                        s.push(song.into());
                                    }
                                    _ => {
                                        s.push(UntaggedFileSong::from_file_path(file_path.into()).into())
                                    }
                                }
                            }
                            Err(_) => (),
                        }
                    }
                    Ok(())
                })
                .for_each(|res: Result<()>| { // ignore errors from files that failed to read
                    match res {
                        Ok(_) => (),
                        Err(err) => {
                            error!("{err}")
                        }
                    }
                });
                // .collect::<Result<_>>()?;
                let action: ContentManagerAction = vec![
                    ContentManagerAction::LoadContentProvider {songs: s, content_providers: sp, loader_id: id},
                    ContentManagerAction::RefreshDisplayContent,
                ].into();
                Ok(action.into())
            }),
        }.into()
    }
}

impl<'b> Display<'b> for FileExplorer {
    type DisplayContext = DisplayContext<'b>;
    fn display(&self, context: Self::DisplayContext) -> ListBuilder<'static> {
        let mut lb = ListBuilder::default();
        lb.title(Span::raw(self.get_name()));

        lb.items = match context.state {
            DisplayState::Normal => { // BAD: code directly copied from yt_explorer. find a way to not duplicate code maybe using ContentProvider.ids() ??
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
            DisplayState::Menu(_) => unreachable!(),
            DisplayState::Edit(_) => unreachable!(),
        };

        lb
    }
    fn get_name(&self) -> Cow<'static, str> {
        self.name.clone()
    }
}

impl ContentProviderTrait for FileExplorer {
    impliment_content_provider!(FileExplorer, Provider, SongProvider, CPProvider, Loadable, Display);
}
