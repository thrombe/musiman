
mod traits;
pub mod main_provider;
pub mod file_explorer;
pub mod yt_explorer;
pub mod ytalbum;


use crate::content::manager::ID;


/// don't impliment clone on this. instead use ContentHnadler.clone_content_provider
#[derive(Debug)]
pub struct ContentProvider(Box<dyn traits::ContentProvider>);
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
