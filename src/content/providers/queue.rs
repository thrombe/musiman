
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use std::borrow::Cow;
use tui::{
    text::Span,
};
use serde::{Serialize, Deserialize};

use crate::{
    content::{
        providers::{
            ContentProvider,
            traits::{
                impliment_content_provider,
                SongProvider,
                Provider,
                ContentProviderTrait,
                YankContext,
                YankDest,
                SongYankDest,
            },
        },
        register::{
            SongID,
            ContentProviderID,
            ID,
        },
        display::{
            DisplayContext,
            DisplayState,
        },
        manager::action::ContentManagerAction,
    },
    app::{
        app::SelectedIndex,
        display::{
            Display,
            ListBuilder,
        },
    },
    service::editors::{
        Yanker,
        YankedContentType,
        Edit,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Queue {
    pub songs: Vec<SongID>,
    pub name: Cow<'static, str>,
    #[serde(skip_serializing, skip_deserializing, default = "Default::default")]
    pub index: SelectedIndex,
    pub source_cp: ContentProviderID, // weak
}
impl Queue {
    /// panics is cp is not a SongProvider
    pub fn new(cp: &ContentProvider, id: ContentProviderID, register: impl FnMut(SongID)) -> Queue {
        cp
        .as_song_provider()
        .unwrap()
        .songs()
        .cloned()
        .for_each(register);
        Queue {
            songs: cp
            .as_song_provider()
            .unwrap()
            .songs()
            .cloned()
            .collect(),
            
            name: cp
            .as_display()
            .get_name(),

            index: Default::default(),
            source_cp: id, // this should not register. the cp might go down even if this persists
        }
    }

    pub fn contains_song(&mut self, song_id: SongID) -> Option<usize> {
        self.songs.iter().position(|id| *id == song_id)
    }

    /// panics if song is not in it
    pub fn select_song(&mut self, id: SongID) {
        let index = self.contains_song(id).unwrap();
        self.index.select(index);
    }
}

impl SongProvider for Queue {
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

impl Provider for Queue {
    fn get_selected_index(&self) -> &SelectedIndex {
        &self.index
    }
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
        &mut self.index
    }
}

impl<'b> Display<'b> for Queue {
    type DisplayContext = DisplayContext<'b>;
    fn display(&self, context: Self::DisplayContext) -> ListBuilder<'static> {
        let mut lb = ListBuilder::default();
        let title = format!(
            "Queue: {name}",
            name = self.get_name(),
        );
        lb.title(Span::raw(title));

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

impl YankDest<SongID> for Queue {
    fn try_paste(&mut self, items: Vec<SongID>, start_index: Option<usize>, self_id: ContentProviderID) -> ContentManagerAction {
        let num_items = self.songs.len();
        vec![
            ContentManagerAction::YankCallback {
                callback: Box::new(move |mut ctx: YankContext| {
                    items.iter().cloned().for_each(|id| {
                        ctx.register(id); // for being stored in the Queue
                        ctx.register(id); // for being stored in Edit
                    });
                    let mut yank = Yanker::new(self_id); // garbaage yanked_from
                    yank.content_type = YankedContentType::Song;
                    yank.yanked_items = items.into_iter()
                    .enumerate()
                    .map(|(i, id)| (ID::Song(id), start_index.map(|j| j+i).unwrap_or(num_items + i)))
                    .collect();
                    vec![
                        ContentManagerAction::PasteIntoProvider { yank: yank.clone(), yanked_to: self_id, paste_pos: start_index },
                        ContentManagerAction::PushEdit { edit: Edit::Pasted { yank, yanked_to: self_id, paste_pos: start_index } },
                        ContentManagerAction::RefreshDisplayContent,
                    ].into()
                }),
            },
        ].into()
    }
    fn dest_vec_mut(&mut self) -> Option<&mut Vec<SongID>> {
        Some(&mut self.songs)
    }
}

#[typetag::serde]
impl ContentProviderTrait for Queue {
    impliment_content_provider!(Queue, SongProvider, Provider, Display, SongYankDest);
}
