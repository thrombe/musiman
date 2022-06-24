
use std::borrow::Cow;

use tui::{
    text::{
        Spans,
        Span,
    },
    style::{
        Style,
        Color,
    },
};

use crate::{
    content::{
        manager::action::ContentManagerAction,
        stack::StateContext,
        register::ContentProviderID,
        providers::{
            traits::{
                impliment_content_provider,
                ContentProviderTrait,
                CPProvider,
                Menu,
                Provider,
            },
            file_explorer::FileExplorer,
            yt_explorer::YTExplorer,
            FriendlyID,
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
            SelectedText,
            Item,
            ListBuilder,
            Line,
        },
    },
};


#[derive(Debug, Clone)]
pub struct MainProvider {
    providers: Vec<ContentProviderID>,
    name: Cow<'static, str>,
    selected: SelectedIndex,
}

impl Default for MainProvider {
    fn default() -> Self {
        Self {
            providers: Default::default(),
            name: Cow::from("main"),
            selected: Default::default(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MainProviderMenuOption {
    ADD_ARTIST_PROVIDER,
    ADD_PLAYLIST_PROVIDER,
    ADD_QUEUE_PROVIDER,
    ADD_FILE_EXPLORER,
    ADD_YT_EXPLORER,
}
impl Into<FriendlyID> for MainProviderMenuOption {
    fn into(self) -> FriendlyID {
        FriendlyID::String(
            String::from(
                format!("{self:#?}")
                .replace("_", " ")
                .to_lowercase()
            )
        )
    }
}

impl CPProvider for MainProvider {
    fn add_provider(&mut self, id: ContentProviderID) {
        self.providers.push(id);
    }
    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
        Box::new(self.providers.iter())
    }
}

impl Menu for MainProvider {
    fn menu_options<'a>(&'a self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID>> {
        Box::new(self.menu(ctx).map(Into::into))
    }
    
    fn apply_option(&mut self, ctx: &mut StateContext, self_id: ContentProviderID) -> ContentManagerAction {
        let option = self.menu(ctx).skip(ctx.last().selected_index()).next().unwrap();
        match option {
            MainProviderMenuOption::ADD_ARTIST_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_PLAYLIST_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_QUEUE_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_FILE_EXPLORER => {
                let path = "/home/issac/daata/phon-data/.musi/IsBac";
                let mut fe = FileExplorer::default();
                fe.name = Cow::from(format!("File Explorer: {dir}", dir = path.rsplit_terminator("/").next().unwrap()));
                fe.path = Cow::from(path);
                vec![
                    ContentManagerAction::PopContentStack,
                    ContentManagerAction::AddCPToCPAndContentStack {
                        id: self_id,
                        cp: fe.into()
                    },
                ].into()
            }
            MainProviderMenuOption::ADD_YT_EXPLORER => {
                vec![
                    ContentManagerAction::PopContentStack,
                    ContentManagerAction::AddCPToCPAndContentStack {
                        id: self_id,
                        cp: YTExplorer::new().into(),
                    },
                ].into()
            }
        }
    }
}

impl Provider for MainProvider {
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
        &mut self.selected
    }
    fn get_selected_index(&self) -> &SelectedIndex {
        &self.selected
    }    
}

impl<'b> Display<'b> for MainProvider {
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
            DisplayState::Menu(ctx) => {
                self.menu(ctx)
                .map(|o| {
                    format!("{o:#?}")
                    .replace("_", " ")
                    .to_lowercase()
                })
                .map(Span::from)
                .map(Line::new)
                .map(|line| Item {
                    text: vec![line],
                    selected_text: SelectedText::Style(Style::default().fg(Color::Rgb(200, 200, 0))),
                })
                .collect()
            }
            
            DisplayState::Edit(_) => unreachable!(),
        };

        lb
    }

    fn get_name<'a>(&'a self) -> std::borrow::Cow<'static, str> {
        self.name.clone()
    }
}

impl ContentProviderTrait for MainProvider {
    impliment_content_provider!(MainProvider, Provider, Menu, CPProvider, Display);
}

impl MainProvider {
    // TODO: fix
    fn menu(&self, ctx: &StateContext) -> Box<dyn Iterator<Item = MainProviderMenuOption>> {
        Box::new([
            MainProviderMenuOption::ADD_ARTIST_PROVIDER,
            MainProviderMenuOption::ADD_PLAYLIST_PROVIDER,
            MainProviderMenuOption::ADD_FILE_EXPLORER,
            MainProviderMenuOption::ADD_YT_EXPLORER,
            MainProviderMenuOption::ADD_QUEUE_PROVIDER,
        ].into_iter())
    }
}
