
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
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
        KeyModifiers,
    },
};
// use unicode_width::UnicodeWidthStr; // string.width() -> gives correct width (including cjk chars) (i assume)
use anyhow::Result;

use crate::{
    content::manager::{
        manager::ContentManager,
        action::ContentManagerAction,
    },
    app::action::AppAction,
};


/// wrapping ListState to make sure not to call select(None) and to eliminate the use of unwrap() on selected_index()
/// currently, theres no way to access/suggest the offset
#[derive(Debug, Clone)]
pub struct SelectedIndex {
    index: ListState,
}
impl Default for SelectedIndex {
    fn default() -> Self {
        Self::new()
    }
}
impl Into<ListState> for SelectedIndex {
    fn into(self) -> ListState {
        self.index
    }
}
impl<'a> Into<&'a mut ListState> for &'a mut SelectedIndex {
    fn into(self) -> &'a mut ListState {
        &mut self.index
    }
}
impl SelectedIndex {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { index: state }
    }

    pub fn selected_index(&self) -> usize {
        self.index.selected().unwrap()
    }

    pub fn select(&mut self, index: usize) {
        self.index.select(Some(index));
    }
}

#[derive(Default)]
pub struct BrowserWidget {
    pub options: Vec<String>,
}

impl BrowserWidget {
    fn new() -> Self {
        Self::default()
    }

    fn handle_events(&mut self, key: KeyEvent, ch: &mut ContentManager) -> Result<bool> {
        match key.code {
            KeyCode::Char('d') => {
                ch.debug_current();
            }
            KeyCode::Char('g') => {
                // self.options = ch.menu_for_selected();
            }
            KeyCode::Char('G') => {
                ch.open_menu_for_current()?;
            }
            KeyCode::Char('E') => {
                ch.open_edit_for_current()?;
            }
            KeyCode::Up => {
                ch.decrement_selection();
            }
            KeyCode::Down => {
                ch.increment_selection();
            }
            KeyCode::Right => {
                ch.enter_selected()?;
            }
            KeyCode::Left => {
                ContentManagerAction::PopContentStack.apply(ch)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn render<B: Backend>(&self, f: &mut Frame<B>, r: Rect, selected_index: &mut SelectedIndex) {
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
        f.render_stateful_widget(messages, r, selected_index.into());
    }
}

struct PlayerWidget {
    render_state: RenderState,
}
#[derive(Clone)]
enum RenderState {
    Normal,
}

impl PlayerWidget {
    fn new() -> Self {
        Self {
            render_state: RenderState::Normal
        }
    }

    fn handle_events(&mut self, key: KeyEvent, ch: &mut ContentManager) -> Result<bool> {
        match key.code {
            KeyCode::Char('p') => {
                ch.toggle_song_pause();
            }
            KeyCode::Char('k') => {
                ch.seek_song(10.0)?;
            }
            KeyCode::Char('j') => {
                ch.seek_song(-10.0)?;
            }
            KeyCode::Char('l') => {
                ch.next_song()?;
            }
            KeyCode::Char('h') => {
                ch.prev_song()?;
            }
            _ => return Ok(false),
        }
        Ok(true)
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
        }
    }

    fn update(&mut self, ch: &mut ContentManager) -> Result<()> {
        ch.poll_action()?;
        if ch.player.is_finished()? {
            ch.next_song()?;
        }
        match &mut self.render_state {
            RenderState::Normal => {

            }
        }
        Ok(())
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
pub enum AppState {
    Browser,
    Help,
    Quit,
    Typing,
}

pub struct App {
    /// Current value of the input box
    pub input: Vec<char>,
    pub input_cursor_pos: usize,
    pub typing_callback: AppAction,
    pub state: AppState,

    // updates status bar depending on the situation
    status_bar: StatusBar,
    // handles all ui things from the browser widget side
    pub browser_widget: BrowserWidget,
    // handles all ui from the player widget side
    player_widget: PlayerWidget,

    pub content_manager: ContentManager,
    pub redraw_needed: bool,
}

impl App {
    pub fn load() -> Result<Self> {
        let a = Self {
            input: Default::default(),
            input_cursor_pos: 0,
            state: AppState::Browser,
            typing_callback: AppAction::None,

            status_bar: StatusBar {},
            browser_widget: BrowserWidget::new(),
            player_widget: PlayerWidget::new(),

            content_manager: ContentManager::load()?,
            redraw_needed: false,
        };
        Ok(a)
    }

    pub fn run_app<B: Backend>(mut self, terminal: &mut Terminal<B>) -> Result<()> {
        terminal.draw(|f| self.render(f).unwrap())?;
        loop {
            self.handle_events()?;
            self.update()?;
            if self.redraw_needed {
                self.redraw_needed = false;
                dbg!("resized");
                terminal.resize(terminal.size()?)?;
            }
            terminal.draw(|f| self.render(f).unwrap())?;

            if let AppState::Quit = self.state {
                return Ok(());
            }
        }
    }

    fn update(&mut self) -> Result<()> {
        self.player_widget.update(&mut self.content_manager)?;
        let action = self.content_manager.get_app_action();
        action.apply(self)?;
        match self.state {
            AppState::Typing => {
                let index = self.content_manager.get_selected_index().selected_index();
                self.browser_widget.options[index] = self.input[..].iter().collect::<String>();
            }
            _ => (),
        }
        Ok(())
    }
    
    fn handle_events(&mut self) -> Result<()> {
        if !event::poll(std::time::Duration::from_millis(500))? { // read does not block as poll returned true
            return Ok(())
        }
        match event::read()? {
            Event::Key(key) => {
                let event_handled = match self.state {
                    AppState::Typing => {
                        let mut event_handled = true;
                        match key.code {
                            KeyCode::Esc => {
                                self.state = AppState::Browser; // TODO: should this be a stack too?
                                ContentManagerAction::PopContentStack.apply(&mut self.content_manager)?;
                            }
                            KeyCode::Char(c) => {
                                self.input.insert(self.input_cursor_pos, c);
                                self.input_cursor_pos += 1;
                            }
                            KeyCode::Backspace => {
                                if self.input_cursor_pos > 0 {
                                    self.input_cursor_pos -= 1;
                                    self.input.remove(self.input_cursor_pos);
                                }
                            }
                            KeyCode::Left => {
                                match key.modifiers { // these are bitfields, not enum variants
                                    KeyModifiers::NONE => {
                                        if self.input_cursor_pos > 0 {
                                            self.input_cursor_pos -= 1;
                                        }
                                    }
                                    // KeyModifiers::CONTROL | KeyModifiers::SHIFT => {}
                                    _ => event_handled = false,
                                }
                            }
                            KeyCode::Right => {
                                match key.modifiers {
                                    KeyModifiers::NONE => {
                                        if self.input_cursor_pos < self.input.len() {
                                            self.input_cursor_pos += 1;
                                        }
                                    }
                                    _ => event_handled = false,
                                }
                            }
                            KeyCode::Home => {
                                self.input_cursor_pos = 0;
                            }
                            KeyCode::End => {
                                self.input_cursor_pos = self.input.len();
                            }
                            KeyCode::Enter => {
                                let action = std::mem::replace(&mut self.typing_callback, AppAction::None);
                                action.apply(self)?;
                                self.state = AppState::Browser;
                            }
                            _ => event_handled = false,
                        }
                        event_handled
                    },
                    _ => {
                        let mut event_handled = false;
                        if !event_handled {
                            event_handled = self.browser_widget.handle_events(key, &mut self.content_manager)?;
                            // if event_handled {
                            //     self.browser_widget.update(&mut self.content_manager);
                            // }
                        }
                        if !event_handled {event_handled = self.player_widget.handle_events(key, &mut self.content_manager)?;}
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
            Event::Resize(_, _) => {
                self.content_manager.image_handler.dimensions_changed();
            }
            Event::Mouse(_) => (),
        }
        Ok(())
    }
    
    fn render<B: Backend>(&mut self, f: &mut Frame<B>) -> Result<()> {
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
        self.browser_widget.render(f, left_rect, self.content_manager.get_selected_index());
        
        self.content_manager.image_handler.set_offset(right_rect.x + 1, right_rect.y + 1);
        self.content_manager.image_handler.set_size(Some(right_rect.width as u32 - 2), Some(right_rect.height as u32 - 2));
        self.content_manager.image_handler.maybe_print()?;

        match self.state {
            AppState::Typing => {
                f.set_cursor(
                    4 + left_rect.x + self.input_cursor_pos as u16, // BAD: offset due to sr no. can't be known here
                    1 + left_rect.y + self.content_manager.get_selected_index().selected_index() as u16, // - offset?
                );
            }
            _ => (),
        }
        Ok(())
    }
}
