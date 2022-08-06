

use tui::{
    style::{
        Color,
        Style,
    },
    text::{
        Span,
    },
};

use crate::{
    content::{
        register::{
            ContentProviderID,
            SongID,
            ContentRegister,
            ID,
        },
        song::Song,
        providers::ContentProvider,
        stack::StateContext,
    },
    service::editors::Yanker,
    app::{
        display::{
            Item,
            Marker,
            MarkerPos,
            Line,
            SelectedText,
        },
    },
};


pub struct DisplayContext<'a> {
    pub state: DisplayState<'a>,
    pub songs: &'a ContentRegister<Song, SongID>,
    pub providers: &'a ContentRegister<ContentProvider, ContentProviderID>,
    pub yanker: Option<&'a Yanker>,
}

impl<'a> DisplayContext<'a> {
    pub fn display_item(&self, id: ID) -> Item<'static> {
        let style = Style::default().fg(Color::Rgb(200, 200, 0));
        match id {
            ID::Song(id) => {
                let song = self.songs.get(id).unwrap();
                let title = song.as_display().title();
                let line = Line::new(Span::from(title.to_owned()));
                let mut selected_line = line.clone();
                selected_line.overwrite_style(style);
                Item {
                    text: vec![self.apply_markers(line, id, MarkerPos::Left)],
                    selected_text: SelectedText::Lines(vec![self.apply_markers(selected_line, id, MarkerPos::Left)]),
                }
            }
            ID::ContentProvider(id) => {
                let name = self.providers
                .get(id)
                .unwrap()
                .as_display()
                .get_name();
                let line = Line::new(Span {content: name, style: Default::default()});
                let mut selected_line = line.clone();
                selected_line.overwrite_style(style);
                Item {
                    text: vec![self.apply_markers(line, id, MarkerPos::Left)],
                    selected_text: SelectedText::Lines(vec![self.apply_markers(selected_line, id, MarkerPos::Left)]),
                }
            }
        }
    }

    pub fn apply_markers<'b, T: Into<ID>>(&self, mut line: Line<'b>, id: T, pos: MarkerPos) -> Line<'b> {
        let id = id.into();
        if self.yanker
        .as_ref()
        .map(|y| y.yanked_items.iter().cloned().map(|(i, _)| i).collect::<Vec<_>>().contains(&id))
        .unwrap_or(false)
        {
            let marker = Marker {symbol: Yanker::marker_symbol(), pos};
            line.markers.push(marker);
        }
        line
    }
}


// BAD: this again introduces the problem that a state with edit can be passed to a provider without edit
pub enum DisplayState<'a> {
    Normal,
    Menu(&'a StateContext),
    Edit(&'a StateContext),
}