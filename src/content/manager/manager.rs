

#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};


use musiplayer::Player;
use anyhow::Result;
use tokio::sync::mpsc::{
    unbounded_channel,
    UnboundedReceiver,
    UnboundedSender,
};

use crate::{
    content::{
        providers::{
            ContentProvider,
            main_provider::MainProvider,
            queue_provider::QueueProvider,
            queue::Queue,
            traits::CPProvider,
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
    pub songs: ContentRegister<Song, SongID>,
    pub content_providers: ContentRegister<ContentProvider, ContentProviderID>,

    pub content_stack: ContentStack,
    pub edit_manager: EditManager,
    pub image_handler: ImageHandler,
    pub player: Player, // FIX: memory leak somewhere maybe. (the ram usage keeps increasing) // https://github.com/sdroege/gstreamer-rs/blob/main/examples/src/bin/play.rs
    notifier: Notifier,
    
    active_queue: Option<ContentProviderID>, // can also be a bunch of queues? like -> play all artists
    pub active_song: Option<SongID>,

    pub parallel_handle: ParallelHandle,

    pub app_action_sender: UnboundedSender<AppAction>, // just so it can be received in async
    pub app_action_receiver: UnboundedReceiver<AppAction>
}

// methods related to content register
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

    pub fn register<T: Into<GlobalContent>>(&mut self, id: T) {
        let id = id.into();
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
    
    pub fn unregister<T: Into<GlobalContent>>(&mut self, id: T) {
        let id = id.into();
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
                                if let Some(cp) = cp.as_song_provider() {
                                    for &s_id in cp.songs() {
                                        let _ = self.unregister(s_id);
                                    }
                                }
                                if let Some(cp) = cp.as_provider() {
                                    for &cp_id in cp.providers() {
                                        let _ = self.unregister(cp_id);
                                    }
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

// methods related to display
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
            yanker: self.edit_manager.yanker.as_ref(),
        })
    }
    fn display_song(&self, id: SongID, state: DisplayState) -> ListBuilder<'static> {
        todo!()
    }
}

// TODO: need to be very careful while saving state. all ids not in the db should be deallocated before saving, or bad bad (id_counter can be > 1 even if there is just 1 id to it in the db)
impl ContentManager {
    pub fn new() -> Result<Self> {
        let mut cr = ContentRegister::new();
        let main_id = {
            let cp = &mut cr;
            let mut ids = vec![];
            let main_provider = MainProvider::new(|item| cp.alloc(item), |id| ids.push(id));
            ids.into_iter().for_each(|id| cp.register(id));
            cp.alloc(main_provider.into())
        };
        let (sender, receiver) = unbounded_channel();
        let ch = Self {
            songs: ContentRegister::new(),
            content_providers: cr,
            content_stack: ContentStack::new(main_id),
            edit_manager: EditManager::new(),
            image_handler: Default::default(),
            player: Player::new()?,
            notifier: Notifier::new(),
            active_queue: None,
            active_song: None,
            parallel_handle: Default::default(),
            app_action_sender: sender,
            app_action_receiver: receiver,
        };
        Ok(ch)
    }

    pub fn try_load() -> Result<Option<Self>> {
        let cm = match DBHandler::try_load()? {
            Some(mut db) => {
                let mut mp = db.content_providers
                .get_mut(db.main_provider)
                .unwrap()
                .as_any_mut()
                .downcast_mut::<MainProvider>()
                .unwrap()
                .clone();
                {
                    let mut ids = vec![];
                    mp.load(|item| db.content_providers.alloc(item), |id| ids.push(id));
                    ids.into_iter().for_each(|id| db.content_providers.register(id));
                }
                *db.content_providers
                .get_mut(db.main_provider)
                .unwrap() = mp.into();
                Some(Self {
                    songs: db.songs,
                    content_providers: db.content_providers,
                    content_stack: ContentStack::new(db.main_provider),
                    edit_manager: db.edit_manager,

                    ..Self::new()?
                })
            }
            None => {
                None
            }
        };
        Ok(cm)
    }

    pub fn save(mut self) -> Result<()> {
        (0..self.content_stack.len()-1)
        .filter_map(|_| self.content_stack.pop())
        .collect::<Vec<_>>()
        .into_iter()
        .for_each(|id| self.unregister(id));

        self.active_song.map(|id| self.unregister(id));
        self.active_queue.map(|id| self.unregister(id));

        self.get_main_provider()
        .providers()
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .for_each(|id| self.unregister(id));
        self.get_main_provider_mut().providers_mut().clear();
        
        let mp = self.content_stack.main_provider();
        let songs = self.songs;
        let cps = self.content_providers;
        // self.edit_manager.yanker.take();
        let edit_manager = self.edit_manager;
        DBHandler {
            main_provider: mp,
            songs,
            content_providers: cps,
            edit_manager,
        }.save()?;
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
}

impl ContentManager {
    pub fn check_for_register(&self) {
        let mut song_keys = vec![];
        let mut provider_keys = vec![];

        fn cp_ids(ch: &ContentManager, id: ContentProviderID, unique: &mut Vec<ContentProviderID>) -> Vec<ID> {
            let mut ids = vec![];
            ids.push(ID::ContentProvider(id));
            if unique.contains(&id) {
                return ids;
            } else {
                unique.push(id);
            }
            let cp = ch.get_provider(id);
            ids.extend(cp.ids().map(|id| match id {
                ID::Song(id) => vec![ID::Song(id)].into_iter(),
                ID::ContentProvider(id) => cp_ids(ch, id, unique).into_iter(),
            })
            .flatten());
            ids
        }
        
        let mut unique_provider_ids = vec![];
        let len = self.content_stack.len();
        (0..len).map(|i| self.content_stack.get(i)) // content_stack.state ids do not count in register
        .map(|id| match id {
            GlobalProvider::Notifier => todo!(),
            GlobalProvider::ContentProvider(id) => cp_ids(self, id, &mut unique_provider_ids).into_iter(),
        })
        .flatten()
        .for_each(|id| match id {
            ID::Song(id) => {
                song_keys.push(id);
            }
            ID::ContentProvider(id) => {
                provider_keys.push(id);
            }
        });

        self.edit_manager
        .edit_stack
        .iter()
        .cloned()
        .chain(self.edit_manager.undo_stack.iter().cloned())
        .map(|e| match e {
            crate::service::editors::Edit::Pasted { yank, .. } => yank,
            crate::service::editors::Edit::Yanked { yank, .. } => yank,
            crate::service::editors::Edit::TextEdit { .. } => todo!(),
        })
        .map(|y| y.yanked_items.clone().into_iter())
        .flatten()
        .for_each(|(id, _)| match id {
            ID::Song(id) => {
                song_keys.push(id);
            }
            ID::ContentProvider(id) => {
                provider_keys.push(id);
            }
        });

        self.active_queue.map(|id| provider_keys.push(id));
        self.active_song.map(|id| song_keys.push(id));
        provider_keys.push(self.get_main_provider().queue_provider);

        // self.songs;
        // self.content_providers;
        // self.content_stack;
        // self.edit_manager;
        // self.active_queue;
        // self.active_song;
        // self.main_provider().queue_provider

        let key_frequencies = song_keys
        .into_iter()
        .map(ID::Song)
        .chain(provider_keys.into_iter().map(ID::ContentProvider))
        .fold(std::collections::HashMap::new(), |mut map, val|{
            map.entry(val)
               .and_modify(|frq|*frq+=1)
               .or_insert(1);
            map
        });

        let mut key_frequencies_2 = (0..self.songs.len())
        .filter_map(|i| self.songs.get_id_count(i))
        .map(|(id, i)| (ID::Song(id.into()), i))
        .chain(
            (0..self.content_providers.len())
            .filter_map(|i| self.content_providers.get_id_count(i))
            // .inspect(|i| dbg!(i)) // dbg:
            .map(|(id, i)| (ID::ContentProvider(id.into()), i))
        )
        .fold(std::collections::HashMap::new(), |mut map, val|{
            assert_eq!(map.insert(val.0, val.1), None);
            map
        });

        key_frequencies
        .into_iter()
        .for_each(|(id, i)| {
            if let Some(j) = key_frequencies_2.remove(&id) {
                if j != i {
                    error!("{} ids available in the wild than recorded. id: {id:#?}, ids found in wild: {i}, ids recorded in register: {j}", if j < i {"more"} else {"less"});
                }
            } else {
                error!("no item available for id: {id:#?}");
            }
        });
        key_frequencies_2.into_iter()
        .for_each(|(id, i)| {
            error!("leaked item. id: {id:#?}, id_count: {i}");
        });
    }

    pub fn debug_current(&self, c: char) {
        match c {
            'c' => {
                self.check_for_register();
                debug!("register checks complete");
            }
            'e' => {
                dbg!(&self.edit_manager);
            }
            's' => {
                dbg!(self.active_queue);
                dbg!(self.active_song);
                dbg!(&self.content_stack);
            }
            'p' => {
                dbg!(&self.content_providers);
            }
            'S' => {
                dbg!(&self.songs);
            }
            'P' => {
                dbg!(&self.player);
                // dbg!(self.player.is_finished());
                // dbg!(self.player.duration());
                // dbg!(self.player.position());
                // dbg!(self.player.progress());
            }
            'i' => {
                dbg!(&self.image_handler);
            }
            'y' => {
                debug!("{}", serde_yaml::to_string(&self.songs).unwrap());
                let a = serde_yaml::to_string(&self.content_providers).unwrap();
                debug!("{}", &a);
                let b = serde_yaml::from_str::<ContentRegister<ContentProvider, ContentProviderID>>(&a);
                dbg!(b);
            }
            _ => {}
        }
    }
}

// methods for interacting with ContentProvider
impl ContentManager {
    pub fn get_main_provider(&self) -> &MainProvider {
        self.get_raw_provider(self.content_stack.main_provider())
    }
    pub fn get_queue_provider(&self) -> &QueueProvider {
        self.get_raw_provider(self.get_main_provider().queue_provider)
    }
    pub fn get_main_provider_mut(&mut self) -> &mut MainProvider {
        self.get_raw_provider_mut(self.content_stack.main_provider())
    }
    pub fn get_queue_provider_mut(&mut self) -> &mut QueueProvider {
        self.get_raw_provider_mut(self.get_main_provider().queue_provider)
    }

    /// will panic if id does not point to correct provider type
    fn get_raw_provider<T: 'static>(&self, id: ContentProviderID) -> &T {
        self.content_providers
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref()
        .unwrap()
    }
    /// will panic if id does not point to correct provider type
    fn get_raw_provider_mut<T: 'static>(&mut self, id: ContentProviderID) -> &mut T {
        self.content_providers
        .get_mut(id)
        .unwrap()
        .as_any_mut()
        .downcast_mut()
        .unwrap()
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
                                self.set_queue(id, song_id);
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
        ContentManagerAction::RefreshDisplayContent.apply(self)?;
        Ok(())
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
        ContentManagerAction::RefreshDisplayContent.apply(self)?;
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
        ContentManagerAction::RefreshDisplayContent.apply(self)?;
        Ok(())
    }

    pub fn toggle_yank_selected(&mut self) -> Result<()> {
        let state = self.content_stack.get_state();
        match state {
            ContentState::Menu { .. } => (),
            ContentState::Edit { .. } => (),
            ContentState::GlobalMenu(_) => (),
            ContentState::Normal => {
                let id = self.content_stack.last();
                match id {
                    GlobalProvider::Notifier => (),
                    GlobalProvider::ContentProvider(id) => {
                        let cp = self.get_provider(id);
                        let selected_id = cp.get_selected();
                        let index = cp.get_selected_index().selected_index();
                        match self.edit_manager.yanker.as_mut() {
                            Some(y) => y.toggle_yank(selected_id, id, index),
                            None => {
                                let mut y = Yanker::new(id);
                                y.toggle_yank(selected_id, id, index);
                                self.edit_manager.yanker = Some(y);
                            }
                        }
                        ContentManagerAction::RefreshDisplayContent.apply(self)?;
                    },
                }
            }
        }
        Ok(())
    }
}

// methods related song to playback
impl ContentManager {
    pub fn set_queue(&mut self, id: ContentProviderID, song_id: SongID) {
        dbg!("maybe setting queue");
        let queue_index = self.get_queue_provider()
        .providers()
        .position(|q| *q == id);
        let converted_queue_index = self.get_queue_provider()
        .providers()
        .cloned()
        .map(|id| self.get_provider(id))
        .map(|cp| cp.as_any().downcast_ref::<Queue>())
        .filter_map(|cp| cp)
        .position(|q| q.source_cp == id);

        // if it already exists, do not make a new one
        if let Some(queue_index) = queue_index
        .map(|i| Some(i))
        .unwrap_or(converted_queue_index)
        {
            let q_id = self.get_queue_provider()
            .providers()
            .skip(queue_index)
            .next()
            .map(|id| *id)
            .unwrap();

            let q = self.get_raw_provider_mut::<Queue>(q_id);
            if let Some(song_index) = q.contains_song(song_id) {
                q.index.select(song_index);
                
                self.get_queue_provider_mut()
                .move_to_top(queue_index);

                self.register(q_id);
                self.active_queue.map(|id| self.unregister(id));
                self.active_queue = Some(q_id);
                dbg!(self.active_queue);

                return;
            }
            // if the song is not in it, then create a new queue
        }
        
        let mut q = {
            let mut register = vec![];
            let cp = self.get_provider(id);
            let q = Queue::new(cp, id, |id: SongID| register.push(id)); // can't pass allocator func as self is immutably borrowed
            register
            .into_iter()
            .for_each(|id| self.register(id));
            q
        };

        q.select_song(song_id);
        let q_id = self.alloc_content_provider(q.into());
        self.get_queue_provider_mut()
        .add_queue(q_id)
        .map(|id| self.unregister(id));

        self.register(q_id); // saved in both queue provider and in active_queue
        self.active_queue.map(|id| self.unregister(id));
        self.active_queue = Some(q_id);
        dbg!(self.active_queue);
    }

    pub fn play_song(&mut self, id: SongID) -> Result<()> {
        self.register(id);
        self.active_song.map(|id| self.unregister(id));
        self.active_song = Some(id);

        self.player.stop().unwrap();
        let song = self.get_song(id);
        let play_action = song.play()?;
        let art_action = song.show_art()?;
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
}

// methods related to managing selectioins
impl ContentManager {
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

