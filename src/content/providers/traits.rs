
use std::fmt::Debug;

use crate::{
    content::{
        stack::StateContext,
        providers::FriendlyID,
        manager::{
            ContentProviderID,
            SongID,
            ID,
        },
        action::ContentHandlerAction,
    },
    app::app::SelectedIndex,
};

pub trait CPClone {
    fn cp_clone(&self) -> Box<dyn ContentProvider>;
}

impl<T> CPClone for T
    where T: 'static + Clone + Debug + ContentProvider
{
    fn cp_clone(&self) -> Box<dyn ContentProvider> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn ContentProvider> {
    fn clone(&self) -> Self {
        self.cp_clone()
    }
}


impl<T> From<T> for super::ContentProvider
    where T: ContentProvider + 'static
{
    fn from(t: T) -> Self {
        super::ContentProvider::new(Box::new(t) as Box<dyn ContentProvider>)
    }
}


pub trait ContentProvider
    where
        Self: std::fmt::Debug + Send + Sync + CPClone,
{
    fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a> {
        Box::new([].into_iter())
    }
    
    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
        Box::new([].into_iter())
    }

    fn ids<'a>(&'a self) -> Box<dyn Iterator<Item = ID> + 'a> {
        Box::new(
            self.providers()
            .map(Clone::clone)
            .map(Into::into)
            .chain(
                self.songs()
                .map(Clone::clone)
                .map(Into::into)
            )
        )
    }
    
    fn get_friendly_ids<'a>(&'a self) -> Box<dyn Iterator<Item = FriendlyID> + 'a> {
        Box::new(
            self
            .ids()
            .map(FriendlyID::ID)
        )
    }

    fn menu_options<'a>(&'a self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID>> {
        Box::new([].into_iter())
    }

    fn has_menu(&self) -> bool {
        let (min, max) = self.menu_options(&StateContext::default()).size_hint();
        // an iterator has exactly 0 elements iff it has atleast 0 and atmost 0 elements
        !(min > 0 && max.is_some() && max.unwrap() == 0)
    }

    fn load(&self, id: ContentProviderID) -> ContentHandlerAction {
        None.into()
    }

    fn get_size(&self) -> usize {
        self.ids().size_hint().0
    }

    fn add_song(&mut self, id: SongID);
    fn add_provider(&mut self, id: ContentProviderID);
    fn get_name(&self) -> &str;
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex;
    fn get_selected_index(&self) -> &SelectedIndex;
    fn get_selected(&self) -> ID {
        self
        .ids()
        .skip(
            self
            .get_selected_index()
            .selected_index()
        )
        .next()
        .unwrap() // should never fail (unless the indices are not managed properly ig)
        .to_owned()
    }
    fn selection_increment(&mut self) -> bool {
        let num_items = self.get_size();
        let i = self.get_selected_index_mut();
        if i.selected_index() < num_items-1 {
            i.select(i.selected_index()+1);
            true
        } else {
            false
        }
    }

    fn selection_decrement(&mut self) -> bool {
        let i = self.get_selected_index_mut();
        if i.selected_index() > 0 {
            i.select(i.selected_index()-1);
            true
        } else {
            false
        }
    }

    fn get_editables(&self) -> Box<dyn Iterator<Item = FriendlyID>> {
        Box::new([].into_iter())
    }
    
    fn has_editables(&self) -> bool {
        // implimentation is super similar to Self::has_menu
    
        let (min, max) = self.get_editables().size_hint();
        // an iterator has exactly 0 elements iff it has atleast 0 and atmost 0 elements
        !(min > 0 && max.is_some() && max.unwrap() == 0)
    }

    fn num_editables(&self) -> usize {
        self.get_editables().size_hint().0
    }

    fn apply_typed(&mut self, _: ContentProviderID, _: String) -> ContentHandlerAction {
        // BAD: eh?? really this?
        None.into()
    }

    fn apply_option(&mut self, ctx: &StateContext, self_id: ContentProviderID) -> ContentHandlerAction {
        None.into()
    }
}


