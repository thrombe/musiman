
use crate::{
    app::app::{
        App,
        AppState,
    },
};

#[derive(Debug)]
pub enum AppAction {
    None,
    Actions {
        actions: Vec<AppAction>,
    },
    EnableTyping {
        content: String,
    },
    UpdateDisplayContent {
        content: Vec<String>,
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

    pub fn apply(self, app: &mut App) {
        match self {
            Self::None => (),
            Self::Actions {actions} => {
                for action in actions {
                    action.apply(app);
                }
            }
            Self::EnableTyping {mut content} => {
                app.state = AppState::Typing;
                // app.input = content.chars().collect();
                app.input = content.drain(..).collect();
                app.input_cursor_pos = app.input.len();
            }
            Self::UpdateDisplayContent {content} => {
                app.browser_widget.options = content;
            }
        }
    }
}