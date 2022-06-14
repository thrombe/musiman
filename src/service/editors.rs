
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
pub struct Yanker {
    yanked_items: Vec<ID>,
    content_type: YankedContentType,
    yanked_from: Option<ContentProviderID>,
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

    pub fn yank_song(&mut self, id: SongID, provider_id: ContentProviderID) {
        if Some(provider_id) != self.yanked_from || self.content_type != YankedContentType::Song {
            self.yanked_items.clear();
            self.content_type = YankedContentType::Song;
            self.yanked_from = Some(provider_id);
        }
        self.yanked_items.push(id.into());
    }

    pub fn yank_content_provider(&mut self, id: ContentProviderID, provider_id: ContentProviderID) {
        if Some(provider_id) != self.yanked_from || self.content_type != YankedContentType::ContentProvider {
            self.yanked_items.clear();
            self.content_type = YankedContentType::ContentProvider;
            self.yanked_from = Some(provider_id);
        }
        self.yanked_items.push(id.into());
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
    IndexChange{
        provider: ID,
        from: usize,
        to: usize,
    },
    TextEdit{ // also need to store info about what field changed
        content: ID,
        from: String,
        to: String,
    },
}

