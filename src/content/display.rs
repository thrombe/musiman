
use crate::{
    content::{
        providers::FriendlyID,
        handler::ContentHandler,
        manager::{
            ID,
        },
    },
};

pub enum DisplayContent {
    Names(Vec<String>),
    IDs(Vec<ID>),
    FriendlyID(Vec<FriendlyID>)
}
impl<'a> From<Box<dyn Iterator<Item = FriendlyID> + 'a>> for DisplayContent {
    fn from(ids: Box<dyn Iterator<Item = FriendlyID> + 'a>) -> Self {
        Self::FriendlyID(ids.collect())
    }
}
impl DisplayContent {
    pub fn get(self, ch: &ContentHandler) -> Vec<String> {
        match self {
            Self::Names(names) => names,
            Self::IDs(ids) => {
                ids.iter().map(|&id| {
                    match id {
                        ID::Song(id) => {
                            ch.get_song(id).get_name()
                        }
                        ID::ContentProvider(id) => {
                            ch.get_provider(id).get_name()
                        }
                    }.to_owned()
                }).collect()
            }
            Self::FriendlyID(fids) => {
                fids
                .into_iter()
                .map(|fid| {
                    match fid {
                        FriendlyID::String(c) => c,
                        FriendlyID::ID(id) => {
                            match id {
                                ID::Song(id) => {
                                    ch.get_song(id).get_name()
                                }
                                ID::ContentProvider(id) => {
                                    &ch.get_provider(id).get_name()
                                }
                            }.to_owned()
                        }
                    }
                })
                .collect()
            }
        }
    }
}
