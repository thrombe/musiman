

#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};


use musiplayer::Player;
use anyhow::Result;

use crate::{
    content::{
        providers::{
            ContentProvider,
            main_provider::MainProvider,
        },
        register::{
            ContentRegister,
            ContentProviderID,
            SongID,
            ID,
            GlobalContent,
            GlobalProvider,
        },
        song::Song,
        stack::ContentStack,
        manager::action::{
            ParallelHandle,
            ContentManagerAction,
        },
        stack::ContentState,
        display::{
            DisplayContext,
            DisplayState,
        },
    },
    app::{
        action::AppAction,
        app::SelectedIndex,
        display::ListBuilder,
    },
    service::{
        db::DBHandler,
        editors::{
            Yanker,
            EditManager,
        },
        notifier::Notifier,
    },
    image::ImageHandler,
};

pub struct ContentManager {
    songs: ContentRegister<Song, SongID>,
    content_providers: ContentRegister<ContentProvider, ContentProviderID>,
    db_handler: DBHandler,

    pub content_stack: ContentStack,
    yanker: Yanker,
    edit_manager: EditManager,
    pub image_handler: ImageHandler,
    pub player: Player, // FIX: memory leak somewhere maybe. (the ram usage keeps increasing) // https://github.com/sdroege/gstreamer-rs/blob/main/examples/src/bin/play.rs
    notifier: Notifier,
    
    active_queue: Option<ContentProviderID>, // can also be a bunch of queues? like -> play all artists
    pub active_song: Option<SongID>,

    pub parallel_handle: ParallelHandle,
    pub app_action: AppAction,
}


impl ContentManager {
    pub fn alloc_song(&mut self, s: Song) -> SongID {
        self.songs.alloc(s)
    }

    pub fn alloc_content_provider(&mut self, cp: ContentProvider) -> ContentProviderID {
        self.content_providers.alloc(cp)
    }

    /// clones the provider and registers everything in it
    pub fn clone_content_provider(&mut self, id: ContentProviderID) -> ContentProviderID {
        let cp = self.get_provider(id).cp_clone();
        cp.ids().for_each(|id| {
            match id {
                ID::Song(id) => {
                    self.songs.register(id);
                }
                ID::ContentProvider(id) => {
                    self.content_providers.register(id);
                }
            }
        });
        self.alloc_content_provider(cp.into())
    }

    pub fn register(&mut self, id: GlobalContent) {
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        self.songs.register(id);
                    }
                    ID::ContentProvider(id) => {
                        self.content_providers.register(id);
                    }
                }
            }
            GlobalContent::Notifier => (),
        }
    }
    
    pub fn unregister(&mut self, id: GlobalContent) {
        dbg!("unregister id", id);
        match id {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        let _ = self.songs.unregister(id);
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.content_providers.unregister(id);
                        match cp {
                            Some(cp) => {
                                for &s_id in cp.as_song_provider().unwrap().songs() {
                                    let _ = self.unregister(s_id.into());
                                }
                                for &cp_id in cp.as_provider().unwrap().providers() {
                                    let _ = self.unregister(cp_id.into());
                                }
                            }
                            None => (),
                        }
                    }
                }
            }
            GlobalContent::Notifier => (),
        }
    }
}

impl ContentManager {
    pub fn display(&self) -> ListBuilder<'static> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                match id {
                    GlobalProvider::ContentProvider(id) => {
                        self.display_provider(id, DisplayState::Normal)
                    }
                    GlobalProvider::Notifier => todo!(),
                }
            }
            ContentState::Menu { ctx, id } => {
                match *id {
                    GlobalContent::Notifier => todo!(),
                    GlobalContent::ID(id) => {
                        match id {
                            ID::Song(id) => {
                                self.display_song(id, DisplayState::Menu(ctx))
                            }
                            ID::ContentProvider(id) => {
                                self.display_provider(id, DisplayState::Menu(ctx))
                            }
                        }
                    }
                }
            }
            ContentState::Edit { ctx, id } => {
                match *id {
                    GlobalContent::Notifier => todo!(),
                    GlobalContent::ID(id) => {
                        match id {
                            ID::Song(id) => {
                                self.display_song(id, DisplayState::Edit(ctx))
                            }
                            ID::ContentProvider(id) => {
                                self.display_provider(id, DisplayState::Edit(ctx))
                            }
                        }
                    }
                }
            }
            ContentState::GlobalMenu(_) => todo!(),
        }
    }

    fn display_provider<'b>(&self, id: ContentProviderID, state: DisplayState<'b>) -> ListBuilder<'static> {
        let cp = self.get_provider(id);
        cp.as_display().display(DisplayContext {
            state,
            songs: &self.songs,
            providers: &self.content_providers,
        })
    }
    fn display_song(&self, id: SongID, state: DisplayState) -> ListBuilder<'static> {
        todo!()
    }
}

impl ContentManager {
    pub fn new() -> Result<Self> {
        let dbh = DBHandler::try_load();
        let mut cp = ContentRegister::new();
        let main_id = cp.alloc(MainProvider::default().into());
        let ch = Self {
            songs: ContentRegister::new(),
            content_providers: cp,
            db_handler: dbh,
            content_stack: ContentStack::new(main_id),
            yanker: Yanker::new(),
            edit_manager: EditManager::new(),
            image_handler: Default::default(),
            player: Player::new()?,
            notifier: Notifier::new(),
            active_queue: None,
            active_song: None,
            parallel_handle: Default::default(),
            app_action: Default::default(),
        };
        Ok(ch)
    }

    // TODO: temporary implimentation
    pub fn load() -> Result<Self> {
        Self::new()
    }

    pub fn debug_current(&mut self) {
        // dbg!(&self.content_providers);
        dbg!(&self.content_stack);
        dbg!(&self.player);
        // dbg!(self.player.is_finished());
        // dbg!(self.player.duration());
        // dbg!(self.player.position());
        // dbg!(self.player.progress());
        dbg!(&self.image_handler);
        if self.active_song.is_some() {
            let s = self.get_song(self.active_song.unwrap()).as_display();
            dbg!(s.title(), s.artist(), s.album());
        }
        let id = self.content_stack.last();
        match id {
            GlobalProvider::ContentProvider(id) => {
                let cp = self.get_provider(id);
                dbg!(cp);
            }
            _ => (),
        }
    }

    pub fn poll_action(&mut self) -> Result<()> {
        self.parallel_handle.poll().apply(self)
    }

    pub fn enter_selected(&mut self) -> Result<()> {
        let state = self.content_stack.get_state_mut();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                match id {
                    GlobalProvider::ContentProvider(id) => {
                        let cp = self.get_provider_mut(id);
                        let content_id = cp.get_selected();
                        match content_id {
                            ID::Song(song_id) => {
                                self.play_song(song_id)?;
                                self.set_queue(id);
                            }
                            ID::ContentProvider(id) => {
                                ContentManagerAction::PushToContentStack { id: id.into() }.apply(self)?;
                            }
                        }
                    }
                    GlobalProvider::Notifier => {
                        // do nothing?
                    }
                }
            }
            ContentState::Menu {ctx, id} => {
                match *id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::ContentProvider(id) => {
                                let cp = self.content_providers.get_mut(id).unwrap();
                                let action = cp.as_menu_mut().unwrap().apply_option(ctx, id);
                                action.apply(self)?;
                            }
                            ID::Song(id) => {
                                todo!()
                            }
                        }
                    }
                    GlobalContent::Notifier => {
                        todo!()
                    }
                }
            }
            ContentState::Edit {ctx, id} => {
                match *id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::ContentProvider(id) => {
                                let cp = self.content_providers.get_mut(id).unwrap();
                                let action = cp.as_editable_mut().unwrap().select_editable(ctx, id);
                                action.apply(self)?;
                            }
                            ID::Song(id) => {
                                todo!()
                            }
                        }
                    }
                    GlobalContent::Notifier => {
                        todo!()
                    }
                }
            }
            ContentState::GlobalMenu(i) => {
                todo!()
            }
        }
        self.app_action.queue(AppAction::UpdateDisplayContent);
        Ok(())
    }

    pub fn get_selected_index(&mut self) -> &mut SelectedIndex {
        let id = self.content_stack.last();
        let state = self.content_stack.get_state_mut();
        match state {
            ContentState::Normal => {
                match id {
                    GlobalProvider::ContentProvider(id) => {
                        let cp = self.content_providers.get_mut(id).unwrap();
                        cp.get_selected_index_mut()
                    }
                    GlobalProvider::Notifier => {
                        todo!()
                    }
                }
            }
            ContentState::Edit {ctx, ..} => {
                ctx.last_mut()
            }
            ContentState::Menu {ctx, ..} => {
                ctx.last_mut()
            }
            ContentState::GlobalMenu(i) => {
                i
            }
        }
    }

    pub fn open_menu_for_current(&mut self) -> Result<()> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                self.open_menu_for(self.content_stack.last())?;
            }
            ContentState::Edit { .. } | ContentState::Menu { .. } | ContentState::GlobalMenu(..) => {
                // do nothing
            }
        }
        Ok(())
    }
    
    pub fn open_menu_for_selected(&mut self) -> Result<()> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                match id {
                    GlobalProvider::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        let id = cp.get_selected();
                        self.open_menu_for(id)?;
                    }
                    GlobalProvider::Notifier => {
                        // do nothing
                    }
                }
            },
            ContentState::Menu { .. } | ContentState::Edit { .. } | ContentState::GlobalMenu(..) => {
                // do nothing
            }
        }
        Ok(())
    }

    fn open_menu_for<T: Into<GlobalContent>>(&mut self, id: T) -> Result<()> {
        match id.into() {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        let s = self.get_song(id);
                        todo!()
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        if cp.as_menu().is_some() {
                            self.content_stack.open_menu(id);
                        }
                    }
                }
            }
            GlobalContent::Notifier => {
                todo!()
            }
        }
        self.app_action.queue(AppAction::UpdateDisplayContent);
        Ok(())
    }    

    pub fn open_edit_for_current(&mut self) -> Result<()> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                self.open_edit_for(self.content_stack.last())?;
            }
            ContentState::Edit { .. } | ContentState::Menu { .. } | ContentState::GlobalMenu(..) => {
                // do nothing
            }
        }
        Ok(())
    }    

    pub fn open_edit_for_selected(&mut self) -> Result<()> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                match id {
                    GlobalProvider::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        let id = cp.get_selected();
                        self.open_edit_for(id)?;
                    }
                    GlobalProvider::Notifier => {
                        // do nothing
                    }
                }
            },
            ContentState::Menu { .. } | ContentState::Edit { .. } | ContentState::GlobalMenu(..) => {
                // do nothing
            }
        }
        Ok(())
    }

    pub fn open_edit_for<T: Into<GlobalContent>>(&mut self, id: T) -> Result<()> {
        match id.into() {
            GlobalContent::ID(id) => {
                match id {
                    ID::Song(id) => {
                        todo!()
                    }
                    ID::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        if cp.as_editable().is_some() {
                            self.content_stack.open_edit(id);
                        }
                    }
                }
            }
            GlobalContent::Notifier => {
                todo!()
            }
        }
        self.app_action.queue(AppAction::UpdateDisplayContent);
        Ok(())
    }    

    pub fn get_provider(&self, id: ContentProviderID) -> &ContentProvider {
        self.content_providers.get(id).unwrap()
    }

    pub fn get_provider_mut(&mut self, id: ContentProviderID) -> &mut ContentProvider {
        self.content_providers.get_mut(id).unwrap()
    }

    pub fn get_song(&self, id: SongID) -> &Song {
        self.songs.get(id).unwrap()
    }
    pub fn get_song_mut(&mut self, id: SongID) -> &mut Song {
        self.songs.get_mut(id).unwrap()
    }

    pub fn set_queue(&mut self, id: ContentProviderID) {
        self.active_queue = Some(id);
        let mp_id = self.content_stack.main_provider();

        // FIX: find queue provider. think of a better soloution
        let mp = self.get_provider(mp_id);
        // for &cp_id in mp.providers().collect::<Vec<_>>() {
            // let cp = self.get_provider_mut(cp_id);
            // if cp.cp_type == ContentProviderType::Queues {
            //     cp.add(id.into());
            // }
        // }
    }
    pub fn play_song(&mut self, id: SongID) -> Result<()> {
        self.active_song.map(|id| self.unregister(id.into()));
        self.register(id.into());

        self.player.stop().unwrap();
        let song = self.get_song(id);
        let play_action = song.play()?;
        let art_action = song.show_art()?;
        self.active_song = Some(id);
        play_action.apply(self)?;
        art_action.apply(self)?;
        Ok(())
    }
    pub fn toggle_song_pause(&mut self) {
        self.player.toggle_pause().unwrap();
    }
    pub fn next_song(&mut self) -> Result<()> { // FIX: browsing around changes the next song instead of choosing the song next to the current song
        let id = match self.active_queue {
            Some(id) => id,
            None => return Ok(()),
        };
        if !self.increment_selection_on(id) {
            return Ok(())
        }        
        let q = self.get_provider_mut(id);
        let song_id = q.get_selected();
        if let ID::Song(id) = song_id {
            self.play_song(id)?;
        }
        Ok(())
    }
    pub fn prev_song(&mut self) -> Result<()> {
        let id = match self.active_queue {
            Some(id) => id,
            None => return Ok(()),
        };
        if !self.decrement_selection_on(id) {
            return Ok(())
        }        
        let q = self.get_provider_mut(id);
        let song_id = q.get_selected();
        if let ID::Song(id) = song_id {
            self.play_song(id)?;
        }
        Ok(())
    }
    pub fn seek_song(&mut self, t: f64) -> Result<()> {
        self.player.seek(t)
    }

    pub fn get_app_action(&mut self) -> AppAction {
        std::mem::replace(&mut self.app_action, Default::default())
    }

    pub fn increment_selection(&mut self) { // TODO: maybe just pass the max limit of index to the content_stack when edit/menu instead of asking the providers (what if it changes tho)
        let state = self.content_stack.get_state_mut();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                self.increment_selection_on(id);
            }
            ContentState::Edit { ctx, id } => {
                let num_items = match *id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::Song(id) => {
                                todo!()
                            }
                            ID::ContentProvider(id) => {
                                let cp = self.content_providers.get_mut(id).unwrap();
                                cp.as_editable().unwrap().num_editables(ctx)
                            }
                        }
                    }
                    GlobalContent::Notifier => todo!(),
                };
                let i = ctx.last_mut();
                if i.selected_index()+1 < num_items {
                    let index = i.selected_index();
                    i.select(index + 1);
                }
            }
            ContentState::Menu { ctx, id } => {
                let num_items = match *id {
                    GlobalContent::ID(id) => {
                        match id {
                            ID::Song(id) => {
                                todo!()
                            }
                            ID::ContentProvider(id) => {
                                let cp = self.content_providers.get(id).unwrap();
                                cp.as_menu().unwrap().num_options(ctx)
                            }
                        }
                    }
                    GlobalContent::Notifier => todo!(),
                };
                let i = ctx.last_mut();
                if i.selected_index()+1 < num_items {
                    let index = i.selected_index();
                    i.select(index + 1);
                }
            }
            ContentState::GlobalMenu(i) => {
                todo!()
            }
        }
    }
    pub fn decrement_selection(&mut self) {
        let state = self.content_stack.get_state_mut();
        match state {
            ContentState::Normal => {
                let id = self.content_stack.last();
                self.decrement_selection_on(id);
            }
            ContentState::Edit { ctx, .. } => {
                let i = ctx.last_mut();
                if i.selected_index() > 0 {
                    let index = i.selected_index();
                    i.select(index - 1);
                }
            }
            ContentState::Menu { ctx, .. } => {
                let i = ctx.last_mut();
                if i.selected_index() > 0 {
                    let index = i.selected_index();
                    i.select(index - 1);
                }
            }
            ContentState::GlobalMenu(i) => {
                todo!()
            }
        }
    }

    fn increment_selection_on<T: Into<GlobalProvider>>(&mut self, id: T) -> bool {
        match id.into() {
            GlobalProvider::ContentProvider(id) => {
                let cp = self.get_provider_mut(id);
                cp.selection_increment()
            }
            GlobalProvider::Notifier => todo!(),
        }
    }
    fn decrement_selection_on<T: Into<GlobalProvider>>(&mut self, id: T) -> bool {
        match id.into() {
            GlobalProvider::ContentProvider(id) => {
                let cp = self.get_provider_mut(id);
                cp.selection_decrement()
            }
            GlobalProvider::Notifier => todo!(),
        }
    }
}

