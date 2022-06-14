
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use derivative::Derivative;
use anyhow::Result;

use crate::{
    app::app::{
        App,
        AppState,
    },
    content::{
        providers::ContentProvider,
        action::ContentManagerAction,
        register::ID,
    },
};

#[derive(Derivative)]
#[derivative(Debug)]
pub enum AppAction {
    None,
    Actions {
        actions: Vec<AppAction>,
    },
    EnableTyping {
        content: String,
        #[derivative(Debug="ignore")]
        callback: Box<dyn Fn(&mut ContentProvider, String) -> ContentManagerAction + 'static + Send + Sync>,
        loader: ID,
    },
    UpdateDisplayContent {
        content: Vec<String>,
    },
    Redraw,
    ApplyTyped {
        #[derivative(Debug="ignore")]
        callback: Box<dyn Fn(&mut ContentProvider, String) -> ContentManagerAction + 'static + Send + Sync>,
        loader: ID,        
    }
}
impl Default for AppAction {
    fn default() -> Self {
        Self::None
    }
}
impl Into<AppAction> for Vec<AppAction> {
    fn into(self) -> AppAction {
        AppAction::Actions {
            actions: self,
        }
    }
}
impl AppAction {
    pub fn queue(&mut self, action: Self) {
        match self {
            Self::Actions {actions} => {
                match action {
                    AppAction::Actions { actions: more_actions } => {
                        actions.extend(more_actions)
                    }
                    AppAction::None => (),
                    a => {
                        actions.push(a);
                    }
                }
            }
            Self::None => {
                *self = action;
            }
            _ => {
                let a = std::mem::replace(self, vec![].into());
                self.queue(a);
                self.queue(action);
            }
        }
    }

    fn dbg_log(&self) {
        if let Self::None = self {return;}
        dbg!(&self);
    }

    pub fn apply(self, app: &mut App) -> Result<()> {
        self.dbg_log();
        match self {
            Self::None => (),
            Self::Actions {actions} => {
                for action in actions {
                    action.apply(app)?;
                }
            }
            Self::EnableTyping {mut content, callback, loader} => {
                app.state = AppState::Typing;
                // app.input = content.chars().collect();
                app.input = content.drain(..).collect();
                app.input_cursor_pos = app.input.len();
                app.typing_callback = AppAction::ApplyTyped { callback, loader }
            }
            Self::ApplyTyped {callback, loader} => {
                match loader {
                    ID::ContentProvider(id) => {
                        let cp = app.content_manager.get_provider_mut(id);
                        let action = callback(cp, app.input[..].iter().collect());
                        action.apply(&mut app.content_manager)?;
                    }
                    ID::Song(id) => {
                        let s = app.content_manager.get_song_mut(id);
                        todo!() // FIX: this is not supported rn!!
                    }
                }
            }
            Self::UpdateDisplayContent {content} => {
                app.browser_widget.options = content;
            }
            Self::Redraw => {
                app.redraw_needed = true;
            }
        }
        Ok(())
    }
}