
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
use std::{borrow::Cow, fmt::Debug};
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


#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct Yank<T> {
    pub item: T,
    pub index: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum YankedContent {
    Songs {
        items: Vec<Yank<SongID>>,
    },
    Providers {
        items: Vec<Yank<ContentProviderID>>,
    },
}

impl From<Vec<Yank<SongID>>> for YankedContent {
    fn from(items: Vec<Yank<SongID>>) -> Self {
        Self::Songs { items }
    }
}
impl From<Vec<Yank<ContentProviderID>>> for YankedContent {
    fn from(items: Vec<Yank<ContentProviderID>>) -> Self {
        Self::Providers { items }
    }
}

impl YankedContent {
    fn len(&self) -> usize {
        match self {
            YankedContent::Songs { items } => {
                items.len()
            }
            YankedContent::Providers { items } => {
                items.len()
            }
        }
    }

    fn remove<I: Into<ID>>(&mut self, id: I, index: usize) -> bool {
        let len = self.len();
        match id.into() {
            ID::Song(id) => {
                let y = Yank { item: id, index };
                match self {
                    YankedContent::Songs { items } => items.retain(|&i| i != y),
                    YankedContent::Providers { .. } => (),
                }        
            }
            ID::ContentProvider(id) => {
                let y = Yank { item: id, index };
                match self {
                    YankedContent::Songs { .. } => (),
                    YankedContent::Providers { items } => items.retain(|&i| i != y),
                }        
            }
        }
        len > self.len()
    }

    pub fn iter<'a>(&'a self) -> YankContentIter<'a> {
        YankContentIter {
            yank: self,
            counter: 0,
        }
    }
}

pub struct YankContentIter<'a> {
    yank: &'a YankedContent,
    counter: usize,
}
impl<'a> Iterator for YankContentIter<'a> {
    type Item = ID;
    fn next(&mut self) -> Option<Self::Item> {
        let id = match self.yank {
            YankedContent::Songs { items } => {
                items.get(self.counter).map(|y| y.item).map(Into::into)
            }
            YankedContent::Providers { items } => {
                items.get(self.counter).map(|y| y.item).map(Into::into)
            }
        };
        self.counter += 1;
        id
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Yanker { // all ids here are weak (but not enforced to be weak)
    pub items: YankedContent,
    yanked_from: ContentProviderID, // not allowed to yank stuff from multiple places
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum YankType {
    Copy,
    Cut,
}

impl Yanker {
    pub fn new<I: Into<ID>>(from: ContentProviderID, id: I, index: usize) -> Self {
        Self {
            items: match id.into() {
                ID::Song(id) => {
                    YankedContent::Songs { items: vec![Yank { item: id, index }] }
                }
                ID::ContentProvider(id) => {
                    YankedContent::Providers { items: vec![Yank { item: id, index }] }
                }
            },
            yanked_from: from,
        }
    }

    fn yank<I: Into<ID>>(&mut self, id: I, provider_id: ContentProviderID, index: usize) {
        if provider_id != self.yanked_from {
            self.yanked_from = provider_id;
        }
        match id.into() {
            ID::Song(id) => {
                let y = Yank { item: id, index };
                match &mut self.items {
                    YankedContent::Songs { items } => {
                        items.push(y);
                    }
                    YankedContent::Providers { .. } => {
                        self.items = YankedContent::Songs { items: vec![y] }
                    }
                }
            }
            ID::ContentProvider(id) => {
                let y = Yank { item: id, index };
                match &mut self.items {
                    YankedContent::Songs { .. } => {
                        self.items = YankedContent::Providers { items: vec![y] }
                    }
                    YankedContent::Providers { items } => {
                        items.push(y);
                    }
                }
            }
        }
    }

    pub fn marker_symbol() -> Span<'static> {
        // "|".into()
        Span { content: Cow::Borrowed("█"), style: Style::default().fg(Color::Green) }
    }

    pub fn toggle_yank<I: Into<ID>>(&mut self, id: I, provider_id: ContentProviderID, index: usize) {
        let id = id.into();
        if provider_id == self.yanked_from {
            if self.items.remove(id, index) {
                return;
            }
        }
        self.yank(id, provider_id, index);
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
        let edit = Edit::Yanked {
            yank: self.yanker.clone().unwrap().items,
            yank_type,
            yanked_from: self.yanker.as_ref().unwrap().yanked_from,
        };
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
            Edit::Yanked { yank, yanked_from, .. } => {
                self.yanker = Some(Yanker { items: yank.clone(), yanked_from: *yanked_from });
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
                    return YankAction::Conditional {
                        if_this: vec![YankAction::TryPasteIntoProvider {yank: yank.clone(), yanked_to: id, paste_pos: pos}],
                        then_this: vec![YankAction::PushEdit { edit: Edit::Pasted { yank, yanked_to: id, paste_pos: pos } }],
                        else_this: vec![],
                    };
                }
            }
        }
        None.into()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Edit {
    Pasted {
        yank: YankedContent,
        yanked_to: ContentProviderID,
        paste_pos: Option<usize>,
    },
    Yanked {
        yank: YankedContent,
        yank_type: YankType,
        yanked_from: ContentProviderID,
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
            Self::Yanked { yank, yank_type, yanked_from } => {
                match yank_type {
                    YankType::Copy => None.into(),
                    YankType::Cut => {
                        vec![
                            ContentManagerAction::Register { ids: yank.iter().collect() }.into(), // for being stored in undo_stack
                            YankAction::InsertIntoProvider { yank: yank.clone(), yanked_to: *yanked_from },
                            ContentManagerAction::RefreshDisplayContent.into(),
                        ].into()
                    },
                }
            },
            Self::Pasted { yank, yanked_to, .. } => {
                vec![
                    YankAction::RemoveFromProvider { yank: yank.clone(), yanked_from: *yanked_to },
                    ContentManagerAction::Unregister { ids: yank.iter().collect() }.into(), // for being removed from the provider
                    ContentManagerAction::RefreshDisplayContent.into(),
                ].into()
            },
            Self::TextEdit { content, from, to } => todo!(),
        }
    }

    fn apply_redo(&self) -> YankAction {
        match self {
            Self::Yanked { yank, yank_type, yanked_from } => {
                match yank_type {
                    YankType::Copy => None.into(),
                    YankType::Cut => {
                        vec![
                            YankAction::RemoveFromProvider { yank: yank.clone(), yanked_from: *yanked_from },
                            ContentManagerAction::Unregister { ids: yank.iter().collect() }.into(),
                            ContentManagerAction::RefreshDisplayContent.into(),
                        ].into()
                    },
                }
            },
            Self::Pasted { yank, yanked_to, paste_pos  } => {
                vec![
                    YankAction::PasteIntoProvider { yank: yank.clone(), yanked_to: *yanked_to, paste_pos: *paste_pos },
                    ContentManagerAction::Register { ids: yank.iter().collect() }.into(), // for being saved in the provider
                    ContentManagerAction::RefreshDisplayContent.into(),
                ].into()
            },
            Self::TextEdit { content, from, to } => todo!(),
        }
    }

    fn unregister(&self) -> YankAction {
        let yank = match self {
            Self::Pasted { yank, yanked_to, .. } => yank,
            Self::Yanked { yank, yanked_from, .. } => yank,
            Self::TextEdit { content, from, to } => todo!(),
        };
        ContentManagerAction::Unregister { ids: yank.iter().collect() }.into()
    }

    pub fn apply(&self) -> YankAction {
        match self {
            Self::Yanked { yank, yank_type, yanked_from } => {
                match yank_type {
                    YankType::Copy => {
                        YankAction::Conditional {
                            if_this: vec![YankAction::ProviderExists { id: *yanked_from }],
                            then_this: vec![
                                    ContentManagerAction::Register { ids: yank.iter().collect() }.into(),
                                    ContentManagerAction::RefreshDisplayContent.into(),
                                ].into(),
                            else_this: vec![YankAction::DropYanker],
                        }
                    }
                    YankType::Cut => { // now the yanker owns these ids, so no unregistering
                        YankAction::Conditional {
                            if_this: vec![YankAction::ProviderExists { id: *yanked_from }],
                            then_this: vec![
                                    YankAction::RemoveFromProvider { yank: yank.clone(), yanked_from: *yanked_from },
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
        yank: YankedContent,
        yanked_from: ContentProviderID,
    },
    TryPasteIntoProvider {
        #[derivative(Debug="ignore")]
        yank: YankedContent,
        yanked_to: ContentProviderID,
        paste_pos: Option<usize>,
    },
    PasteIntoProvider {
        #[derivative(Debug="ignore")]
        yank: YankedContent,
        yanked_to: ContentProviderID,
        paste_pos: Option<usize>,
    },
    InsertIntoProvider {
        #[derivative(Debug="ignore")]
        yank: YankedContent,
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
    /// returns false if any of the actions fail (except the "if" part of conditional action)
    pub fn apply(self, ch: &mut ContentManager) -> Result<bool> {
        dbg!(&self);
        match self {
            Self::RemoveFromProvider { yank, yanked_from } => { // (un)registering handled on caller's side
                let res = match yank {
                    YankedContent::Songs { items } => {
                        ch.get_provider_mut(yanked_from)
                        .as_song_yank_dest_mut()
                        .map(|y| y.remove(items))
                    }
                    YankedContent::Providers { items } => {
                        ch.get_provider_mut(yanked_from)
                        .as_provider_yank_dest_mut()
                        .map(|y| y.remove(items))
                    }
                }.is_some();
                return Ok(res);
            }
            Self::TryPasteIntoProvider { yank, yanked_to, paste_pos } => {
                match yank {
                    YankedContent::Songs { items } => {
                        let b = ch.get_provider_mut(yanked_to)
                        .as_song_yank_dest_mut()
                        .map(|y| y.try_paste(items, paste_pos, yanked_to))
                        .map(|a| a.apply(ch));
                        return b.unwrap_or(Ok(false));
                    }
                    YankedContent::Providers { items } => {
                        let e = items.iter()
                        .map(|y| {
                            ch.get_provider_mut(y.item)
                            .as_loadable()
                            .map(|cp| cp.maybe_load(y.item).ok())
                            .flatten()
                            .unwrap_or(None.into())
                            .apply(ch)
                        })
                        .collect::<Result<()>>();

                        let b = ch.get_provider_mut(yanked_to)
                        .as_provider_yank_dest_mut()
                        .map(|y| y.try_paste(items, paste_pos, yanked_to))
                        .map(|a| a.apply(ch));

                        return match e {
                            Ok(_) => b.unwrap_or(Ok(false)),
                            Err(e) => Err(e),
                        };
                    }
                }
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
                let res = match yank {
                    YankedContent::Songs { items } => {
                        ch.get_provider_mut(yanked_to)
                        .as_song_yank_dest_mut()
                        .map(|y| y.paste(items, paste_pos))
                    }
                    YankedContent::Providers { items } => {
                        ch.get_provider_mut(yanked_to)
                        .as_provider_yank_dest_mut()
                        .map(|y| y.paste(items, paste_pos))
                    }
                }.is_some();
                return Ok(res);
            }
            Self::InsertIntoProvider { yank, yanked_to } => { // TODO: maybe crash instead of map
                let res = match yank {
                    YankedContent::Songs { items } => {
                        ch.get_provider_mut(yanked_to)
                        .as_song_yank_dest_mut()
                        .map(|y| y.insert(items))
                    }
                    YankedContent::Providers { items } => {
                        ch.get_provider_mut(yanked_to)
                        .as_provider_yank_dest_mut()
                        .map(|y| y.insert(items))
                    }
                }.is_some();
                return Ok(res);
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