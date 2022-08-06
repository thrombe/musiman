
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
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::{
    content::{
        register::{
            ID,
            SongID,
            ContentProviderID,
            GlobalProvider,
        },
        manager::{
            action::ContentManagerAction,
        },
    },
};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum YankedContentType {
    Song,
    ContentProvider,
    None,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Yanker { // all ids here are weak (but not enforced to be weak)
    pub yanked_items: Vec<(ID, Option<usize>)>,
    content_type: YankedContentType,
    yanked_from: ContentProviderID, // not allowed to yank stuff from multiple places
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum YankType {
    Copy,
    Cut,
}

impl Yanker {
    pub fn new(from: ContentProviderID) -> Self {
        Self {
            yanked_items: vec![],
            yanked_from: from,
            content_type: YankedContentType::None,
        }
    }

    fn yank_song(&mut self, id: SongID, provider_id: ContentProviderID, index: Option<usize>) {
        if provider_id != self.yanked_from || self.content_type != YankedContentType::Song {
            self.yanked_items.clear();
            self.content_type = YankedContentType::Song;
            self.yanked_from = provider_id;
        }
        self.yanked_items.push((id.into(), index));
    }

    fn yank_content_provider(&mut self, id: ContentProviderID, provider_id: ContentProviderID, index: Option<usize>) {
        if provider_id != self.yanked_from || self.content_type != YankedContentType::ContentProvider {
            self.yanked_items.clear();
            self.content_type = YankedContentType::ContentProvider;
            self.yanked_from = provider_id;
        }
        self.yanked_items.push((id.into(), index));
    }

    pub fn marker_symbol() -> Span<'static> {
        // "|".into()
        Span { content: Cow::Borrowed("█"), style: Style::default().fg(Color::Green) }
    }

    pub fn toggle_yank(&mut self, id: ID, provider_id: ContentProviderID, index: Option<usize>) {
        if provider_id == self.yanked_from {
            let old_len = self.yanked_items.len();
            self.yanked_items.retain(|(y_id, _)| id != *y_id);
            let new_len = self.yanked_items.len();
            if old_len > new_len {
                return;
            }
        }
        match id {
            ID::Song(id) => self.yank_song(id, provider_id, index),
            ID::ContentProvider(id) => self.yank_content_provider(id, provider_id, index),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EditManager {
    pub yanker: Option<Yanker>,

    // TODO: vectors do not seem the best fit for this job. some kinda circular stack with fixed length (it's called a ring buffer i think) might be better
    // all ids stored in here should be valid. (non weak). and should get unregistered once the edits are removed
    // but Yanker::{yank_from, yank_to} are still weak ig
    edit_stack: Vec<Edit>,
    undo_stack: Vec<Edit>, // edits get popped off and get stored here after getting converted into their undo edit
}

impl EditManager {
    pub fn new() -> Self {
        Self {
            yanker: None,
            edit_stack: vec![],
            undo_stack: Default::default(),
        }
    }

    pub fn clear(&mut self) -> ContentManagerAction {
        let action = self.edit_stack
        .iter()
        .chain(self.undo_stack.iter())
        .map(|e| e.unregister())
        .collect::<Vec<_>>().into();

        *self = Self::new();
        action
    }

    pub fn apply_yank(&mut self, yank_type: YankType) -> ContentManagerAction {
        if self.yanker.is_none() {return None.into()}
        let edit = Edit::Yanked { yank: self.yanker.take().unwrap(), yank_type };
        let action = edit.apply();
        self.edit_stack.push(edit);
        let stack = std::mem::replace(&mut self.undo_stack, vec![]);
        let stack = stack.into_iter().map(|e| e.unregister()).collect::<Vec<_>>().into();
        vec![
            stack,
            action,
        ].into()
    }

    pub fn undo_last_edit(&mut self) -> ContentManagerAction{
        if self.edit_stack.is_empty() {return None.into()}
        let undo = self.edit_stack.pop().unwrap();
        match &undo {
            Edit::Yanked { yank, .. } => {
                self.yanker = Some(yank.clone());
            },
            Edit::Pasted { .. } => (),
            Edit::TextEdit { .. } => (),
        }
        let action = undo.apply_undo();
        self.undo_stack.push(undo);
        action
    }

    pub fn redo_last_undo(&mut self) -> ContentManagerAction {
        if self.undo_stack.is_empty() {return None.into()}
        let redo = self.undo_stack.pop().unwrap();
        match &redo {
            Edit::Yanked { .. } => {
                self.yanker = None;
            },
            Edit::Pasted { .. } => (),
            Edit::TextEdit { .. } => (),
        }
        let action = redo.apply_redo();
        self.edit_stack.push(redo);
        action
    }

    pub fn try_paste(&mut self, id: GlobalProvider, pos: Option<usize>) -> ContentManagerAction {
        if let Some(Edit::Yanked { .. }) = self.edit_stack.last() {
            if let GlobalProvider::ContentProvider(id) = id {
                if let Edit::Yanked { yank, yank_type } = self.edit_stack.last().cloned().unwrap() {
                    let edit = Edit::Pasted { yank, yank_type, yanked_to: id, paste_pos: pos }; // FIX: no gurantee that the provider can store these
                    self.edit_stack.push(edit);
                    return self.edit_stack.last().unwrap().apply();
                }
            }
        }
        None.into()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Edit {
    Pasted {
        yank: Yanker,
        yank_type: YankType,
        yanked_to: ContentProviderID,
        paste_pos: Option<usize>,
    },
    Yanked {
        yank: Yanker,
        yank_type: YankType,
    },
    TextEdit { // also need to store info about what field changed
        content: ID,
        from: String,
        to: String,
    },
}

impl Edit {
    fn apply_undo(&self) -> ContentManagerAction {
        match self {
            Self::Yanked { yank, yank_type } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>(); // TODO: restore each at correct position
                match yank_type {
                    YankType::Copy => None.into(),
                    YankType::Cut => {
                        vec![
                            ContentManagerAction::Register {ids: items.clone()}, // for being stored in undo_stack
                            ContentManagerAction::AddToProvider {ids: items, to: yank.yanked_from, pos: None},
                            ContentManagerAction::RefreshDisplayContent,
                        ].into()
                    },
                }
            },
            Self::Pasted { yank, yanked_to, .. } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>();
                vec![
                    ContentManagerAction::RemoveFromProvider { ids: items.clone(), from: *yanked_to},
                    ContentManagerAction::Unregister {ids: items}, // for being removed from the provider
                    ContentManagerAction::RefreshDisplayContent,
                ].into()
            },
            Self::TextEdit { content, from, to } => todo!(),
        }
    }

    fn apply_redo(&self) -> ContentManagerAction {
        match self {
            Self::Yanked { yank, yank_type } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>();
                match yank_type {
                    YankType::Copy => None.into(),
                    YankType::Cut => {
                        vec![
                            ContentManagerAction::Unregister {ids: items.clone()},
                            ContentManagerAction::RemoveFromProvider { ids: items, from: yank.yanked_from },
                            ContentManagerAction::RefreshDisplayContent,
                        ].into()
                    },
                }
            },
            Self::Pasted { yank, yanked_to, paste_pos, .. } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>();
                vec![
                    ContentManagerAction::AddToProvider { ids: items.clone(), to: *yanked_to, pos: paste_pos.to_owned()},
                    ContentManagerAction::Register {ids: items}, // for being saved in the provider
                    ContentManagerAction::RefreshDisplayContent,
                ].into()
            },
            Self::TextEdit { content, from, to } => todo!(),
        }
    }

    fn unregister(&self) -> ContentManagerAction {
        let yank = match self {
            Self::Pasted { yank, .. } => yank,
            Self::Yanked { yank, .. } => yank,
            Self::TextEdit { content, from, to } => todo!(),
        };
        let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>();
        ContentManagerAction::Unregister { ids: items }
    }

    fn apply(&self) -> ContentManagerAction {
        match self {
            Self::Yanked { yank, yank_type } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect();
                match yank_type {
                    YankType::Copy => {
                        ContentManagerAction::Register { ids: items }
                    }
                    YankType::Cut => { // now the yanker owns these ids, so no unregistering
                        vec![
                            ContentManagerAction::RemoveFromProvider { ids: items, from: yank.yanked_from },
                            ContentManagerAction::RefreshDisplayContent,
                        ].into()
                    }
                }
            }
            Self::Pasted { yank, yanked_to, paste_pos, .. } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>();
                vec![
                    ContentManagerAction::AddToProvider { ids: items.clone(), to: *yanked_to, pos: paste_pos.to_owned()},
                    ContentManagerAction::Register {ids: items.clone()}, // for being stored in the provider
                    ContentManagerAction::Register {ids: items}, // for being stored in Edit::Pasted
                    ContentManagerAction::RefreshDisplayContent,
                ].into()
            }
            Self::TextEdit { content, from, to } => todo!(),
        }
    }
}

