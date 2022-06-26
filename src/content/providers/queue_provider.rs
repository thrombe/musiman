
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
            Item,
            Line,
            SelectedText,
        },
    },
};

#[derive(Debug, Clone)]
pub struct QueueProvider {
    providers: Vec<ContentProviderID>,
    name: Cow<'static, str>,
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
                self.providers
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
                })
                .map(Line::new)
                .map(|line| Item {
                    text: vec![line],
                    selected_text: SelectedText::Style(Style::default().fg(Color::Rgb(200, 200, 0)))
                })
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
}


impl ContentProviderTrait for QueueProvider {
    impliment_content_provider!(QueueProvider, Provider, CPProvider, Display);
}
