
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


#[typetag::serde]
impl ContentProviderTrait for Queue {
    impliment_content_provider!(Queue, SongProvider, Provider, Display);
}
