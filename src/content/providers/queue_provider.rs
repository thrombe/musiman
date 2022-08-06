
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
        },
        providers::{
            traits::{
                impliment_content_provider,
                ContentProviderTrait,
                Provider,
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
            ListBuilder,
        },
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

#[typetag::serde]
impl ContentProviderTrait for QueueProvider {
    impliment_content_provider!(QueueProvider, Provider, CPProvider, Display);
}
