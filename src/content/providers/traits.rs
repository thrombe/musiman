
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
            GlobalContent,
        },
        manager::{
            action::ContentManagerAction,
            manager::ContentManager,
        },
        display::DisplayContext,
        providers::ContentProvider,
    },
    app::{
        app::SelectedIndex,
        display::{
            Display,
        },
    },
    service::editors::{
        YankAction,
        Yank,
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


impl<T> From<T> for ContentProvider
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
#[typetag::serde(tag = "type")]
pub trait ContentProviderTrait
    where
        Self: std::fmt::Debug + Send + Sync + CPClone + Any,
{
    fn as_song_provider(&self) -> Option<&dyn SongProvider> {None}
    fn as_song_provider_mut(&mut self) -> Option<&mut dyn SongProvider> {None}

    fn as_song_yank_dest_mut(&mut self) -> Option<&mut dyn SongYankDest> {None}

    fn as_provider_yank_dest_mut(&mut self) -> Option<&mut dyn CPYankDest> {None}

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
        if i.selected_index()+1 < num_items {
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
    fn ids<'a>(&'a self) -> Box<dyn Iterator<Item = ID> + 'a> { // ? maybe try Iter/IntoIter instead of return a boxed iterator? does it satisfy the requirements?
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

pub trait SongProvider: ContentProviderTrait {
    fn songs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a SongID> + 'a>;
    fn add_song(&mut self, id: SongID);
    fn songs_mut(&mut self) -> &mut Vec<SongID>; // any other option??

    fn remove_song(&mut self, id: SongID) -> Option<SongID> {
        let index = self.songs().position(|&i| i == id);
        match index {
            Some(index) => {
                if self.ids().position(|i| i == ID::from(id)).unwrap() <= self.get_selected_index().selected_index() {
                    self.selection_decrement();
                }
                Some(self.songs_mut().remove(index))
            }
            None => None,
        }
    }
}

pub trait CPProvider: ContentProviderTrait {
    fn providers<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContentProviderID> + 'a>;
    fn add_provider(&mut self, id: ContentProviderID);

    fn providers_mut(&mut self) -> &mut Vec<ContentProviderID>;

    fn remove_provider(&mut self, id: ContentProviderID) -> Option<ContentProviderID> {
        let index = self.providers().position(|&i| i == id);
        match index {
            Some(index) => {
                if self.ids().position(|i| i == ID::from(id)).unwrap() <= self.get_selected_index().selected_index() {
                    self.selection_decrement();
                }
                Some(self.providers_mut().remove(index))
            }
            None => None,
        }
    }
}

pub trait Provider {
    fn get_selected_index_mut(&mut self) -> &mut SelectedIndex;
    fn get_selected_index(&self) -> &SelectedIndex;
}

pub struct YankContext<'a>(&'a mut ContentManager);
impl<'a> YankContext<'a> {
    pub fn new(inner: &'a mut ContentManager) -> Self {YankContext(inner)}
    pub fn get_provider(&self, id: ContentProviderID) -> &ContentProvider {self.0.get_provider(id)}
    pub fn get_provider_mut(&mut self, id: ContentProviderID) -> &mut ContentProvider {self.0.get_provider_mut(id)}
    pub fn alloc_provider(&mut self, provider: ContentProvider) -> ContentProviderID {self.0.alloc_content_provider(provider)}
    pub fn register<T: Into<GlobalContent>>(&mut self, id: T) {self.0.register(id)}
}

/// the items might be copied and modified when pasted (only in try_* methods). some other things might trigger on paste too
/// the paste might even be rejected // TODO: how do i communicate the rejections back? is it even needed tho?
pub trait YankDest<T: PartialEq + Copy + Debug>: ContentProviderTrait {
    fn dest_vec_mut(&mut self) -> Option<&mut Vec<T>> {None} // for default implimentations of paste and insert, else they panic

    /// all items are pasted one after the other starting at the mentioned index, else appended at the last
    fn try_paste(&mut self, items: Vec<Yank<T>>, start_index: Option<usize>, self_id: ContentProviderID) -> YankAction;
    fn paste(&mut self, items: Vec<Yank<T>>, start_index: Option<usize>) {
        start_index
        .filter(|i| self.get_selected_index().selected_index() >= *i)
        .map(|_| (0..items.len()).for_each(|_| {self.selection_increment();}));
        
        let vecc = self.dest_vec_mut().unwrap();
        let len = vecc.len();
        items.into_iter()
        .enumerate()
        .map(|(i, y)| (start_index.map(|j| j+i).unwrap_or(len+i), y))
        .for_each(|(i, y)| vecc.insert(i, y.item));
    }

    /// each item will be at the associated index once the entire operation is done
    // fn try_insert(&mut self, items: Vec<(T, usize)>) -> ContentManagerAction;
    fn insert(&mut self, mut items: Vec<Yank<T>>) {
        let mut counter = 0;
        let selected_index = self.get_selected_index().selected_index();
        let vecc = self.dest_vec_mut().unwrap();
        items.sort_by(|a, b| a.index.cmp(&b.index));
        let mut items = items.into_iter().peekable();
        let mut old_items = std::mem::replace(vecc, vec![]).into_iter().peekable();
        while items.peek().is_some() || old_items.peek().is_some() {
            if let Some(&y) = items.peek() {
                if vecc.len() == y.index || old_items.peek().is_none() {
                    vecc.push(items.next().unwrap().item);
                    if y.index <= selected_index+counter {
                        counter += 1;
                    }
                    continue;
                }
            }
            vecc.push(old_items.next().unwrap());
        }
        (0..counter).for_each(|_| {self.selection_increment();});
    }

    /// assuming T exists at the provided index (necessary for multiple of the same thing present in the list)
    fn remove(&mut self, mut items: Vec<Yank<T>>) {
        let mut counter = 0;
        let selected_index = self.get_selected_index().selected_index();
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
                        counter += 1;
                    }
                    return None;
                }
            }
            Some(*id)
        })
        .collect();

        (0..counter).for_each(|_| {self.selection_decrement();});
        if items.peek().is_some() {
            dbg!(items.collect::<Vec<_>>());
            panic!("everything was not removed");
        }
    }
}

pub trait SongYankDest: YankDest<SongID> {}
impl<T: YankDest<SongID>> SongYankDest for T {}
pub trait CPYankDest: YankDest<ContentProviderID> {}
impl<T: YankDest<ContentProviderID>> CPYankDest for T {}

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
    ($t:ident, SongYankDest) => {
        fn as_song_yank_dest_mut(&mut self) -> Option<&mut dyn SongYankDest> {Some(self)}
    };
    ($t:ident, CPYankDest) => {
        fn as_provider_yank_dest_mut(&mut self) -> Option<&mut dyn CPYankDest> {Some(self)}
    };
    ($t:ident, $r:tt, $($e:tt), +) => {
        impliment_content_provider!($t, $r); // it does not recognise _impliment_content_provider as it is defined only in this module
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


