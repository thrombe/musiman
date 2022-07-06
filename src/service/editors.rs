
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use tui::{
    text::Span,
    style::{
        Style,
        Color,
    },
};
use std::borrow::Cow;

use crate::{
    content::register::{
        ID,
        SongID,
        ContentProviderID,
    },
};

#[derive(Clone, PartialEq, Eq)]
pub enum YankedContentType {
    Song,
    ContentProvider,
    None,
}

#[derive(Clone)]
pub struct Yanker { // all ids here are weak (but not enforced to be weak)
    pub yanked_items: Vec<ID>,
    content_type: YankedContentType,
    yanked_from: Option<ContentProviderID>, // not allowed to yank stuff from multiple places
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YankType {
    Copy,
    Cut,
}

impl Yanker {
    pub fn new() -> Self {
        Self {
            yanked_items: vec![],
            yanked_from: None,
            content_type: YankedContentType::None,
        }
    }

    fn yank_song(&mut self, id: SongID, provider_id: ContentProviderID) {
        if Some(provider_id) != self.yanked_from || self.content_type != YankedContentType::Song {
            self.yanked_items.clear();
            self.content_type = YankedContentType::Song;
            self.yanked_from = Some(provider_id);
        }
        self.yanked_items.push(id.into());
    }

    fn yank_content_provider(&mut self, id: ContentProviderID, provider_id: ContentProviderID) {
        if Some(provider_id) != self.yanked_from || self.content_type != YankedContentType::ContentProvider {
            self.yanked_items.clear();
            self.content_type = YankedContentType::ContentProvider;
            self.yanked_from = Some(provider_id);
        }
        self.yanked_items.push(id.into());
    }

    pub fn marker_symbol() -> Span<'static> {
        Span { content: Cow::Borrowed("█"), style: Style::default().fg(Color::Green) }
    }

    pub fn toggle_yank(&mut self, id: ID, provider_id: ContentProviderID) {
        if Some(provider_id) == self.yanked_from {
            let old_len = self.yanked_items.len();
            self.yanked_items.retain(|y_id| id != *y_id);
            let new_len = self.yanked_items.len();
            if old_len > new_len {
                return;
            }
        }
        match id {
            ID::Song(id) => self.yank_song(id, provider_id),
            ID::ContentProvider(id) => self.yank_content_provider(id, provider_id),
        }
    }
}

pub struct EditManager {
    edits: Vec<Edit>,
}

impl EditManager {
    pub fn new() -> Self {
        Self {
            edits: vec![],
        }
    }
}

pub enum Edit {
    Yanked {
        yanked_items: Vec<ID>,
        content_type: YankedContentType,
        yank_type: YankType,
        yanked_from: ContentProviderID,
        yanked_to: ContentProviderID,
    },
    IndexChange {
        provider: ID,
        from: usize,
        to: usize,
    },
    TextEdit { // also need to store info about what field changed
        content: ID,
        from: String,
        to: String,
    },
}

