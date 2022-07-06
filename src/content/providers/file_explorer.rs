
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
use serde::{Serialize, Deserialize};

use crate::{
    content::{
        song::{
            tagged_file_song::TaggedFileSong,
            untagged_file_song::UntaggedFileSong,
        },
        manager::action::{
            ContentManagerAction,
        },
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
                Loadable,
                Provider,
                Editable,
                SongProvider,
                CPProvider,
            },
        },
        display::{
            DisplayContext,
            DisplayState,
        },
        stack::StateContext,
    },
    app::{
        app::SelectedIndex,
        display::{
            Display,
            Line,
            Item,
            SelectedText,
            ListBuilder,
            Marker,
            MarkerPos,
        },
    },
    service::editors::Yanker,
};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileExplorer {
    songs: Vec<SongID>,
    providers: Vec<ContentProviderID>,
    pub name: Cow<'static, str>,
    #[serde(skip_serializing, skip_deserializing, default = "Default::default")]
    selected: SelectedIndex,
    pub path: Cow<'static, str>,
    loaded: bool,
    child: bool,
}

#[derive(Debug, Clone, Copy)]
enum Editables {
    Path
}

impl FileExplorer {
    fn editables(&self, _: &StateContext) -> Box<dyn Iterator<Item = Editables>> {
        Box::new([Editables::Path].into_iter())
    }

    pub fn new(path: Cow<'static, str>) -> Self {
        let mut fe = FileExplorer::default();
        fe.name = Cow::from(format!("File Explorer: {dir}", dir = path.rsplit_terminator("/").next().unwrap()));
        fe.path = Cow::from(path);
        fe
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
            child: false,
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

    fn load(&mut self, id: ContentProviderID) -> Result<ContentManagerAction> {
        self.loaded = true;
        let path = self.path.clone();
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
                    child: true,
                    ..Default::default()
                }.into());
            } else if e.is_file() {
                let file_path = e.to_str().unwrap();
                let file = Probe::open(file_path)?; // file open error
                match file.guess_file_type() {
                    Ok(_) => { // FIX: this does not mean this is some kinda song???
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
        let action = vec![
            ContentManagerAction::LoadContentProvider {songs: s, content_providers: sp, loader_id: id},
            ContentManagerAction::RefreshDisplayContent,
        ].into();
        Ok(action)
    }
}

impl<'b> Display<'b> for FileExplorer {
    type DisplayContext = DisplayContext<'b>;
    fn display(&self, context: Self::DisplayContext) -> ListBuilder<'static> {
        let mut lb = ListBuilder::default();
        lb.title(Span::raw(self.get_name()));

        lb.items = match context.state {
            DisplayState::Normal => {
                let more_items = self.songs
                .iter()
                .map(|&id| {
                    let song = context.songs.get(id).unwrap();
                    let title = song.as_display().title();
                    let mut line = Line::new(Span::from(title.to_owned()));
                    let mut selected_line = line.clone();
                    selected_line.overwrite_style(Style::default().fg(Color::Rgb(200, 200, 0)));
                    if context.yanker.yanked_items.contains(&id.into()) {
                        let marker = Marker {symbol: Yanker::marker_symbol(), pos: MarkerPos::Left};
                        line.markers.push(marker.clone());
                        selected_line.markers.push(marker);
                    }
                    Item {
                        text: vec![line],
                        selected_text: SelectedText::Lines(vec![selected_line]),
                    }
                });
                
                let items = self.providers
                .iter()
                .map(|&id| {
                    let name = context.providers
                    .get(id)
                    .unwrap()
                    .as_display()
                    .get_name();
                    let mut line = Line::new(Span {content: name, style: Default::default()});
                    let mut selected_line = line.clone();
                    selected_line.overwrite_style(Style::default().fg(Color::Rgb(200, 200, 0)));
                    if context.yanker.yanked_items.contains(&id.into()) {
                        let marker = Marker {symbol: Yanker::marker_symbol(), pos: MarkerPos::Left};
                        line.markers.push(marker.clone());
                        selected_line.markers.push(marker);
                    }
                    Item {
                        text: vec![line],
                        selected_text: SelectedText::Lines(vec![selected_line]),
                    }
                });

                items.chain(more_items)
                .collect()
            }
            DisplayState::Edit(ctx) => {
                self.editables(ctx)
                .map(|e| {
                    match e {
                        Editables::Path => {
                            format!("{e:#?}: {path}", path=&self.path)
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

impl Editable for FileExplorer {
    fn select_editable(&mut self, ctx: &mut StateContext, self_id: ContentProviderID) -> ContentManagerAction {
        let i = ctx.last().selected_index();
        let e = self.editables(ctx).skip(i).next().unwrap();
        match e {
            Editables::Path => {
                let mut index = SelectedIndex::default();
                index.select(i);
                ctx.push(index);
                vec![
                    ContentManagerAction::EnableTyping {
                        content: self.path.as_ref().to_owned(),
                        loader: self_id.into(),
                        callback: Box::new(move |me: &mut ContentProvider, content: String| {
                            let cp = me.as_any_mut().downcast_mut::<Self>().unwrap();
                            let ids = cp.pop_all_ids();
                            *me = Self::new(content.into()).into();
                            vec![
                                ContentManagerAction::Unregister {
                                    ids,
                                },
                                ContentManagerAction::PopContentStack, // typing
                                ContentManagerAction::PopContentStack, // edit
                                ContentManagerAction::MaybePushToContentStack {id: self_id.into()},
                            ].into()
                        }),
                    },
                ].into()
            }
        }
    }
    fn num_editables(&self, ctx: &StateContext) -> usize {
        self.editables(ctx).count()
    }
}

#[typetag::serde]
impl ContentProviderTrait for FileExplorer {
    fn as_editable(&self) -> Option<&dyn Editable> {
        if self.child {
            None
        } else {
            Some(self)
        }
    }
    fn as_editable_mut(&mut self) -> Option<&mut dyn Editable> {
        if self.child {
            None
        } else {
            Some(self)
        }        
    }
    impliment_content_provider!(FileExplorer, Provider, SongProvider, CPProvider, Loadable, Display);
}
