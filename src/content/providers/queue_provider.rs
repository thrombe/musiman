
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
        register::{
            ContentProviderID,
            SongID,
        },
        providers::{
            traits::{
                impliment_content_provider,
                ContentProviderTrait,
                Provider,
                CPProvider,
                YankDest,
                CPYankDest,
                YankContext,
            },
            queue::Queue,
        },
        display::{
            DisplayContext,
            DisplayState,
        },
        manager::{
            action::ContentManagerAction,
        },
    },
    app::{
        app::SelectedIndex,
        display::{
            Display,
            ListBuilder,
        },
    },
    service::editors::{
        Edit,
        YankAction,
        Yank,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueProvider {
    providers: Vec<ContentProviderID>,
    name: Cow<'static, str>,
    #[serde(skip_serializing, skip_deserializing, default = "Default::default")]
    selected: SelectedIndex,
}
impl Default for QueueProvider {
    fn default() -> Self {
        Self {
            providers: Default::default(),
            selected: Default::default(),
            name: "Queues".into(),
        }
    }
}

impl QueueProvider {
    pub fn add_queue(&mut self, id: ContentProviderID) -> Option<ContentProviderID> {
        self.providers.insert(0, id);
        if self.providers.len() > 50 {
            self.providers.pop()
        } else {
            None
        }
    }

    pub fn move_to_top(&mut self, index: usize) {
        let id = self.providers.remove(index);
        self.providers.insert(0, id);
    }
}

impl<'b> Display<'b> for QueueProvider {
    type DisplayContext = DisplayContext<'b>;
    fn display(&self, context: Self::DisplayContext) -> ListBuilder<'static> {
        let mut lb = ListBuilder::default();
        lb.title(Span::raw(self.get_name()));

        lb.items = match context.state {
            DisplayState::Normal => {
                self.ids()
                .map(|id| context.display_item(id))
                .collect()
            }
            
            DisplayState::Edit(_) => unreachable!(),
            DisplayState::Menu(_) => unreachable!(),
        };

        lb
    }

    fn get_name(&self) -> Cow<'static, str> {
        self.name.clone()
    }
}


impl Provider for QueueProvider {
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
        &mut self.selected
    }
    fn get_selected_index(&self) -> &SelectedIndex {
        &self.selected
    }
}
impl CPProvider for QueueProvider {
    fn add_provider(&mut self, id: ContentProviderID) {
        self.providers.push(id);
    }
    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
        Box::new(self.providers.iter())
    }
    fn providers_mut(&mut self) -> &mut Vec<ContentProviderID> {
        &mut self.providers
    }
}

impl YankDest<ContentProviderID> for QueueProvider {
    fn try_paste(&mut self, items: Vec<Yank<ContentProviderID>>, start_index: Option<usize>, self_id: ContentProviderID) -> YankAction {
        let num_items = self.providers.len();
        vec![
            YankAction::Callback {
                callback: Box::new(move |mut ctx: YankContext| {
                    let items = items.into_iter()
                    .filter_map(|y| {
                        let id = y.item;
                        let e = ctx.get_provider(id);
                        if e.as_any().downcast_ref::<Queue>().is_some() {
                            ctx.register(id); // for being saved in QueueProvider
                            Some(id)
                        } else {
                            if e.as_song_provider().map(|cp| cp.songs().count()).unwrap_or(0) == 0 {
                                None
                            } else {
                                let mut songs = vec![];
                                let q = Queue::new(e, id, |id: SongID| {songs.push(id)}).into();
                                songs.into_iter().for_each(|id| ctx.register(id));
                                Some(ctx.alloc_provider(q))
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                    // for being stored in Edit
                    items.iter().cloned().for_each(|id| ctx.register(id));
                    ctx.register(self_id);

                    let yank = items.into_iter()
                    .enumerate()
                    .map(|(i, id)| Yank {
                        item: id,
                        index: start_index.map(|j| j+i).unwrap_or(num_items + i),
                    })
                    .collect::<Vec<_>>();
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
                        YankAction::False, // cuz of custom PushEdit, we handle this here
                        ContentManagerAction::RefreshDisplayContent.into(),
                    ].into()
                }),
            },
        ].into()
    }
    fn dest_vec_mut(&mut self) -> Option<&mut Vec<ContentProviderID>> {
        Some(&mut self.providers)
    }
}

#[typetag::serde]
impl ContentProviderTrait for QueueProvider {
    impliment_content_provider!(QueueProvider, Provider, CPProvider, Display, CPYankDest);
}
