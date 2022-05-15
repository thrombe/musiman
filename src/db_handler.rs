
use crate::{
    content_handler::{ContentManager},
    content_providers::{ContentProvider},
};


pub struct DBHandler {

}

// TODO: temporary implimentation
impl DBHandler {
    pub fn try_load() -> Self {
        Self {}
    }

    pub fn song_providers(&mut self) -> ContentManager<ContentProvider> {
        ContentManager::new()
    }
}