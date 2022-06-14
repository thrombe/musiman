
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use std::{
    fmt::Debug,
    any::Any,
};

use crate::{
    content::{
        stack::StateContext,
        providers::FriendlyID,
        register::{
            ContentProviderID,
            SongID,
            ID,
        },
        manager::action::ContentManagerAction,
    },
    app::app::SelectedIndex,
};

pub trait CPClone {
    fn cp_clone(&self) -> Box<dyn ContentProviderTrait>;
}

impl<T> CPClone for T
    where T: 'static + Clone + Debug + ContentProviderTrait
{
    fn cp_clone(&self) -> Box<dyn ContentProviderTrait> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn ContentProviderTrait> {
    fn clone(&self) -> Self {
        self.cp_clone()
    }
}


impl<T> From<T> for super::ContentProvider
    where T: ContentProviderTrait + 'static
{
    fn from(t: T) -> Self {
        super::ContentProvider::new(Box::new(t) as Box<dyn ContentProviderTrait>)
    }
}


// instead of implimenting this directly, implement the other traits (as required) and use the macro for
// the boilerplate implimentation of ContentProvider trait
// the default implimentations to the methods in this trait (that correspond to other traits) are done just
// so it compiles with somewhat reasonable values.
// the implimentations from the other traits should be prefered

// ? this requirement is quite dangerous time waster. can it be enforced?
// the macro must be called on all the traits, else those implimentations will not be used
pub trait ContentProviderTrait
    where
        Self: std::fmt::Debug + Send + Sync + CPClone + Any,
{
    fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a> {
        Box::new([].into_iter())
    }
    fn add_song(&mut self, _: SongID) {
        error!("this provider cannot store songs {self:#?}");
    }
    

    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
        Box::new([].into_iter())
    }
    fn add_provider(&mut self, _: ContentProviderID) {
        error!("this provider cannot store providers {self:#?}");
    }


    fn menu_options(&self, _: &StateContext) -> Box<dyn Iterator<Item = FriendlyID>> {
        Box::new([].into_iter())
    }
    fn has_menu(&self) -> bool {
        false
    }
    fn num_options(&self, _: &StateContext) -> usize {
        0
    }
    fn apply_option(&mut self, _: &mut StateContext, _: ContentProviderID) -> ContentManagerAction {
        ContentManagerAction::None
    }


    fn maybe_load(&mut self, _: ContentProviderID) -> ContentManagerAction {
        ContentManagerAction::None
    }
    fn load(&mut self, _: ContentProviderID) -> ContentManagerAction {
        ContentManagerAction::None
    }
    fn is_loaded(&self) -> bool {
        true
    }

    
    fn get_editables<'a>(&'a self, _: &StateContext) -> Box<dyn Iterator<Item = FriendlyID> + 'a> {
        Box::new([].into_iter())
    }
    fn has_editables(&self) -> bool {
        false
    }
    fn num_editables(&self, _: &StateContext) -> usize {
        0
    }
    fn select_editable(&mut self, _: &mut StateContext, _: ContentProviderID) -> ContentManagerAction {
        ContentManagerAction::None
    }


    // every content provider has to impliment these
    fn get_name(&self) -> &str;
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex;
    fn get_selected_index(&self) -> &SelectedIndex;


    // sensible default implimentations
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
    fn get_size(&self) -> usize {
        self.ids().size_hint().0
    }
    fn get_friendly_ids<'a>(&'a self) -> Box<dyn Iterator<Item = FriendlyID> + 'a> {
        Box::new(
            self
            .ids()
            .map(FriendlyID::ID)
        )
    }
    
    // for downcasting (the macro has implimentation for this)
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}


pub trait Loadable {
    fn maybe_load(&mut self, self_id: ContentProviderID) -> ContentManagerAction {
        if self.is_loaded() {
            ContentManagerAction::None
        } else {
            self.load(self_id)
        }
    }
    fn load(&mut self, self_id: ContentProviderID) -> ContentManagerAction;
    fn is_loaded(&self) -> bool;
}

pub trait Menu {
    fn apply_option(&mut self, ctx: &mut StateContext, self_id: ContentProviderID) -> ContentManagerAction;
    fn menu_options(&self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID>>;
    fn has_menu(&self) -> bool {
        let (min, max) = self.menu_options(&StateContext::default()).size_hint();
        // an iterator has exactly 0 elements iff it has atleast 0 and atmost 0 elements
        !(min > 0 && max.is_some() && max.unwrap() == 0)
    }
    fn num_options(&self, ctx: &StateContext) -> usize {
        self.menu_options(ctx).size_hint().0
    }
}

pub trait Editable {
    fn select_editable(&mut self, ctx: &mut StateContext, self_if: ContentProviderID) -> ContentManagerAction;
    fn num_editables(&self, ctx: &StateContext) -> usize {
        self.get_editables(ctx).size_hint().0
    }
    fn has_editables(&self) -> bool {
        // implimentation is super similar to Self::has_menu
    
        let (min, max) = self.get_editables(&Default::default()).size_hint();
        // an iterator has exactly 0 elements iff it has atleast 0 and atmost 0 elements
        !(min > 0 && max.is_some() && max.unwrap() == 0)
    }
    fn get_editables<'a>(&'a self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID> + 'a>;
}

pub trait SongProvider {
    fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a>;
    fn add_song(&mut self, id: SongID);
}

pub trait CPProvider {
    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a>;
    fn add_provider(&mut self, id: ContentProviderID);
}

pub trait Provider {
    fn get_name(&self) -> &str;
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex;
    fn get_selected_index(&self) -> &SelectedIndex;
}


#[macro_export]
macro_rules! _impliment_content_provider {
    ($t:ident, CPProvider) => {
        fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a> {
            CPProvider::providers(self)
        }
        fn add_provider(&mut self, id: ContentProviderID) {
            CPProvider::add_provider(self, id)
        }
    };
    ($t:ident, SongProvider) => {
        fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a> {
            SongProvider::songs(self)
        }
        fn add_song(&mut self, id: SongID) {
            SongProvider::add_song(self, id)
        }
    };
    ($t:ident, Editable) => {
        fn select_editable(&mut self, ctx: &mut StateContext, self_id: ContentProviderID) -> ContentManagerAction {
            Editable::select_editable(self, ctx, self_id)
        }
        fn num_editables(&self, ctx: &StateContext) -> usize {
            Editable::num_editables(self, ctx)
        }
        fn has_editables(&self) -> bool {
            Editable::has_editables(self)
        }
        fn get_editables<'a>(&'a self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID> + 'a> {
            Editable::get_editables(self, ctx)
        }        
    };
    ($t:ident, Menu) => {
        fn apply_option(&mut self, ctx: &mut StateContext, self_id: ContentProviderID) -> ContentManagerAction {
            Menu::apply_option(self, ctx, self_id)
        }
        fn menu_options(&self, ctx: &StateContext) -> Box<dyn Iterator<Item = FriendlyID>> {
            Menu::menu_options(self, ctx)
        }
        fn has_menu(&self) -> bool {
            Menu::has_menu(self)
        }
        fn num_options(&self, ctx: &StateContext) -> usize {
            Menu::num_options(self, ctx)
        }        
    };
    ($t:ident, Loadable) => {
        fn maybe_load(&mut self, self_id: ContentProviderID) -> ContentManagerAction {
            Loadable::maybe_load(self, self_id)
        }
        fn load(&mut self, self_id: ContentProviderID) -> ContentManagerAction {
            Loadable::load(self, self_id)
        }
        fn is_loaded(&self) -> bool {
            Loadable::is_loaded(self)
        }   
    };
    ($t:ident, Provider) => {
        fn get_name(&self) -> &str {
            Provider::get_name(self)
        }
        fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
            Provider::get_selected_index_mut(self)
        }
        fn get_selected_index(&self) -> &SelectedIndex {
            Provider::get_selected_index(self)
        }
    };
    // ($t:ident, CPProvider) => {
    //     impl ContentProvider for $t {

    //     }
    // };
    ($t:ident, $r:tt, $($e:tt), +) => {
        impliment_content_provider!($t, $r); // wtf, it does not recognise _impliment_content_provider
        $(
            impliment_content_provider!($t, $e);
        )+

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    };
}

// somehow forces the macro to be in this module.
// this seems like a hack, but goddamit imma use it
pub(crate) use _impliment_content_provider as impliment_content_provider;


