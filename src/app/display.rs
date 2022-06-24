
use std::borrow::Cow;
use tui::{
    style::{
        Style,
    },
    widgets::{
        Block,
        Borders,
        BorderType,
        List,
        ListItem,
    },
    text::{
        Span,
        Spans,
        Text,
    },
    layout::Rect,
};

/* // TODO:
how do i choose how the song is printed from the cp???
  . the song should not show album name if the content provider itself is a album for example
plan:
  . have a trait for songs DisplaySong with functions for querying artist and stuff. returns Option<string>
    . and have a callback func that prints songs
    . or have the songs sent to the provider in associated types
*/

pub type ReplaceSelectedTextCallback<'a> = Box<dyn FnOnce(Text<'a>) -> Text<'a>>;

pub struct ReplaceSelectedTextListBuilder<'a, 'b> {
    builder: &'b ListBuilder<'a>,
    callback: ReplaceSelectedTextCallback<'a>,
}
impl<'a, 'b> ReplaceSelectedTextListBuilder<'a, 'b> {
    pub fn list(self, rect: Rect, selected_index: usize) -> List<'a> {
        let mut items = self.builder.texts(rect, selected_index);
        
        if let Some(t) = items.get_mut(selected_index) {
            *t = (self.callback)(t.clone());
        }

        let items = items
        .into_iter()
        .map(ListItem::new)
        .collect::<Vec<_>>();
        
        let mut list = List::new(items);
        
        if self.builder.block.is_some() {
            list = list.block(self.builder.block.as_ref().unwrap().clone())
        }
        
        list
    }
}

#[derive(Default, Debug)]
pub struct ListBuilder<'a> {
    pub items: Vec<Item<'a>>,
    numbered: bool,
    title: Option<Spans<'a>>,
    block: Option<Block<'a>>,
}
impl<'a> ListBuilder<'a> {
    pub fn list<'c>(&'c self, rect: Rect, selected_index: usize) -> List<'a> {

        let items = self.texts(rect, selected_index);

        let items = items
        .into_iter()
        .map(ListItem::new)
        .collect::<Vec<_>>();

        let mut list = List::new(items);
        if self.block.is_some() {
            list = list.block(self.block.as_ref().unwrap().clone())
        }
        list
    }
    fn texts<'c>(&'c self, rect: Rect, selected_index: usize) -> Vec<Text<'a>> {
        let rect = self.get_inner_rect(rect);

        let items = self.items
        .iter()
        .enumerate()
        .map(|(index, item)| (index == selected_index, item))
        .map(|(selected, item)| item.text(rect.width, selected))
        .collect::<Vec<_>>();

        items
    }
    pub fn replace_selected<'b>(&'b self, callback: ReplaceSelectedTextCallback<'a>) -> ReplaceSelectedTextListBuilder<'a, 'b> {
        ReplaceSelectedTextListBuilder { builder: self, callback }
    }
    pub fn title<'b: 'c + 'a, 'c, T: Into<Spans<'b>>>(&'c mut self, title: T) -> &mut Self {
        self.title = Some(title.into());
        let block = self.block
        .take()
        .unwrap_or(
            Block::default()
            .borders(Borders::all())
            .border_type(BorderType::Rounded)
        );
        self.block(block);
        self
    }
    pub fn block<'b: 'c + 'a, 'c>(&'c mut self, block: Block<'b>) -> &mut Self {
        if self.title.is_some() {
            let title = self.title
            .as_ref()
            .unwrap()
            .clone();
            let block = block.title(title);
            self.block = Some(block)
        } else {
            self.block = Some(block);
        }
        self
    }
    pub fn set_numbered(&mut self, numbered: bool) -> &mut Self {
        self.numbered = numbered;
        self
    }
    pub fn get_inner_rect(&self, r: Rect) -> Rect {
        self.block
        .as_ref()
        .map(|b| Block::inner(b, r))
        .unwrap_or(r)
    }
    pub fn get_abs_pos(&self, rect: Rect, selected_index: usize) -> Pos {
        Pos {
            text_width: self.items[selected_index].text[0].main_text.width().try_into().unwrap(),
            serial_number_width: if self.numbered {format!("{}", self.items.len()).len().try_into().unwrap()} else {0},
            inner_rect: self.get_inner_rect(rect),
        }
    }
}

#[derive(Debug)]
pub struct Pos {
    pub text_width: u16,
    pub serial_number_width: u16,
    pub inner_rect: Rect,
}

/// mark a item with some symbol/style/color to indicate something
#[derive(Debug, Clone)]
pub struct Marker<'a> {
    pub symbol: Span<'a>,
    pub pos: MarkerPos,
    // pub item_style: Option<Style>, // priority where?
}


#[derive(Debug, Clone)]
pub struct Item<'a> {
    pub text: Vec<Line<'a>>,
    pub selected_text: SelectedText<'a>,
}
impl<'a> Item<'a> {
    fn text(&self, width: u16, selected: bool) -> Text<'a> {
        if selected {
            match &self.selected_text {
                SelectedText::Style(styles) => {
                    let mut spans: Text = self.text
                    .iter()
                    .map(|l| l.spans(width))
                    .collect::<Vec<_>>()
                    .into()
                    ;
                    spans.patch_style(styles.clone());
                    spans
                }
                SelectedText::Lines(text) => {
                    text
                    .iter()
                    .map(|l| l.spans(width))
                    .collect::<Vec<_>>()
                    .into()
                }
            }
        } else {
            self.text
            .iter()
            .map(|l| l.spans(width))
            .collect::<Vec<_>>()
            .into()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Line<'a> {
    main_text: Spans<'a>, // spans is a single line in tui::text::Text
    secondary_text: Option<Spans<'a>>,
    main_text_alignment: Alignment, // the alignment of secondary text should be the opposite to that of main text
    markers: Vec<Marker<'a>>,    
}
impl<'a> Line<'a> {
    pub fn new<T: Into<Spans<'a>>>(main_text: T) -> Self {
        Self {
            main_text: main_text.into(),
            markers: Default::default(),
            main_text_alignment: Default::default(),
            secondary_text: Default::default(),
        }
    }

    fn spans(&self, width: u16) -> Spans<'a> {
        match self.main_text_alignment {
            Alignment::Left => {
                for marker in self.markers.iter() {
                    todo!()
                }
                match self.secondary_text {
                    Some(_) => todo!(),
                    None => {
                        self.main_text.clone()
                    }
                }
            }
            Alignment::Centered => todo!(),
            Alignment::Right => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SelectedText<'a> {
    Style(Style), // overrides all styles to this
    Lines(Vec<Line<'a>>)
}

#[derive(Debug, Clone)]
pub enum Alignment {
    Centered,
    Left,
    Right,
    // Top,
    // Bottom,
}
impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Debug, Clone)]
pub enum MarkerPos {
    Left,
    Right,
}



pub trait Display<'b> {
    type DisplayContext;
    fn display(&self, _context: Self::DisplayContext) -> ListBuilder<'static>;
    fn get_name(&self) -> Cow<'static, str>;
}

