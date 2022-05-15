
use crate::{
    content_handler::{ContentID, ID, ContentType, ContentProviderID, SongID}, content_providers::ContentProvider
};

#[derive(Clone, PartialEq, Eq)]
enum YankedContentType {
    Song,
    ContentProvider,
    None,
}

#[derive(Clone)]
pub struct Yanker {
    yanked_items: Vec<ID>,
    content_type: YankedContentType,
    yank_type: YankType,
    yanked_from: Option<ContentProviderID>,
    yanked_to: Option<ContentProviderID>, // for undo
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YankType {
    None, // ?? why this
    Copy,
    Cut,
}

impl Yanker {
    pub fn new() -> Self {
        Self {
            yanked_items: vec![],
            yank_type: YankType::None,
            yanked_from: None,
            yanked_to: None,
            content_type: YankedContentType::None,
        }
    }

    pub fn yank_song(&mut self, id: SongID) {
        if self.content_type != YankedContentType::Song {
            self.yanked_items.clear();
            self.content_type = YankedContentType::Song;
        }
        self.yanked_items.push(id.into());
    }

    pub fn yank_content_provider(&mut self, id: ContentProviderID) {
        if self.content_type != YankedContentType::ContentProvider {
            self.yanked_items.clear();
            self.content_type = YankedContentType::ContentProvider;
        }
        self.yanked_items.push(id.into());
    }

    // can generalise this with some Yankable trait
    pub fn apply(&mut self, from: &mut ContentProvider, to: &mut ContentProvider) {
        for &cid in &self.yanked_items {
            if YankType::Cut == self.yank_type {
                from.remove(cid);
            }
            to.add(cid);
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
    Yanked(Yanker),
    IndexChange{
        provider: ID,
        from: usize,
        to: usize,
    },
    TextEdit{
        content: ID,
        from: String,
        to: String,
    },
}
