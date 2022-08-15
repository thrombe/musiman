
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
        Style,
        Color,
    },
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
        Yank,
        Edit,
        YankAction,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Queue {
    pub songs: Vec<SongID>,
    pub name: Cow<'static, str>,
    #[serde(skip_serializing, skip_deserializing, default = "Default::default")]
    pub index: SelectedIndex,
    pub currently_playing: Option<usize>,
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
            currently_playing: None,
            source_cp: id, // this should not register. the cp might go down even if this persists
        }
    }

    pub fn contains_song(&mut self, song_id: SongID) -> Option<usize> {
        self.songs.iter().position(|id| *id == song_id)
    }

    /// panics if song is not in it
    pub fn select_song(&mut self, id: SongID) {
        let index = self.contains_song(id).unwrap();
        self.currently_playing = Some(index);
    }

    pub fn next_song(&mut self) -> Option<SongID> {
        self.currently_playing.as_mut().map(|i| {
            if *i+1 < self.songs.len() {
                *i = *i+1;
                Some(self.songs[*i])
            } else {
                None
            }
        })
        .flatten()
    }

    pub fn prev_song(&mut self) -> Option<SongID> {
        self.currently_playing.as_mut().map(|i| {
            if *i > 0 {
                *i = *i-1;
                
                self.songs.get(*i)
                .map(|&id| id)
            } else {
                None
            }
        })
        .flatten()
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

    // using custom implimentations instead of default trait method implimentations // BAD: code it too similar to the default implimentation
    fn remove_song(&mut self, id: SongID) -> Option<SongID> {
        let index = self.songs().position(|&i| i == id);
        match index {
            Some(index) => {
                let item_index = self.ids().position(|i| i == ID::from(id)).unwrap();
                if item_index <= Provider::get_selected_index(self).selected_index() {
                    self.selection_decrement();
                }
                if self.currently_playing.map(|i| item_index < i).unwrap_or(false) {
                    self.currently_playing = self.currently_playing.map(|i| i-1);
                }
                Some(self.songs_mut().remove(index))
            }
            None => None,
        }
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
                let mut items = self.ids()
                .map(|id| context.display_item(id))
                .collect::<Vec<_>>();
                self.currently_playing.map(|i| items[i].text_style(Style::default().fg(Color::Rgb(200, 100, 0))));
                items
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
    fn try_paste(&mut self, items: Vec<Yank<SongID>>, start_index: Option<usize>, self_id: ContentProviderID) -> YankAction {
        let num_items = self.songs.len();
        vec![
            YankAction::Callback {
                callback: Box::new(move |mut ctx: YankContext| {
                    items.iter().for_each(|y| {
                        ctx.register(y.item); // for being stored in the Queue
                        ctx.register(y.item); // for being stored in Edit
                    });
                    let yank = items.into_iter()
                    .enumerate()
                    .map(|(i, mut y)| {
                        y.index = start_index.map(|j| j+i).unwrap_or(num_items + i);
                        y
                    })
                    .collect::<Vec<_>>();
                    ctx.register(self_id); // for being stored in Edit
                    vec![
                        YankAction::PasteIntoProvider {
                            yank: yank.clone().into(),
                            yanked_to: self_id,
                            paste_pos: start_index,
                        },
                        YankAction::PushEdit {
                            edit: Edit::Pasted {
                                yank: yank.into(),
                                yanked_to: self_id,
                                paste_pos: start_index,
                            },
                        },
                        YankAction::False,
                        ContentManagerAction::RefreshDisplayContent.into(),
                    ].into()
                }),
            },
        ].into()
    }
    fn dest_vec_mut(&mut self) -> Option<&mut Vec<SongID>> {
        Some(&mut self.songs)
    }

    // using custom implimentations instead of default trait method implimentations // BAD: code it too similar to the default implimentation

    fn paste(&mut self, items: Vec<Yank<SongID>>, start_index: Option<usize>) {
        start_index
        .filter(|i| Provider::get_selected_index(self).selected_index() >= *i)
        .map(|_| (0..items.len()).for_each(|_| {self.selection_increment();}));

        start_index
        .filter(|i| self.currently_playing.map(|j| j >= *i).unwrap_or(false))
        .map(|_| (0..items.len()).for_each(|_| {self.currently_playing = self.currently_playing.map(|i| i+1);}));
        self.currently_playing = self.currently_playing.map(|i| i.min(self.songs.len()-1));
        
        let vecc = self.dest_vec_mut().unwrap();
        let len = vecc.len();
        items.into_iter()
        .enumerate()
        .map(|(i, y)| (start_index.map(|j| j+i).unwrap_or(len+i), y))
        .for_each(|(i, y)| vecc.insert(i, y.item));
    }

    /// each item will be at the associated index once the entire operation is done
    // fn try_insert(&mut self, items: Vec<(T, usize)>) -> ContentManagerAction;
    fn insert(&mut self, mut items: Vec<Yank<SongID>>) {
        let mut provider_index_counter = 0;
        let mut currently_playing_index_counter = 0;
        let selected_index = Provider::get_selected_index(self).selected_index();
        let currently_playing = self.currently_playing;
        let vecc = self.dest_vec_mut().unwrap();
        items.sort_by(|a, b| a.index.cmp(&b.index));
        let mut items = items.into_iter().peekable();
        let mut old_items = std::mem::replace(vecc, vec![]).into_iter().peekable();
        while items.peek().is_some() || old_items.peek().is_some() {
            if let Some(&y) = items.peek() {
                if vecc.len() == y.index || old_items.peek().is_none() {
                    vecc.push(items.next().unwrap().item);
                    if y.index <= selected_index+provider_index_counter {
                        provider_index_counter += 1;
                    }
                    if currently_playing.map(|index| y.index <= index+currently_playing_index_counter).unwrap_or(false) {
                       currently_playing_index_counter += 1;
                    }
                    continue;
                }
            }
            vecc.push(old_items.next().unwrap());
        }
        
        (0..provider_index_counter).for_each(|_| {self.selection_increment();});

        self.currently_playing = self.currently_playing.map(|i| i+currently_playing_index_counter);
        self.currently_playing = self.currently_playing.map(|i| i.min(self.songs.len()-1));
    }

    /// assuming T exists at the provided index (necessary for multiple of the same thing present in the list)
    fn remove(&mut self, mut items: Vec<Yank<SongID>>) {
        let mut provider_index_counter = 0;
        let mut currently_playing_index_counter = 0;
        let selected_index = Provider::get_selected_index(self).selected_index();
        let currently_playing = self.currently_playing;
        let vecc = self.dest_vec_mut().unwrap();
        items.sort_by(|a, b| a.index.cmp(&b.index));
        let mut items = items.into_iter().peekable();
        *vecc = vecc.into_iter()
        .enumerate()
        .filter_map(|(i, id)| {
            if let Some(y) = items.peek() {
                if y.item == *id && y.index == i {
                    let _ = items.next();
                    if i <= selected_index {
                        provider_index_counter += 1;
                    }
                    if currently_playing.map(|j| i <= j).unwrap_or(false) {
                        currently_playing_index_counter += 1;
                    }
                    return None;
                }
            }
            Some(*id)
        })
        .collect();

        (0..provider_index_counter).for_each(|_| {self.selection_decrement();});
        self.currently_playing = self.currently_playing.map(|i| {
            if i < currently_playing_index_counter {
                0
            } else {
                i-currently_playing_index_counter
            }
        });
        if items.peek().is_some() {
            dbg!(items.collect::<Vec<_>>());
            panic!("everything was not removed");
        }
    }
}

#[typetag::serde]
impl ContentProviderTrait for Queue {
    impliment_content_provider!(Queue, SongProvider, Provider, Display, SongYankDest);
}
