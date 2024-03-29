
pub mod traits;
pub mod main_provider;
pub mod file_explorer;
pub mod yt_explorer;
pub mod ytalbum;
pub mod ytplaylist;
pub mod queue_provider;
pub mod queue;

use serde::{Serialize, Deserialize};

/// don't impliment clone on this. instead use ContentManager.clone_content_provider
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentProvider(Box<dyn traits::ContentProviderTrait>);
impl std::ops::Deref for ContentProvider {
    type Target = Box<dyn traits::ContentProviderTrait>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ContentProvider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<Box<dyn traits::ContentProviderTrait>> for ContentProvider {
    fn from(o: Box<dyn traits::ContentProviderTrait>) -> Self {
        Self(o)
    }
}
impl ContentProvider {
    pub fn new(t: Box<dyn traits::ContentProviderTrait>) -> Self {
        Self(t)
    }
}
