
use crate::{
    content_handler::{MainProvider, ContentManager},
    content_providers::{SPProvider, SongProvider},
};


pub struct DBHandler {

}

// TODO: temporary implimentation
impl DBHandler {
    pub fn try_load() -> Self {
        Self {}
    }

    pub fn main_provider(&mut self) -> MainProvider {
        MainProvider::new()
    }

    pub fn song_providers(&mut self) -> ContentManager<SongProvider> {
        ContentManager::new()
    }

    pub fn sp_providers(&mut self) -> ContentManager<SPProvider> {
        ContentManager::new()
    }
}