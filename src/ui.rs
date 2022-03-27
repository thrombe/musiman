


use tui::{
    backend::{Backend},
    widgets::{Block, Borders, List, ListItem, Paragraph, ListState},
    layout::{Layout, Constraint, Direction, Alignment, Rect},
    Terminal, Frame,
    style::{Color, Style, Modifier},
    text::{Span, Spans, Text},
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
};
use unicode_width::UnicodeWidthStr;
use anyhow::Result;

use crate::{
    content_handler::ContentHandler,
};


pub struct App {
    /// Current value of the input box
    input: String,
    messages: Vec<String>,
    input_mode: InputMode,
    state: AppState,

    status_bar: StatusBar,
    browser_widget: BrowserWidget,
    player_widget: PlayerWidget,

    content_handler: ContentHandler,
}


#[derive(Clone, Copy)]
enum InputMode {
    Normal,
    Editing,
}

enum AppState {
    Browser,
    Popup,
    Help,
    Menu,
    Quit,
}

#[derive(Default)]
struct BrowserWidget {
    options: Vec<String>,
    selected_index: usize,
}

impl BrowserWidget {
    fn new() -> Self {
        // Self {
        //     options: vec![],
        // }
        Self::default()
    }

    fn handle_events(&mut self, key: KeyEvent, input_mode: &mut InputMode, ch: &mut ContentHandler) -> bool {
        match input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('g') => {
                    // self.options = ch.menu_for_selected();
                },
                _ => return false,
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => {
                    *input_mode = InputMode::Normal;
                }
                KeyCode::Up => {
                    if self.selected_index > 0 {self.selected_index -= 1};
                }
                KeyCode::Down => {
                    if self.selected_index < self.options.len() {self.selected_index += 1};
                }
                // KeyCode::Enter => {
                //     self.messages.push(self.input.drain(..).collect());
                // }
                // KeyCode::Char(c) => {
                //     self.input.push(c);
                // }
                // KeyCode::Backspace => {
                //     self.input.pop();
                // }
                _ => return false,
            },
        }
        true
    }

    fn render<B: Backend>(&self, f: &mut Frame<B>, r: Rect, input_mode: InputMode) {
        match input_mode {
            InputMode::Normal =>
                // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                {}
            InputMode::Editing => {
                // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
                f.set_cursor(
                    // Put cursor past the end of the input text
                    r.x,// + self.input.width() as u16 + 1 + 3,
                    // Move one line down, from the border to the input line
                    r.y + 1,
                )
            }
        }

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
        .style(match input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        // .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Rgb(200, 100, 0)))
        // .highlight_symbol("> ")
        ;
        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));
        f.render_stateful_widget(messages, r, &mut list_state);
    }
}

struct PlayerWidget {}

impl PlayerWidget {
    fn handle_events(&mut self, key: KeyEvent, input_mode: InputMode) -> bool {
        match input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('p') => {
                    true
                },
                _ => false,
            },
            InputMode::Editing => (
                false
            ),
        }
    }

    fn render<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        f.render_widget(
            Block::default()
                .borders(Borders::ALL).title("Player Widget"),
            r
        );
    }
}

struct StatusBar {}
impl StatusBar {
    fn render<B: Backend>(&self, f: &mut Frame<B>, r: Rect, input_mode: InputMode) {
        let (msg, style) = match input_mode {
            InputMode::Normal => (vec![
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
            ),
            InputMode::Editing => (vec![
                Spans::from(vec![
                    Span::raw("Press "),
                    Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to stop editing, "),
                    Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to record the message"),
                ])],
                Style::default(),
            ),
        };
        let mut text = Text::from(msg);
        text.patch_style(style);
        let help_message = Paragraph::new(text).alignment(Alignment::Center).style(Style::default().bg(Color::White).fg(Color::Black));
        f.render_widget(help_message, r);    }
}



impl App {
    pub fn load() -> Self {
        Self {
            messages: vec![],
            input: String::new(),
            input_mode: InputMode::Normal,
            state: AppState::Browser,

            status_bar: StatusBar {},
            browser_widget: BrowserWidget::new(),
            player_widget: PlayerWidget {},

            content_handler: ContentHandler::load(),
        }
    }

    pub fn run_app<B: Backend>(mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;
            self.handle_events()?;

            if let AppState::Quit = self.state {
                return Ok(());
            }
        }
    }
    
    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            let handled = match self.state {
                // AppState::Popup => {},
                _ => {
                    let mut event_handled = false;
                    if !event_handled {event_handled = self.browser_widget.handle_events(key, &mut self.input_mode, &mut self.content_handler);}
                    if !event_handled {event_handled = self.player_widget.handle_events(key, self.input_mode);}
                    event_handled
                },
            };
            if handled {return Ok(())}

            match self.input_mode {
                InputMode::Normal => match self.state {
                    AppState::Browser => match key.code {
                        KeyCode::Char('e') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            self.state = AppState::Quit;
                        }
                        _ => {}
                    },
                    _ => (),
                },
                InputMode::Editing => match key.code {
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                    }
                    _ => (),
                },
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

        self.status_bar.render(f, status_rect, self.input_mode);
        self.player_widget.render(f, right_rect);
        self.browser_widget.render(f, left_rect, self.input_mode);
    }
}
