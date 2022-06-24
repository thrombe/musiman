

use crate::{
    content::{
        register::{
            ContentProviderID,
            SongID,
            ContentRegister,
        },
        song::Song,
        providers::ContentProvider,
    },
};

use super::stack::StateContext;


pub struct DisplayContext<'a> {
    pub state: DisplayState<'a>,
    pub songs: &'a ContentRegister<Song, SongID>,
    pub providers: &'a ContentRegister<ContentProvider, ContentProviderID>
}


// BAD: this again introduces the problem that a state with edit can be passed to a provider without edit
pub enum DisplayState<'a> {
    Normal,
    Menu(&'a StateContext),
    Edit(&'a StateContext),
}