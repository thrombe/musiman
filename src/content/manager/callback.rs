
use std::{
    fmt::Debug,
};
use anyhow::Result;

use crate::{
    content::{
        manager::{
            manager::ContentManager,
            action::ContentManagerAction,
        },
    },
};


#[derive(Debug)]
pub struct ContentManagerCallback(Box<dyn ContentManagerCallbackTrait>);
impl ContentManagerCallback {
    pub fn new(callback: Box<dyn ContentManagerCallbackTrait>) -> Self {
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


// why something else like Actions?
// callbacks can be implimented by anyone (so it dosent pollute the ContentManagerActions), and are generally
// made to do io stuff but not blocking in main thread
pub trait ContentManagerCallbackTrait: Send + Sync + Debug {
    fn call(self: Box<Self>, ch: &mut ContentManager) -> Result<()>;
}

impl<T> From<T> for ContentManagerCallback
    where T: ContentManagerCallbackTrait + 'static
{
    fn from(callback: T) -> Self {
        ContentManagerCallback::new(Box::new(callback))
    }
}
