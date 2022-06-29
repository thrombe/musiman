
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
use anyhow::Result;

use crate::{
    content::{
        stack::StateContext,
        register::{
            ContentProviderID,
            SongID,
            ID,
        },
        manager::action::ContentManagerAction,
        display::DisplayContext,
    },
    app::{
        app::SelectedIndex,
        display::{
            Display,
        },
    },
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
    fn as_song_provider(&self) -> Option<&dyn SongProvider> {None}
    fn as_song_provider_mut(&mut self) -> Option<&mut dyn SongProvider> {None}
    

    fn as_provider(&self) -> Option<&dyn CPProvider> {None}
    fn as_provider_mut(&mut self) -> Option<&mut dyn CPProvider> {None}


    fn as_menu(&self) -> Option<&dyn Menu> {None}
    fn as_menu_mut(&mut self) -> Option<&mut dyn Menu> {None}


    fn as_loadable(&mut self) -> Option<&mut dyn Loadable> {None}

    
    fn as_editable(&self) -> Option<&dyn Editable> {None}
    fn as_editable_mut(&mut self) -> Option<&mut dyn Editable> {None}


    // every content provider has to impliment these
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
            self.as_provider()
            .map(|p| p.providers())
            .unwrap_or(Box::new([].into_iter()))
            .map(Clone::clone)
            .map(Into::into)
            .chain(
                self.as_song_provider()
                .map(|s| s.songs())
                .unwrap_or(Box::new([].into_iter()))
                .map(Clone::clone)
                .map(Into::into)
            )
        )
    }
    fn get_size(&self) -> usize {
        self.ids().count()
    }
    
    // for downcasting (the macro has implimentation for this)
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    // to have to not replicate all methods from display here
    fn as_display(&self) -> &dyn Display<DisplayContext = DisplayContext>;
}


pub trait Loadable {
    fn maybe_load(&mut self, self_id: ContentProviderID) -> Result<ContentManagerAction> {
        if self.is_loaded() {
            Ok(ContentManagerAction::None)
        } else {
            self.load(self_id)
        }
    }
    fn load(&mut self, self_id: ContentProviderID) -> Result<ContentManagerAction>;
    fn is_loaded(&self) -> bool;
}

pub trait Menu {
    fn apply_option(&mut self, ctx: &mut StateContext, self_id: ContentProviderID) -> ContentManagerAction;
    fn num_options(&self, ctx: &StateContext) -> usize;
}

pub trait Editable {
    fn select_editable(&mut self, ctx: &mut StateContext, self_id: ContentProviderID) -> ContentManagerAction;
    fn num_editables(&self, ctx: &StateContext) -> usize;
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
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex;
    fn get_selected_index(&self) -> &SelectedIndex;
}


#[macro_export]
macro_rules! _impliment_content_provider {
    ($t:ident, CPProvider) => {
        fn as_provider(&self) -> Option<&dyn CPProvider> {Some(self)}
        fn as_provider_mut(&mut self) -> Option<&mut dyn CPProvider> {Some(self)}
    };
    ($t:ident, SongProvider) => {
        fn as_song_provider(&self) -> Option<&dyn SongProvider> {Some(self)}
        fn as_song_provider_mut(&mut self) -> Option<&mut dyn SongProvider> {Some(self)}
    };
    ($t:ident, Editable) => {
        fn as_editable(&self) -> Option<&dyn Editable> {Some(self)}
        fn as_editable_mut(&mut self) -> Option<&mut dyn Editable> {Some(self)}
    };
    ($t:ident, Menu) => {
        fn as_menu(&self) -> Option<&dyn Menu> {Some(self)}
        fn as_menu_mut(&mut self) -> Option<&mut dyn Menu> {Some(self)}
    };
    ($t:ident, Loadable) => {
        fn as_loadable(&mut self) -> Option<&mut dyn Loadable> {Some(self)}
    };
    ($t:ident, Provider) => {
        fn get_selected_index_mut(&mut self) -> &mut SelectedIndex {
            Provider::get_selected_index_mut(self)
        }
        fn get_selected_index(&self) -> &SelectedIndex {
            Provider::get_selected_index(self)
        }
    };
    ($t:ident, Display) => {
        fn as_display(&self) -> &dyn Display<DisplayContext = DisplayContext> {self}
    };
    ($t:ident, $r:tt, $($e:tt), +) => {
        impliment_content_provider!($t, $r); // wtf, it does not recognise _impliment_content_provider
        $(
            impliment_content_provider!($t, $e);
        )+

        fn as_any(&self) -> &dyn std::any::Any {self}
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {self}
    };
}

// somehow forces the macro to be in this module.
// this seems like a hack, but goddamit imma use it
pub(crate) use _impliment_content_provider as impliment_content_provider;


