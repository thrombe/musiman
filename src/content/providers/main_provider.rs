
use std::borrow::Cow;

use crate::{
    content::{
        action::ContentHandlerAction,
        manager::SongID,
        stack::StateContext,
        manager::ContentProviderID,
        providers::{
            FriendlyID,
            traits::ContentProvider,
            file_explorer::FileExplorer,
        },
    },
    app::app::SelectedIndex,
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

impl ContentProvider for MainProvider {
    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
        Box::new(self.providers.iter())
    }

    fn menu_options<'a>(&'a self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID>> {
        Box::new(self.menu(ctx).map(Into::into))
    }

    fn add_song(&mut self, id: SongID) {
        unreachable!()
        // BAD: eh?
    }

    fn add_provider(&mut self, id: ContentProviderID) {
        self.providers.push(id);
    }

    fn get_name(&self) -> &str {
        self.name.as_ref()
    }

    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
        &mut self.selected
    }
    fn get_selected_index(&self) -> &SelectedIndex {
        &self.selected
    }
    fn apply_option(&mut self, ctx: &StateContext, self_id: ContentProviderID) -> ContentHandlerAction {
        let option = self.menu(ctx).skip(ctx.last().selected_index()).next().unwrap();
        match option {
            MainProviderMenuOption::ADD_ARTIST_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_PLAYLIST_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_QUEUE_PROVIDER => todo!(),
            MainProviderMenuOption::ADD_FILE_EXPLORER => {
                vec![
                    ContentHandlerAction::PopContentStack,
                    ContentHandlerAction::AddCPToCPAndContentStack {
                        id: self_id,
                        cp: FileExplorer::new(
                            "File Explorer: ",
                            "/home/issac/daata/phon-data/.musi/IsBac/",
                        ).into(),
                        // new_cp_content_type: ContentProviderContentType::Normal,
                    },
                ].into()
            }
            MainProviderMenuOption::ADD_YT_EXPLORER => todo!(),
        }
    }
}

impl MainProvider {
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
