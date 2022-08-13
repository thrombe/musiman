
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
use derivative::Derivative;
use anyhow::Result;

use crate::{
    content::{
        providers::traits::YankContext,
        register::{
            ID,
            SongID,
            ContentProviderID,
            GlobalProvider,
        },
        manager::{
            action::ContentManagerAction,
            manager::ContentManager,
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
    pub yanked_items: Vec<(ID, usize)>,
    pub content_type: YankedContentType,
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

    fn yank_song(&mut self, id: SongID, provider_id: ContentProviderID, index: usize) {
        if provider_id != self.yanked_from || self.content_type != YankedContentType::Song {
            self.yanked_items.clear();
            self.content_type = YankedContentType::Song;
            self.yanked_from = provider_id;
        }
        self.yanked_items.push((id.into(), index));
    }

    fn yank_content_provider(&mut self, id: ContentProviderID, provider_id: ContentProviderID, index: usize) {
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

    pub fn toggle_yank(&mut self, id: ID, provider_id: ContentProviderID, index: usize) {
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

    pub fn yanked_songs(&self) -> Option<Vec<(SongID, usize)>> {
        match self.content_type {
            YankedContentType::Song => Some(self.yanked_items.iter().cloned().map(|(id, i)| match id {
                ID::Song(id) => (id, i),
                ID::ContentProvider(_) => unreachable!(),
            }).collect()),
            YankedContentType::ContentProvider => None,
            YankedContentType::None => None,
        }
    }

    pub fn yanked_providers(&self) -> Option<Vec<(ContentProviderID, usize)>> {
        match self.content_type {
            YankedContentType::ContentProvider => Some(self.yanked_items.iter().cloned().map(|(id, i)| match id {
                ID::ContentProvider(id) => (id, i),
                ID::Song(_) => unreachable!(),
            }).collect()),
            YankedContentType::Song => None,
            YankedContentType::None => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EditManager {
    pub yanker: Option<Yanker>,

    // TODO: vectors do not seem the best fit for this job. some kinda circular stack with fixed length (it's called a ring buffer i think) might be better
    // all ids stored in here should be valid. (non weak). and should get unregistered once the edits are removed
    // but Yanker::{yank_from, yank_to} are still weak ig
    pub edit_stack: Vec<Edit>,
    pub undo_stack: Vec<Edit>, // edits get popped off and get stored here after getting converted into their undo edit
}

impl EditManager {
    pub fn new() -> Self {
        Self {
            yanker: None,
            edit_stack: vec![],
            undo_stack: Default::default(),
        }
    }

    pub fn clear(&mut self) -> YankAction {
        let action = self.edit_stack
        .iter()
        .chain(self.undo_stack.iter())
        .map(|e| e.unregister())
        .collect::<Vec<_>>().into();

        *self = Self::new();
        action
    }

    pub fn apply_yank(&mut self, yank_type: YankType) -> YankAction {
        if self.yanker.is_none() {return None.into()}
        let edit = Edit::Yanked { yank: self.yanker.clone().unwrap(), yank_type };
        let action = edit.apply();
        YankAction::Conditional {
            if_this: vec![action],
            then_this: vec![
                YankAction::ClearUndoStack,
                YankAction::PushEdit { edit },
                YankAction::DropYanker,
            ],
            else_this: vec![YankAction::False],
        }
    }

    /// assuming undo should not fail (unlike trying to apply some edit first time)
    pub fn undo_last_edit(&mut self) -> YankAction {
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

    /// assuming redo should not fail (unlike trying to apply some edit first time)
    pub fn redo_last_undo(&mut self) -> YankAction {
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

    pub fn try_paste(&mut self, id: GlobalProvider, pos: Option<usize>) -> YankAction {
        if let Some(Edit::Yanked { .. }) = self.edit_stack.last() {
            if let GlobalProvider::ContentProvider(id) = id {
                if let Edit::Yanked { yank, .. } = self.edit_stack.last().cloned().unwrap() {
                    return YankAction::Conditional { // ?: YankDest::try_paste should call PushEdit as the items of yank may be replaced
                        if_this: vec![YankAction::TryPasteIntoProvider {yank: yank.clone(), yanked_to: id, paste_pos: pos}],
                        then_this: vec![YankAction::PushEdit { edit: Edit::Pasted { yank, yanked_to: id, paste_pos: pos } }],
                        else_this: vec![],
                    };
                    // return YankAction::TryPasteIntoProvider {yank, yanked_to: id, paste_pos: pos};
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
    fn apply_undo(&self) -> YankAction {
        match self {
            Self::Yanked { yank, yank_type } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>(); // TODO: restore each at correct position
                match yank_type {
                    YankType::Copy => None.into(),
                    YankType::Cut => {
                        vec![
                            ContentManagerAction::Register {ids: items.clone()}.into(), // for being stored in undo_stack
                            YankAction::InsertIntoProvider {yank: yank.clone(), yanked_to: yank.yanked_from},
                            ContentManagerAction::RefreshDisplayContent.into(),
                        ].into()
                    },
                }
            },
            Self::Pasted { yank, yanked_to, .. } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>();
                vec![
                    YankAction::RemoveFromProvider { yank: yank.clone(), yanked_from: yank.yanked_from },
                    ContentManagerAction::Unregister {ids: items}.into(), // for being removed from the provider
                    ContentManagerAction::RefreshDisplayContent.into(),
                ].into()
            },
            Self::TextEdit { content, from, to } => todo!(),
        }
    }

    fn apply_redo(&self) -> YankAction {
        match self {
            Self::Yanked { yank, yank_type } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>();
                match yank_type {
                    YankType::Copy => None.into(),
                    YankType::Cut => {
                        vec![
                            YankAction::RemoveFromProvider { yank: yank.clone(), yanked_from: yank.yanked_from },
                            ContentManagerAction::Unregister {ids: items.clone()}.into(),
                            ContentManagerAction::RefreshDisplayContent.into(),
                        ].into()
                    },
                }
            },
            Self::Pasted { yank, yanked_to, paste_pos  } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>();
                vec![
                    YankAction::PasteIntoProvider {yank: yank.clone(), yanked_to: *yanked_to, paste_pos: *paste_pos},
                    ContentManagerAction::Register {ids: items}.into(), // for being saved in the provider
                    ContentManagerAction::RefreshDisplayContent.into(),
                ].into()
            },
            Self::TextEdit { content, from, to } => todo!(),
        }
    }

    fn unregister(&self) -> YankAction {
        let yank = match self {
            Self::Pasted { yank, .. } => yank,
            Self::Yanked { yank, .. } => yank,
            Self::TextEdit { content, from, to } => todo!(),
        };
        let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>();
        ContentManagerAction::Unregister { ids: items }.into()
    }

    pub fn apply(&self) -> YankAction {
        match self {
            Self::Yanked { yank, yank_type } => {
                let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect();
                match yank_type {
                    YankType::Copy => {
                        YankAction::Conditional {
                            if_this: vec![YankAction::ProviderExists { id: yank.yanked_from }],
                            then_this: vec![
                                    ContentManagerAction::Register { ids: items }.into(),
                                    ContentManagerAction::RefreshDisplayContent.into(),
                                ].into(),
                            else_this: vec![YankAction::DropYanker],
                        }
                    }
                    YankType::Cut => { // now the yanker owns these ids, so no unregistering
                        YankAction::Conditional {
                            if_this: vec![YankAction::ProviderExists { id: yank.yanked_from }],
                            then_this: vec![
                                    YankAction::RemoveFromProvider { yank: yank.clone(), yanked_from: yank.yanked_from },
                                    ContentManagerAction::RefreshDisplayContent.into(),
                                ].into(),
                            else_this: vec![YankAction::DropYanker],
                        }
                    }
                }
            }
            Self::Pasted { .. } => {
                unreachable!();
            }
            // Self::Pasted { yank, yanked_to, paste_pos } => {
            //     let items = yank.yanked_items.iter().cloned().map(|(id, _)| id).collect::<Vec<_>>();
            //     vec![
            //         YankAction::TryPasteIntoProvider {yank: yank.clone(), yanked_to: *yanked_to, paste_pos: *paste_pos}, // ? might loop infinitely
            //         ContentManagerAction::Register {ids: items.clone()}.into(), // for being stored in the provider
            //         ContentManagerAction::Register {ids: items}.into(), // for being stored in Edit::Pasted
            //         ContentManagerAction::RefreshDisplayContent.into(),
            //     ].into()
            // }
            Self::TextEdit { content, from, to } => todo!(),
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum YankAction {
    RemoveFromProvider {
        #[derivative(Debug="ignore")]
        yank: Yanker,
        yanked_from: ContentProviderID,
    },
    TryPasteIntoProvider {
        #[derivative(Debug="ignore")]
        yank: Yanker,
        yanked_to: ContentProviderID,
        paste_pos: Option<usize>,
    },
    PasteIntoProvider {
        #[derivative(Debug="ignore")]
        yank: Yanker,
        yanked_to: ContentProviderID,
        paste_pos: Option<usize>,
    },
    InsertIntoProvider {
        #[derivative(Debug="ignore")]
        yank: Yanker,
        yanked_to: ContentProviderID,
    },
    PushEdit {
        #[derivative(Debug="ignore")]
        edit: Edit,
    },
    ClearUndoStack,
    Callback {
        #[derivative(Debug="ignore")]
        callback: Box<dyn FnOnce(YankContext) -> YankAction + 'static + Send + Sync>,
    },
    ContentManagerAction {
        a: ContentManagerAction,
    },
    ProviderExists {
        id: ContentProviderID,
    },
    DropYanker,
    False,
    Conditional {
        // using Vec<Self> instead of Box<Self>
        if_this: Vec<Self>, // the bool output of this is not considered in the overall bool output
        then_this: Vec<Self>,
        else_this: Vec<Self>,
    },
    Actions {
        v: Vec<Self>,
    },
    None,
}
impl From<ContentManagerAction> for YankAction {
    fn from(a: ContentManagerAction) -> Self {
        Self::ContentManagerAction { a }
    }
}
impl From<Vec<YankAction>> for YankAction {
    fn from(v: Vec<YankAction>) -> Self {
        Self::Actions { v }
    }
}
impl Into<YankAction> for Option<YankAction> {
    fn into(self) -> YankAction {
        match self {
            Self::Some(a) => {
                a
            }
            None => {
                YankAction::None
            }
        }
    }
}

impl YankAction {
    /// returns true if // TODO: decide true/false behaviour
    pub fn apply(self, ch: &mut ContentManager) -> Result<bool> {
        dbg!(&self);
        match self {
            Self::RemoveFromProvider { yank, yanked_from } => { // (un)registering handled on caller's side
                let a = yank.yanked_songs()
                .map(|items| {
                    ch.get_provider_mut(yanked_from)
                    .as_song_yank_dest_mut()
                    .map(|y| y.remove(items))
                })
                .flatten();

                let b = yank.yanked_providers()
                .map(|items| {
                    ch.get_provider_mut(yanked_from)
                    .as_provider_yank_dest_mut()
                    .map(|y| y.remove(items))
                })
                .flatten();

                dbg!(a, b);
                return Ok(a.or(b).is_some());
            }
            Self::TryPasteIntoProvider { yank, yanked_to, paste_pos } => {
                let a = yank.yanked_songs()
                .map(|items| {
                    ch.get_provider_mut(yanked_to)
                    .as_song_yank_dest_mut()
                    .map(|y| y.try_paste(items.into_iter().map(|(id, _)| id).collect(), paste_pos, yanked_to))
                    .map(|a| a.apply(ch))
                })
                .flatten();
                
                let b = yank.yanked_providers()
                .map(|items| {
                    let e = items.iter().cloned()
                    .map(|(id, _)| {
                        ch.get_provider_mut(id)
                        .as_loadable()
                        .map(|cp| 
                            cp.maybe_load(id)
                            .ok()
                        )
                        .unwrap_or(None.into())
                        .unwrap_or(None.into())
                        .apply(ch)
                    })
                    .collect::<Result<()>>();

                    let b = ch.get_provider_mut(yanked_to)
                    .as_provider_yank_dest_mut()
                    .map(|y| y.try_paste(items.into_iter().map(|(id, _)| id).collect(), paste_pos, yanked_to))
                    .map(|a| a.apply(ch));

                    match e {
                        Ok(_) => b,
                        Err(e) => Some(Err(e)),
                    }
                })
                .flatten();

                return a.or(b).unwrap_or(Ok(false));
            }
            Self::ProviderExists { id } => {
                return Ok(ch.content_providers.get(id).is_some());
            }
            Self::PushEdit { edit } => {
                ch.edit_manager.edit_stack.push(edit);
            }
            Self::ClearUndoStack => {
                if ch.edit_manager.undo_stack.len() > 0 {
                    let stack = std::mem::replace(&mut ch.edit_manager.undo_stack, vec![]);
                    let stack: YankAction = stack.into_iter().map(|e| e.unregister()).collect::<Vec<_>>().into();
                    return stack.apply(ch);
                }
            }
            Self::DropYanker => {
                let _ = ch.edit_manager.yanker.take();
            }
            Self::PasteIntoProvider { yank, yanked_to, paste_pos } => { // TODO: maybe crash instead of map
                let a = yank.yanked_songs()
                .map(|items|
                    ch.get_provider_mut(yanked_to)
                    .as_song_yank_dest_mut()
                    .map(|y| y.paste(items.into_iter().map(|(id, _)| id).collect(), paste_pos))
                )
                .flatten();
                
                let b = yank.yanked_providers()
                .map(|items|
                    ch.get_provider_mut(yanked_to)
                    .as_provider_yank_dest_mut()
                    .map(|y| y.paste(items.into_iter().map(|(id, _)| id).collect(), paste_pos))
                )
                .flatten();

                return Ok(a.or(b).is_some());
            }
            Self::InsertIntoProvider { yank, yanked_to } => { // TODO: maybe crash instead of map
                let a = yank.yanked_songs()
                .map(|items|
                    ch.get_provider_mut(yanked_to)
                    .as_song_yank_dest_mut()
                    .map(|y| y.insert(items))
                )
                .flatten();
                
                let b = yank.yanked_providers()
                .map(|items|
                    ch.get_provider_mut(yanked_to)
                    .as_provider_yank_dest_mut()
                    .map(|y| y.insert(items))
                )
                .flatten();

                return Ok(a.or(b).is_some());
            }
            Self::Callback { callback } => {
                return callback(YankContext::new(ch)).apply(ch);
            }
            Self::Actions { v } => {
                return v.into_iter()
                .map(|a| a.apply(ch))
                .collect::<Result<Vec<bool>>>()
                .map(|r|
                    r.into_iter()
                    .all(|b| b)
                );
            }
            Self::Conditional { if_this, then_this, else_this } => {
                let if_this = Self::Actions { v: if_this };
                let then_this = Self::Actions { v: then_this };
                let else_this = Self::Actions { v: else_this };
                if if_this.apply(ch)? {
                    dbg!(true);
                    return then_this.apply(ch);
                } else {
                    dbg!(false);
                    return else_this.apply(ch);
                }
            }
            Self::ContentManagerAction { a } => {
                a.apply(ch)?;
            }
            Self::False => {
                return Ok(false);
            }
            Self::None => {}
        }
        Ok(true)
    }
}