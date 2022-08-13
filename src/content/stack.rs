
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};


use crate::{
    content::register::{
        GlobalContent,
        ContentProviderID,
        GlobalProvider,
    },
    app::app::SelectedIndex,
};

#[derive(Clone, Debug)]
pub enum ContentState {
    Normal,
    Menu {
        ctx: StateContext,
        id: GlobalContent,
    },
    Edit {
       ctx: StateContext,
       id: GlobalContent,
    },
    GlobalMenu(SelectedIndex),
}
impl Default for ContentState {
    fn default() -> Self {
        Self::Normal
    }
}
#[derive(Clone, Debug)]
pub struct StateContext(Vec<SelectedIndex>);
impl Default for StateContext {
    fn default() -> Self {
        Self(vec![Default::default()])
    }
}
impl StateContext {
    pub fn pop(&mut self) -> Option<SelectedIndex> {
        self.0.pop()
    }
    pub fn push(&mut self, i: SelectedIndex) {
        self.0.push(i);
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn last_mut(&mut self) -> &mut SelectedIndex {
        self.0.last_mut().unwrap()
    }
    pub fn last(&self) -> &SelectedIndex {
        self.0.last().unwrap()
    }
    pub fn get(&self, index: usize) -> Option<&SelectedIndex> {
        self.0.get(index)
    }
}

#[derive(Clone, Debug)]
pub struct ContentStack {
    state: ContentState,
    stack: Vec<GlobalProvider>,
}
impl ContentStack {
    pub fn new<T>(main_provider: T) -> Self
        where T: Into<ContentProviderID>
    {
        Self {
            state: Default::default(),
            stack: vec![main_provider.into().into()],
        }
    }

    pub fn get_state(&self) -> &ContentState {
        &self.state
    }

    pub fn get_state_mut(&mut self) -> &mut ContentState {
        &mut self.state
    }

    pub fn set_state(&mut self, state: ContentState) {
        self.state = state;
    }

    pub fn open_menu<T>(&mut self, id: T)
        where T: Into<GlobalContent>
    {
        self.state = ContentState::Menu {
            ctx: Default::default(),
            id: id.into(),
        }
    }

    pub fn open_edit<T>(&mut self, id: T)
        where T: Into<GlobalContent>
    {
        self.state = ContentState::Edit {
            ctx: Default::default(),
            id: id.into(),
        }
    }

    pub fn open_global_menu(&mut self) {
        self.state = ContentState::GlobalMenu(Default::default());
    }

    pub fn set_state_normal(&mut self) {
        self.state = ContentState::Normal;
    }
    
    pub fn main_provider(&self) -> ContentProviderID {
        if let GlobalProvider::ContentProvider(id) = self.stack.first().unwrap() {
            return *id;
        }
        unreachable!()
    }
    
    pub fn push<T>(&mut self, id: T)
        where T: Into<GlobalProvider>
    {
        self.stack.push(id.into());
    }
    
    pub fn pop(&mut self) -> Option<GlobalProvider> {
        dbg!(&self);
        debug!("popping");
        match &mut self.state {
            ContentState::Normal => {
                if self.stack.len() > 1 {
                    self.stack.pop()
                } else {
                    None
                }
            }
            ContentState::Edit { ctx, .. } | ContentState::Menu { ctx, .. } => {
                if ctx.len() > 1 {
                    ctx.pop();
                    None
                } else {
                    self.state = ContentState::Normal;
                    None
                }
            }
            _ => {
                self.state = ContentState::Normal;
                None
            }
        }
    }

    pub fn last(&self) -> GlobalProvider {
        *self.stack.last().unwrap()
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn get(&self, index: usize) -> GlobalProvider {
        self.stack[index]
    }
}