
use crate::{
    content_handler::{ContentID, ContentType}, content_providers::ContentProvider
};



#[derive(Clone)]
pub struct Yanker {
    yanked_items: Vec<ContentID>,
    yank_type: YankType,
    yanked_from: Option<ContentID>,
    yanked_to: Option<ContentID>, // for undo
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
        }
    }

    pub fn yank(&mut self, cid: ContentID) {
        if self.yanked_items.len() != 0 && cid.content_type != self.yanked_content_type().unwrap() {
            self.yanked_items.clear();
        }
        self.yanked_items.push(cid);
    }

    pub fn yanked_content_type(&self) -> Option<ContentType> {
        if self.yanked_items.len() > 0 {
            Some(self.yanked_items[0].content_type)
        } else {
            None
        }
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

pub struct UndoManager {
    edits: Vec<Edit>,
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            edits: vec![],
        }
    }
}

pub enum Edit {
    Yanked(Yanker),
    IndexChange{
        provider: ContentID,
        from: usize,
        to: usize,
    },
    TextEdit{
        content: ContentID,
        from: String,
        to: String,
    },
}
