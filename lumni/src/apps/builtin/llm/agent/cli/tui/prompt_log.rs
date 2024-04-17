use hmac::digest::generic_array::typenum::Length;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, ScrollbarState};
use textwrap::{wrap, Options, WordSplitter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptRect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl PromptRect {
    pub fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn update(&mut self, area: &Rect) -> bool {
        // adjust widget area for borders
        // return true if updated, else false
        let previous = *self;   // copy current state

        self.x = area.x;
        self.y = area.y;
        self.width = area.width - 2;
        self.height = area.height - 2;

        if *self != previous {
            true
        } else {
            false
        }
    }
}

pub struct PromptLogWindow<'a> {
    raw_text: String,   // text as received
    display_text: Vec<Line<'a>>,  // text processed for display
    area: PromptRect,
    vertical_scroll: usize,
    vertical_scroll_state: ScrollbarState,
}

impl PromptLogWindow<'_> {
    pub fn new() -> Self {
        Self {
            raw_text: String::new(),
            display_text: Vec::new(),
            area: PromptRect::default(),
            vertical_scroll: 0,
            vertical_scroll_state: ScrollbarState::default(),
        }
    }
    
    pub fn vertical_scroll_state(&mut self) -> &mut ScrollbarState {
        &mut self.vertical_scroll_state
    }

    pub fn scroll_down(&mut self) {
        let content_length = self.content_length();
        let area_height = self.area.height as usize;
        let end_scroll = content_length.saturating_sub(area_height);
        if content_length > area_height {
            // scrolling enabled when content length exceeds area height
            if self.vertical_scroll + 10 <= end_scroll {
                self.vertical_scroll += 10;
            } else {
                self.vertical_scroll = end_scroll;
            }
            self.update_scroll_state();
        }
    }

    pub fn scroll_up(&mut self) {
        if self.vertical_scroll == 0 {
            return;
        }
        self.vertical_scroll = self.vertical_scroll.saturating_sub(10);
        self.update_scroll_state();
    }


    pub fn update_scroll_state(&mut self) {
        let display_length = self.content_length().saturating_sub(self.area.height as usize);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(display_length)
            .viewport_content_length(self.area.height.into())
            .position(self.vertical_scroll);
    }

    fn update_display_text(&mut self) -> () {
        let display_width = self.area.width as usize;
        let processed_text = self.raw_text
            .split('\n')
            .flat_map(|line| {
                wrap(
                    line,
                    Options::new(display_width)
                        .word_splitter(WordSplitter::NoHyphenation),
                )
                .into_iter()
                .map(|cow_str| Line::from(Span::from(cow_str.to_string())))
                .collect::<Vec<Line>>()
            })
            .collect();
        self.display_text = processed_text;
    }

    fn content_length(&self) -> usize {
        self.display_text.len()
    }

    pub fn widget(&mut self, area: &Rect) -> Paragraph {
        if self.area.update(area) == true {
            // re-fit text to updated display
            self.update_display_text();    
        }

        Paragraph::new(Text::from(self.display_text.clone()))
            .block(Block::default().title("Paragraph").borders(Borders::ALL))
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .scroll((self.vertical_scroll as u16, 0))
    }

    pub fn insert_str(&mut self, text: &str) {
        self.raw_text.push_str(text);
        self.update_display_text();

        let length = self.content_length();
        let height = self.area.height as usize;
        self.vertical_scroll = if length > height { length - height } else { 0 };
        self.update_scroll_state();
    }
}
