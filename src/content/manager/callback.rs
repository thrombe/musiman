
use std::{
    fmt::Debug,
};
use anyhow::Result;

use crate::{
    content::{
        manager::{
            manager::ContentManager,
            action::{
                ContentManagerAction,
            },
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


// why something else like Actions?
// callbacks can be implimented by anyone (so it dosent pollute the ContentManagerActions), and are generally
// made to do io stuff but not blocking in main thread
// maybe a provider calls some actions and it needs to save something in itself, then a implimentation of
// this can pass in the provider as arg in the callback func
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


