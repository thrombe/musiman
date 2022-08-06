
use std::borrow::Cow;

use tui::{
    text::{
        Span,
    },
    style::{
        Style,
        Color,
    },
};
use serde::{Serialize, Deserialize};

use crate::{
    content::{
        manager::action::ContentManagerAction,
        stack::StateContext,
        register::ContentProviderID,
        providers::{
            ContentProvider,
            traits::{
                impliment_content_provider,
                ContentProviderTrait,
                CPProvider,
                Menu,
                Provider,
            },
            file_explorer::FileExplorer,
            yt_explorer::YTExplorer,
            queue_provider::QueueProvider,
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
    service::config::config,
};


// https://play.rust-lang.org/?version=stable&mode=debug&edition=2015&gist=146d926f9829419f60e3302131ba9709
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainProvider {
    providers: Vec<ContentProviderID>,
    pub queue_provider: ContentProviderID,

    // pub artist_provider: ContentProviderID,
    name: Cow<'static, str>,

    // https://serde.rs/attr-default.html
    // https://serde.rs/attr-skip-serializing.html
    #[serde(skip_serializing, skip_deserializing, default = "Default::default")]
    selected: SelectedIndex,
}
// pub struct MainProviderBuilder {}

impl MainProvider {
    pub fn new(mut alloc: impl FnMut(ContentProvider) -> ContentProviderID, register: impl FnMut(ContentProviderID)) -> Self {
        let queue_provider = alloc(QueueProvider::default().into());

        let mut mp = Self {
            providers: Default::default(),
            name: Cow::from("main"),
            selected: Default::default(),
            queue_provider,
            // artist_provider: alloc(),
        };

        mp.load(alloc, register);
        mp
    }

    pub fn load(&mut self, mut alloc: impl FnMut(ContentProvider) -> ContentProviderID, mut register: impl FnMut(ContentProviderID)) {
        register(self.queue_provider);
        self.providers = vec![
            self.queue_provider,
            alloc(FileExplorer::new(config().file_explorer_default_path.to_str().unwrap().into()).into()),
            alloc(YTExplorer::new().into()),
        ];
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MainProviderMenuOption {
    ADD_ARTIST_PROVIDER,
    ADD_PLAYLIST_PROVIDER,
    ADD_FILE_EXPLORER,
    ADD_YT_EXPLORER,
}

impl CPProvider for MainProvider {
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

impl Menu for MainProvider {
    fn num_options(&self, ctx: &StateContext) -> usize {
        self.menu(ctx).count()
    }

    fn apply_option(&mut self, ctx: &mut StateContext, self_id: ContentProviderID) -> ContentManagerAction {
        let option = self.menu(ctx).skip(ctx.last().selected_index()).next().unwrap();
        match option {
            MainProviderMenuOption::ADD_ARTIST_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_PLAYLIST_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_FILE_EXPLORER => {
                vec![
                    ContentManagerAction::PopContentStack,
                    ContentManagerAction::AddCPToCPAndContentStack {
                        id: self_id,
                        cp: FileExplorer::new(config().file_explorer_default_path.to_str().unwrap().into()).into()
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
                self.ids()
                .map(|id| context.display_item(id))
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

#[typetag::serde]
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
        ].into_iter())
    }
}
