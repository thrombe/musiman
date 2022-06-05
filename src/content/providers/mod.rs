
mod traits;
pub mod main_provider;
pub mod file_explorer;
pub mod yt_explorer;
pub mod ytalbum;


use crate::content::manager::ID;


#[derive(Debug, Clone)]
// pub struct ContentProvider(Box<dyn content_providers::ContentProvider<MenuOption = dyn HumanReadable>>);
pub struct ContentProvider(Box<dyn traits::ContentProvider>);
// pub type ContentProvider = Box<dyn content_providers::ContentProvider>;
impl std::ops::Deref for ContentProvider {
    type Target = Box<dyn traits::ContentProvider>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ContentProvider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<Box<dyn traits::ContentProvider>> for ContentProvider {
    fn from(o: Box<dyn traits::ContentProvider>) -> Self {
        Self(o)
    }
}
impl ContentProvider {
    pub fn new(t: Box<dyn traits::ContentProvider>) -> Self {
        Self(t)
    }
}





// pub struct Provider(Box<dyn ContentProvider>);

pub enum FriendlyID {
    String(String),
    ID(ID),
}

pub trait HumanReadable {
    
}
