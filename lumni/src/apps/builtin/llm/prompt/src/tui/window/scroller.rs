use ratatui::widgets::ScrollbarState;

#[derive(Debug, Clone, Copy)]
pub struct Scroller {
    vertical_scroll_bar_state: ScrollbarState, // visual state of the scrollbar
    pub vertical_scroll: usize, // vertical scroll position (line index)
    pub auto_scroll: bool, // automatically scroll to end of text when updated
}

impl Scroller {
    pub fn new() -> Self {
        Self {
            vertical_scroll_bar_state: ScrollbarState::default(),
            vertical_scroll: 0,
            auto_scroll: false,
        }
    }

    pub fn enable_auto_scroll(&mut self) {
        self.auto_scroll = true;
    }

    pub fn disable_auto_scroll(&mut self) {
        self.auto_scroll = false;
    }

    pub fn vertical_scroll_bar_state<'b>(
        &'b mut self,
    ) -> &'b mut ScrollbarState {
        &mut self.vertical_scroll_bar_state
    }

    pub fn update_scroll_bar(
        &mut self,
        display_length: usize,
        content_length: usize,
    ) {
        self.vertical_scroll_bar_state = self
            .vertical_scroll_bar_state
            .content_length(display_length)
            .viewport_content_length(content_length)
            .position(self.vertical_scroll);
    }
}
