


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
// use unicode_width::UnicodeWidthStr; // string.width() -> gives correct width (including cjk chars) (i assume)
use anyhow::Result;

use crate::{
    content_handler::ContentHandler,
};


pub struct App {
    /// Current value of the input box
    input: String,
    state: AppState,

    status_bar: StatusBar,
    browser_widget: BrowserWidget,
    player_widget: PlayerWidget,

    content_handler: ContentHandler,
}


#[derive(Clone, Copy)]
enum AppState {
    Browser,
    Popup,
    Help,
    Menu,
    Quit,
    Typing,
}

#[derive(Default)]
struct BrowserWidget {
    options: Vec<String>,
    selected_index: usize,
    top_index: usize,
}

impl BrowserWidget {
    fn new() -> Self {
        // Self {
        //     options: vec![],
        // }
        Self::default()
    }

    fn handle_events(&mut self, key: KeyEvent, ch: &mut ContentHandler) -> bool {
        match key.code {
            KeyCode::Char('g') => {
                // self.options = ch.menu_for_selected();
            }
            KeyCode::Up => {
                if self.selected_index > 0 {self.selected_index -= 1};
            }
            KeyCode::Down => {
                if self.selected_index < self.options.len()-1 {self.selected_index += 1};
            }
            KeyCode::Right => {
                ch.enter(self.selected_index+self.top_index);
                self.update(ch);
            }
            _ => return false,
        }
        true
    }

    fn update(&mut self, ch: &mut ContentHandler) {
        self.options = ch.get_content_names();
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

struct PlayerWidget {}

impl PlayerWidget {
    fn handle_events(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('p') => {
                true
            },
            _ => false,
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



impl App {
    pub fn load() -> Self {
        Self {
            input: String::new(),
            state: AppState::Browser,

            status_bar: StatusBar {},
            browser_widget: BrowserWidget::new(),
            player_widget: PlayerWidget {},

            content_handler: ContentHandler::load(),
        }
    }

    pub fn run_app<B: Backend>(mut self, terminal: &mut Terminal<B>) -> Result<()> {
        self.browser_widget.update(&mut self.content_handler);
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
                    if !event_handled {
                        event_handled = self.browser_widget.handle_events(key, &mut self.content_handler);
                        // if event_handled {
                        //     self.browser_widget.update(&mut self.content_handler);
                        // }
                    }
                    if !event_handled {event_handled = self.player_widget.handle_events(key);}
                    event_handled
                },
            };
            if handled {return Ok(())}

            match self.state {
                AppState::Browser => match key.code {
                    KeyCode::Char('q') => {
                        self.state = AppState::Quit;
                    }
                    _ => {}
                },
                _ => (),
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
