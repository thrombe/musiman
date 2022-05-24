
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
};

use tui::{
    backend::Backend,
    widgets::{
        Block,
        Borders,
        List,
        ListItem,
        Paragraph,
        ListState,
    },
    layout::{
        Layout,
        Constraint,
        Direction,
        Alignment,
        Rect,
    },
    Terminal,
    Frame,
    style::{
        Color,
        Style,
        Modifier,
    },
    text::{
        Span,
        Spans,
        Text,
    },
};
use crossterm::{
    event::{
        self,
        Event,
        KeyCode,
        KeyEvent,
    },
};
// use unicode_width::UnicodeWidthStr; // string.width() -> gives correct width (including cjk chars) (i assume)
use anyhow::Result;

use crate::{
    content_handler::ContentHandler,
};


pub enum AppAction {
    EnableTyping {
        // index: SelectedIndex,
    },
    Actions {
        actions: Vec<AppAction>,
    },
    None,
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

    fn apply(self, app: &mut App) {
        match self {
            Self::Actions {actions} => {
                for action in actions {
                    action.apply(app);
                }
            }
            Self::None => (),
            Self::EnableTyping {} => {
                app.state = AppState::Typing;
                // dbg!(app.browser_widget.selected_index);
                // app.browser_widget.update(&mut app.content_handler);
                // dbg!(app.browser_widget.selected_index);
            }
        }
    }
}

/// wrapping ListState to make sure not to call select(None) and to eliminate the use of unwrap() on selected_index()
/// currently, theres no way to access/suggest the offset
pub struct SelectedIndex {
    index: ListState,
}
impl Into<ListState> for SelectedIndex {
    fn into(self) -> ListState {
        self.index
    }
}
impl SelectedIndex {
    fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { index: state }
    }

    fn selected_index(&self) -> usize {
        self.index.selected().unwrap()
    }

    fn select(&mut self, index: usize) {
        self.index.select(Some(index));
    }
}

#[derive(Default)]
struct BrowserWidget {
    options: Vec<String>,
    selected_index: usize,
}

impl BrowserWidget {
    fn new() -> Self {
        Self::default()
    }

    fn handle_events(&mut self, key: KeyEvent, ch: &mut ContentHandler) -> bool {
        match key.code {
            KeyCode::Char('g') => {
                // self.options = ch.menu_for_selected();
            }
            KeyCode::Char('G') => {
                ch.open_menu_for_current();
                self.update(ch);
            }
            KeyCode::Up => {
                if self.selected_index > 0 {self.selected_index -= 1};
            }
            KeyCode::Down => {
                if self.selected_index < self.options.len()-1 {self.selected_index += 1};
            }
            KeyCode::Right => {
                ch.enter(self.selected_index);
                self.update(ch);
            }
            KeyCode::Left => {
                ch.back();
                self.update(ch);
            }
            _ => return false,
        }
        true
    }

    fn update(&mut self, ch: &mut ContentHandler) {
        self.options = ch.get_content_names();
        self.selected_index = ch.get_selected_index();
    }

    fn render<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        let messages = List::new(
                self.options.iter()
                .enumerate()
                .map(|(i, m)| {
                    let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
                    ListItem::new(content).style(Style::default().fg(Color::Cyan))
                })
                .collect::<Vec<_>>()
        )
        .block(Block::default().borders(Borders::ALL).title("Browser Widget"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Rgb(200, 100, 0)))
        // .highlight_symbol("> ")
        ;
        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));
        f.render_stateful_widget(messages, r, &mut list_state);
    }
}

struct PlayerWidget {
    render_state: RenderState,
}
#[derive(Clone)]
enum RenderState {
    Log(Vec<String>),
    Normal,
}

impl PlayerWidget {
    fn new() -> Self {
        Self {
            render_state: RenderState::Normal
        }
    }

    fn handle_events(&mut self, key: KeyEvent, ch: &mut ContentHandler) -> bool {
        match key.code {
            KeyCode::Char('p') => {
                ch.toggle_song_pause();
            }
            KeyCode::Char('k') => {
                ch.seek_song(10.0);
            }
            KeyCode::Char('j') => {
                ch.seek_song(-10.0);
            }
            KeyCode::Char('l') => {
                ch.next_song();
            }
            KeyCode::Char('h') => {
                ch.prev_song();
            }
            _ => return false,
        }
        true
    }

    fn render<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        match self.render_state.clone() {
            RenderState::Normal => {
                f.render_widget(
                    Block::default()
                        .borders(Borders::ALL).title("Player Widget"),
                    r
                );
            }
            RenderState::Log(logs) => {
                let messages = List::new(
                    logs.iter()
                    .enumerate()
                    .map(|(i, m)| {
                        let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
                        ListItem::new(content).style(Style::default().fg(Color::Cyan))
                    })
                    .collect::<Vec<_>>()
                )
                .block(Block::default().borders(Borders::ALL).title("Browser Widget"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Rgb(200, 100, 0)))
                // .highlight_symbol("> ")
                ;
                let mut list_state = ListState::default();
                f.render_stateful_widget(messages, r, &mut list_state);
            }
        }
    }

    fn update(&mut self, ch: &mut ContentHandler) {
        if ch.player.is_finished() {
            ch.next_song();
        }
        match &mut self.render_state {
            RenderState::Log(logs) => {
                logs.clear();
                logs.append(&mut ch.get_logs().clone().into_iter().rev().collect());
            }
            RenderState::Normal => {

            }
        }
    }
}

struct StatusBar {}
impl StatusBar {
    fn render<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        let (msg, style) = (vec![
            Spans::from(vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ]),
            // Spans::from(vec![
            //     Span::raw("lol"),
            // ]),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        );
        let mut text = Text::from(msg);
        text.patch_style(style);
        let help_message = Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().bg(Color::White).fg(Color::Black));
        f.render_widget(help_message, r);    }
}


#[derive(Clone, Copy)]
enum AppState {
    Browser,
    Help,
    Quit,
    Typing,
}

pub struct App {
    /// Current value of the input box
    input: Vec<char>,
    input_cursor_pos: usize,
    state: AppState,

    // updates status bar depending on the situation
    status_bar: StatusBar,
    // handles all ui things from the browser widget side
    browser_widget: BrowserWidget,
    // handles all ui from the player widget side
    player_widget: PlayerWidget,

    content_handler: ContentHandler,
}

impl App {
    pub fn load() -> Self {
        Self {
            input: Default::default(),
            input_cursor_pos: 0,
            state: AppState::Browser,

            status_bar: StatusBar {},
            browser_widget: BrowserWidget::new(),
            player_widget: PlayerWidget::new(),

            content_handler: ContentHandler::load(),
        }
    }

    pub fn run_app<B: Backend>(mut self, terminal: &mut Terminal<B>) -> Result<()> {
        self.browser_widget.update(&mut self.content_handler);
        loop {
            terminal.draw(|f| self.render(f))?;
            self.handle_events()?;
            self.update();

            if let AppState::Quit = self.state {
                return Ok(());
            }
        }
    }

    fn update(&mut self) {
        self.player_widget.update(&mut self.content_handler);
        let action = self.content_handler.get_app_action();
        action.apply(self)
    }
    
    fn handle_events(&mut self) -> Result<()> {
        if !event::poll(std::time::Duration::from_millis(500))? { // read does not block as poll returned true
            return Ok(())
        }
        if let Event::Key(key) = event::read()? {
            let event_handled = match self.state {
                AppState::Typing => {
                    let mut event_handled = true;
                    match key.code {
                        KeyCode::Esc => {
                            self.state = AppState::Browser; // TODO: should this be a stack too?
                            self.content_handler.back();
                            self.browser_widget.update(&mut self.content_handler);
                        }
                        KeyCode::Char(c) => {
                            self.input.insert(self.input_cursor_pos, c);
                            self.input_cursor_pos += 1;
                        }
                        KeyCode::Backspace => {
                            self.input.remove(self.input_cursor_pos -1);
                        }
                        KeyCode::Left => {
                            todo!();
                        }
                        KeyCode::Right => {
                            todo!();
                        }
                        KeyCode::Up => {
                            todo!();
                        }
                        KeyCode::Down => {
                            todo!();
                        }
                        KeyCode::PageUp => {
                            todo!();
                        }
                        KeyCode::PageDown => {
                            todo!();
                        }
                        KeyCode::Home => {
                            todo!();
                        }
                        KeyCode::End => {
                            todo!();
                        }
                        KeyCode::Enter => {
                            todo!();
                        }
                        _ => event_handled = false,
                    }
                    event_handled
                },
                _ => {
                    let mut event_handled = false;
                    if !event_handled {
                        event_handled = self.browser_widget.handle_events(key, &mut self.content_handler);
                        // if event_handled {
                        //     self.browser_widget.update(&mut self.content_handler);
                        // }
                    }
                    if !event_handled {event_handled = self.player_widget.handle_events(key, &mut self.content_handler);}
                    event_handled
                },
            };
            if event_handled {return Ok(())}
        
            match self.state {
                _ => match key.code {
                    KeyCode::Char('q') => {
                        self.state = AppState::Quit;
                    }
                    _ => ()
                }
            }
        }
        Ok(())
    }
    
    fn render<B: Backend>(&self, f: &mut Frame<B>) {
        let (status_rect, right_rect, left_rect) = {
            let mut chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(0),
                ].as_ref())
                .split(f.size());
            let mut lower_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 2),
                    Constraint::Ratio(1, 2),
                ].as_ref())
                .split(chunks.pop().unwrap());
            (chunks.pop().unwrap(), lower_chunks.pop().unwrap(), lower_chunks.pop().unwrap())
        };

        self.status_bar.render(f, status_rect);
        self.player_widget.render(f, right_rect);
        self.browser_widget.render(f, left_rect);
    }
}
