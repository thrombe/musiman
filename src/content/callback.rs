
use std::{
    fmt::Debug,
};
use anyhow::Result;

use crate::{
    content::{
        handler::ContentHandler,
    },
};

pub mod wrapper {
    use crate::content::{
        action::ContentHandlerAction,
        handler::ContentHandler,
    };
    use anyhow::Result;

    #[derive(Debug)]
    pub struct ContentHandlerCallback(Box<dyn super::ContentHandlerCallback>);
    impl ContentHandlerCallback {
        pub fn new(callback: Box<dyn super::ContentHandlerCallback>) -> Self {
            Self(callback)
        }
        pub fn call(self, ch: &mut ContentHandler) -> Result<()> {
            self.0.call(ch)
        }
    }
    impl Into<ContentHandlerAction> for ContentHandlerCallback {
        fn into(self) -> ContentHandlerAction {
            ContentHandlerAction::Callback { callback: self }
        }
    }
    // impl Deref for ContentHandlerCallback {
    //     type Target = Box<dyn super::ContentHandlerCallback>;
    //     fn deref(&self) -> &Self::Target {
    //         &self.0
    //     }
    // }
}

// why something else like Actions?
// callbacks can be implimented by anyone (so it dosent pollute the ContentHandlerActions), and are generally
// made to do io stuff but not blocking in main thread
pub trait ContentHandlerCallback: Send + Sync + Debug {
    fn call(self: Box<Self>, ch: &mut ContentHandler) -> Result<()>;
}

impl<T> From<T> for wrapper::ContentHandlerCallback
    where T: ContentHandlerCallback + 'static
{
    fn from(callback: T) -> Self {
        wrapper::ContentHandlerCallback::new(Box::new(callback))
    }
}
