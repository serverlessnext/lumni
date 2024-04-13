use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Paragraph, ScrollbarState,
};
use textwrap::{wrap, Options, WordSplitter};


pub struct PromptLogWindow {
    text: String,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

impl PromptLogWindow {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            vertical_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
        }
    }

    pub fn vertical_scroll_state(&mut self) -> &mut ScrollbarState {
        &mut self.vertical_scroll_state
    }

    /// Updates the scroll state based on the current size of the terminal.
    pub fn update_scroll_state(&mut self, size: &Rect) {
        let height = (size.height - 2) as usize;
        let lines = self.process_text(&size).len();
        self.vertical_scroll = if lines > height { lines - height } else { 0 };

        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(self.vertical_scroll)
            .viewport_content_length(height)
            .position(self.vertical_scroll + height);
    }

    fn process_text(&self, size: &Rect) -> Vec<Line> {
        self.text
            .split('\n')
            .flat_map(|line| {
                wrap(
                    line,
                    Options::new((size.width - 2) as usize)
                        .word_splitter(WordSplitter::NoHyphenation),
                )
                .into_iter()
                .map(|cow_str| Line::from(Span::from(cow_str.to_string())))
                .collect::<Vec<Line>>()
            })
            .collect()
    }

    pub fn widget(&mut self, size: &Rect) -> Paragraph {
        self.update_scroll_state(size);

        let text_lines = self.process_text(&size);
        //eprintln!("Text lines: {:?}", text_lines.len());
        Paragraph::new(Text::from(text_lines))
            .block(Block::default().title("Paragraph").borders(Borders::ALL))
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            //.wrap(Wrap { trim: true })
            .scroll((self.vertical_scroll as u16, 0))
    }

    pub fn insert_str(&mut self, text: &str) {
        self.text.push_str(text);
    }
}
