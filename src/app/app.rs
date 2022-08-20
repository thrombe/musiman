
#[allow(unused_imports)]
use crate::{
    dbg,
    debug,
    error,
};

use tokio::select;
use tui::{
    backend::Backend,
    widgets::{
        Block,
        Borders,
        Paragraph,
        ListState,
        List,
        ListItem,
        Gauge,
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
        EventStream,
        Event,
        KeyCode,
        KeyEvent,
        KeyModifiers,
    },
};
// use unicode_width::UnicodeWidthStr; // string.width() -> gives correct width (including cjk chars) (i assume)
use anyhow::Result;
use std::borrow::Cow;
use futures::{
    StreamExt,
    FutureExt,
};

use crate::{
    content::{
        manager::{
            manager::ContentManager,
            action::ContentManagerAction,
        },
        stack::ContentState,
    },
    app::{
        action::AppAction,
        display::{
            ListBuilder,
        },
    },
    service::editors::YankType,
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
    pub list_builder: ListBuilder<'static>,
}

impl BrowserWidget {
    fn new() -> Self {
        Self::default()
    }

    fn handle_events(&mut self, key: KeyEvent, ch: &mut ContentManager) -> Result<bool> {
        match key.code {
            KeyCode::Char('g') => {
                todo!()
            }
            KeyCode::Char('G') => {
                ch.open_menu_for_current()?;
            }
            KeyCode::Char('E') => {
                ch.open_edit_for_current()?;
            }
            KeyCode::Char('y') => {
                ch.toggle_yank_selected()?;
                ch.increment_selection();
            }
            KeyCode::Char('Y') => {
                ch.edit_manager.clear().apply(ch)?;
            }
            KeyCode::Char('X') => {
                if ch.edit_manager.yanker.is_none() {
                    ch.toggle_yank_selected()?;
                }
                let action = ch.edit_manager.apply_yank(YankType::Cut);
                if !action.apply(ch)? {
                    let _ = ch.edit_manager.yanker.take();
                    ContentManagerAction::RefreshDisplayContent.apply(ch)?;
                }
            }
            KeyCode::Char('C') => {
                let action = ch.edit_manager.apply_yank(YankType::Copy);
                action.apply(ch)?;
            }
            KeyCode::Char('v') => {
                if let None = ch.edit_manager.yanker { // necessary as pasting things where something is yanked from, desyncs the inidex in Yanker
                     if let ContentState::Normal = ch.content_stack.get_state() {
                         let index = ch.get_selected_index().selected_index();
                         let action = ch.edit_manager.try_paste(ch.content_stack.last(), Some(index));
                         action.apply(ch)?;
                     }
                }
             }
             KeyCode::Char('V') => {
                if let None = ch.edit_manager.yanker { // necessary as pasting things where something is yanked from, desyncs the inidex in Yanker
                     if let ContentState::Normal = ch.content_stack.get_state() {
                         let index = ch.get_selected_index().selected_index();
                         let action = ch.edit_manager.try_paste(ch.content_stack.last(), Some(index+1));
                         action.apply(ch)?;
                     }
                }
             }
            KeyCode::Char('Z') => {
                ch.edit_manager.redo_last_undo().apply(ch)?;
            }
            KeyCode::Char('z') => {
                ch.edit_manager.undo_last_edit().apply(ch)?;
            }
            KeyCode::Esc => {
                match ch.edit_manager.yanker.take() {
                    Some(_) => ContentManagerAction::RefreshDisplayContent.apply(ch)?,
                    None => return Ok(false),
                }
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

    fn render<'a, B: Backend>(&self, f: &mut Frame<B>, r: Rect, cm: &mut ContentManager, input: &[char], input_cursor_pos: usize, state: AppState) {
        let selected_index = cm.get_selected_index().selected_index();

        let list = if let AppState::Typing = state {
            let pos = self.list_builder.get_abs_pos(r, selected_index);
            let x = pos.inner_rect.x + pos.serial_number_width;
            if x as usize + input.len() < (pos.inner_rect.x + pos.inner_rect.width) as usize {
                // TODO: show the right side of the text when typing if it dosent fit. like  "..olling<cursor>"
                f.set_cursor(
                    x + input_cursor_pos as u16,
                    // BAD: cannot figure out the exact y of the cursor (will need ListState.offset)
                    pos.inner_rect.y
                    .checked_add(selected_index.try_into().unwrap())
                    .unwrap()
                );
            }

            let input = input.iter().collect::<String>();
            let callback = move |mut text: Text<'a>| -> Text<'a> {
                text.lines[0].0[0].content = Cow::from(input);
                text
            };
            

            let callback = Box::new(callback);
            self.list_builder.replace_selected(callback).list(r, selected_index)
        } else {
            self.list_builder.list(r, selected_index)
        };
        f.render_stateful_widget(list, r, cm.get_selected_index().into());
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

    fn render<B: Backend>(&self, f: &mut Frame<B>, r: Rect, cm: &mut ContentManager) -> Result<()> {
        let block = Block::default().borders(Borders::ALL);
        let inner_rect = block.inner(r);

        if let Some(song_id) = cm.active_song {
            let song = cm.get_song(song_id).as_display();
            // TODO: center align the info
            let song_info = [
                Some(format!("title: {title}", title = song.title())),
                song.artist().map(|artist| format!("artist: {artist}")),
                song.album().map(|album| format!("album: {album}")),
            ].into_iter()
            .filter_map(|i| i)
            .map(Span::raw)
            .map(Spans::from)
            .map(ListItem::new)
            .collect::<Vec<_>>();

            let (image_rect, song_info_rect, song_progress_rect) = {
                let mut rects = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(
                        song_info
                        .len()
                        .try_into()
                        .ok()
                        .map(|a: u16| a.checked_add(1).unwrap()) // +1 for progress bar
                        .unwrap()
                    )
                ].as_ref())
                .split(inner_rect)
                .into_iter();

                let mut song_info_rects = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
                .split(rects.next_back().unwrap())
                .into_iter();
                
                (
                    rects.next().unwrap(),
                    song_info_rects.next().unwrap(),
                    song_info_rects.next().unwrap(),
                )
            };

            // render the progress bar
            let gauge = Gauge::default()
            .ratio(cm.player.progress()?)
            .gauge_style(Style::default().fg(Color::Cyan))
            // .style(Style::default().fg(Color::LightGreen)) // label style
            //? maybe show the time too? "<progress>/<duration>"
            .label(""); // this disables the default label of percentage
            f.render_widget(gauge, song_progress_rect);
    
            // render the image
            cm.image_handler.set_offset(image_rect.x, image_rect.y);
            cm.image_handler.set_size(Some(image_rect.width as u32), Some(image_rect.height as u32));
            cm.image_handler.maybe_print()?;    

            let song_info = List::new(song_info);
            f.render_widget(song_info, song_info_rect);
        }



        // tui::widgets::LineGauge/tui::widgets::Gauge for progress bar
        match self.render_state.clone() {
            RenderState::Normal => {
                f.render_widget(
                    block.title("Player Widget"),
                    r
                );
            }
        }
        Ok(())
    }

    fn update(&mut self, ch: &mut ContentManager) -> Result<()> {
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
    DbgInput,
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

            content_manager: ContentManager::try_load()?
            .unwrap_or(ContentManager::new()?),
            redraw_needed: false,
        };
        Ok(a)
    }

    pub async fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        AppAction::UpdateDisplayContent.apply(self)?;
        terminal.draw(|f| self.render(f).unwrap())?;
        let mut reader = EventStream::new();
        #[cfg(feature = "sixel")]
        let _ = crate::image::printer::sixel::is_sixel_supported(); // for some reason this does not behave well when tokio does its stuff (maybe cus Write on stdout). so cache it (lazy_static)
        loop {
            let event = reader.next().fuse();
            let action = self.content_manager.parallel_handle.recv();
            let app_action = self.content_manager.app_action_receiver.recv();
            let sleep = tokio::time::sleep(std::time::Duration::from_secs_f64(0.5));
            select! {
                ev = event => self.handle_events(ev.unwrap()?)?,
                action = action => action.apply(&mut self.content_manager)?,
                app_action = app_action => app_action.unwrap().apply(self)?,
                _ = sleep => (),
            }
            let _ = self.content_manager.app_action_receiver // to make sure not to render without updating content
            .try_recv()
            .map(|e| e.apply(self));
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
        Ok(())
    }
    
    fn handle_events(&mut self, event: Event) -> Result<()> {
        match event {
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
                    }
                    AppState::DbgInput => {
                        match key.code {
                            KeyCode::Char(c) => {
                                self.content_manager.debug_current(c);
                                debug!("debug print end -- ---- -----");
                                self.state = AppState::Browser;
                                true
                            }
                            _ => {
                                false
                            },
                        }
                    }
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
                        KeyCode::Char('d') => {
                            self.state = AppState::DbgInput;
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
        self.player_widget.render(f, right_rect, &mut self.content_manager)?;
        self.browser_widget.render(f, left_rect, &mut self.content_manager, &self.input, self.input_cursor_pos, self.state);
        
        Ok(())
    }
}
