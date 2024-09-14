use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Scrollbar, ScrollbarOrientation,
    ScrollbarState, StatefulWidget, StatefulWidgetRef,
};

#[derive(Debug, Clone)]
pub struct ListWidget {
    pub items: Vec<Text<'static>>,
    pub title: Option<String>,
    pub normal_style: Style,
    pub selected_style: Style,
    pub highlight_symbol: String,
    pub show_borders: bool,
}

#[derive(Debug, Clone)]
pub struct ListWidgetState {
    pub selected_index: usize,
    pub scroll_offset: usize,
}

impl Default for ListWidgetState {
    fn default() -> Self {
        Self {
            selected_index: 0,
            scroll_offset: 0,
        }
    }
}

impl ListWidget {
    pub fn new(items: Vec<Text<'static>>) -> Self {
        Self {
            items,
            title: None,
            normal_style: Style::default().fg(Color::Cyan),
            selected_style: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            highlight_symbol: "> ".to_string(),
            show_borders: true,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn show_borders(mut self, show: bool) -> Self {
        self.show_borders = show;
        self
    }

    pub fn normal_style(mut self, style: Style) -> Self {
        self.normal_style = style;
        self
    }

    pub fn selected_style(mut self, style: Style) -> Self {
        self.selected_style = style;
        self
    }

    pub fn highlight_symbol(mut self, symbol: String) -> Self {
        self.highlight_symbol = symbol;
        self
    }

    pub fn get_selected_item(
        &self,
        state: &ListWidgetState,
    ) -> Option<&Text<'static>> {
        self.items.get(state.selected_index)
    }

    pub fn move_selection(&self, state: &mut ListWidgetState, delta: i32) {
        let len = self.items.len();
        if len == 0 {
            return;
        }

        let new_index = if delta > 0 {
            (state.selected_index + 1) % len
        } else if delta < 0 {
            (state.selected_index + len - 1) % len
        } else {
            state.selected_index
        };

        state.selected_index = new_index;
    }

    pub fn page_up(&mut self, state: &mut ListWidgetState, page_size: usize) {
        if self.items.is_empty() {
            return;
        }
        let new_index = state.selected_index.saturating_sub(page_size);
        state.selected_index = new_index;
    }

    pub fn page_down(&mut self, state: &mut ListWidgetState, page_size: usize) {
        if self.items.is_empty() {
            return;
        }
        let max_index = self.items.len() - 1;
        let new_index = (state.selected_index + page_size).min(max_index);
        state.selected_index = new_index;
    }

    fn render_item(
        &self,
        item: &Text<'static>,
        is_selected: bool,
    ) -> Vec<Line<'static>> {
        let style = if is_selected {
            self.selected_style
        } else {
            self.normal_style
        };

        item.lines
            .iter()
            .map(|line| {
                let spans: Vec<Span> = line
                    .spans
                    .iter()
                    .map(|span| {
                        Span::styled(
                            span.content.clone(),
                            style.patch(span.style),
                        )
                    })
                    .collect();
                Line::from(spans)
            })
            .collect()
    }

    fn render_scrollbar(
        &self,
        buf: &mut Buffer,
        area: Rect,
        state: &ListWidgetState,
    ) {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        let items_count = self.items.len();
        let mut scrollbar_state =
            ScrollbarState::new(items_count).position(state.selected_index);

        StatefulWidget::render(
            scrollbar,
            scrollbar_area,
            buf,
            &mut scrollbar_state,
        );
    }

    pub fn get_selected_item_content(
        &self,
        state: &ListWidgetState,
    ) -> Option<String> {
        self.get_selected_item(state).map(extract_text_content)
    }
}

impl StatefulWidget for &ListWidget {
    type State = ListWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidgetRef::render_ref(&self, area, buf, state)
    }
}

impl StatefulWidgetRef for &ListWidget {
    type State = ListWidgetState;

    fn render_ref(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State,
    ) {
        let list_area = area.inner(Margin::new(1, 1));
        let list_height = list_area.height as usize;

        let items_with_heights: Vec<(usize, &Text<'static>)> = self
            .items
            .iter()
            .map(|item| (item.height(), item))
            .collect();

        let total_height: usize =
            items_with_heights.iter().map(|(h, _)| *h).sum();
        let max_scroll = total_height.saturating_sub(list_height);

        // Adjust scroll if necessary
        if state.selected_index >= state.scroll_offset + list_height {
            state.scroll_offset =
                state.selected_index.saturating_sub(list_height) + 1;
        } else if state.selected_index < state.scroll_offset {
            state.scroll_offset = state.selected_index;
        }
        state.scroll_offset = state.scroll_offset.min(max_scroll);

        let mut visible_items = Vec::new();
        let mut current_height = 0;
        let mut visible_index = 0;

        for (index, (height, item)) in items_with_heights.iter().enumerate() {
            if current_height >= state.scroll_offset + list_height {
                break;
            }

            if current_height + height > state.scroll_offset {
                let is_selected = index == state.selected_index;
                let lines = self.render_item(item, is_selected);
                visible_items.push(ListItem::new(lines));

                if index == state.selected_index {
                    visible_index = visible_items.len() - 1;
                }
            }

            current_height += height;
        }

        let mut block = Block::default().borders(if self.show_borders {
            Borders::ALL
        } else {
            Borders::NONE
        });

        if let Some(title) = &self.title {
            block = block.title(title.clone());
        }

        let list = List::new(visible_items)
            .block(block)
            .highlight_style(self.selected_style)
            .highlight_symbol(&self.highlight_symbol);
        let mut list_state = ListState::default();
        list_state.select(Some(visible_index));

        // Render list
        StatefulWidget::render(list, area, buf, &mut list_state);

        // Render scrollbar
        if total_height > list_height {
            self.render_scrollbar(buf, area, state);
        }
    }
}

fn extract_text_content(text: &Text) -> String {
    text.lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<Vec<&str>>()
        .join("")
}
