
use std::{
    fmt::Debug,
};
use anyhow::Result;

use crate::{
    content::{
        manager::ContentManager,
    },
};

pub mod wrapper {
    use crate::content::{
        action::ContentManagerAction,
        manager::ContentManager,
    };
    use anyhow::Result;

    #[derive(Debug)]
    pub struct ContentManagerCallback(Box<dyn super::ContentManagerCallback>);
    impl ContentManagerCallback {
        pub fn new(callback: Box<dyn super::ContentManagerCallback>) -> Self {
            Self(callback)
        }
        pub fn call(self, ch: &mut ContentManager) -> Result<()> {
            self.0.call(ch)
        }
    }
    impl Into<ContentManagerAction> for ContentManagerCallback {
        fn into(self) -> ContentManagerAction {
            ContentManagerAction::Callback { callback: self }
        }
    }
    // impl Deref for ContentManagerCallback {
    //     type Target = Box<dyn super::ContentManagerCallback>;
    //     fn deref(&self) -> &Self::Target {
    //         &self.0
    //     }
    // }
}

// why something else like Actions?
// callbacks can be implimented by anyone (so it dosent pollute the ContentManagerActions), and are generally
// made to do io stuff but not blocking in main thread
pub trait ContentManagerCallback: Send + Sync + Debug {
    fn call(self: Box<Self>, ch: &mut ContentManager) -> Result<()>;
}

impl<T> From<T> for wrapper::ContentManagerCallback
    where T: ContentManagerCallback + 'static
{
    fn from(callback: T) -> Self {
        wrapper::ContentManagerCallback::new(Box::new(callback))
    }
}
