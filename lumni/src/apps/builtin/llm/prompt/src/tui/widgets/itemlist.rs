use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Scrollbar, ScrollbarOrientation,
    ScrollbarState, StatefulWidget, StatefulWidgetRef,
};

use super::TextSegment;

#[derive(Debug, Clone)]
pub struct ListWidget {
    items: Vec<TextSegment>,
    title: String,
    normal_style: Style,
    selected_style: Style,
    highlight_symbol: String,
}

#[derive(Debug, Clone)]
pub struct ListWidgetState {
    selected_index: usize,
    scroll_offset: usize,
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
    pub fn new(items: Vec<TextSegment>, title: String) -> Self {
        Self {
            items,
            title,
            normal_style: Style::default().fg(Color::Cyan),
            selected_style: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            highlight_symbol: "> ".to_string(),
        }
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

    pub fn get_selected_item(
        &self,
        state: &ListWidgetState,
    ) -> Option<&TextSegment> {
        self.items.get(state.selected_index)
    }

    fn render_scrollbar(
        &self,
        buf: &mut Buffer,
        area: Rect,
        total_items: usize,
        list_height: usize,
        state: &ListWidgetState,
    ) {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .style(Style::default().fg(Color::Gray));

        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        let max_scroll = total_items.saturating_sub(list_height);
        let scroll_position = (state.scroll_offset as f64 / max_scroll as f64
            * (list_height.saturating_sub(1)) as f64)
            .round() as usize;

        let mut scrollbar_state = ScrollbarState::new(list_height)
            .position(scroll_position.min(list_height.saturating_sub(1)));

        StatefulWidget::render(
            scrollbar,
            scrollbar_area,
            buf,
            &mut scrollbar_state,
        );
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
        let list_height = area.height.saturating_sub(2) as usize;
        let total_items = self.items.len();

        if total_items == 0 {
            let empty_list = List::new(Vec::<ListItem>::new()).block(
                Block::default()
                    .title(self.title.clone())
                    .borders(Borders::ALL),
            );
            StatefulWidget::render(
                empty_list,
                area,
                buf,
                &mut ListState::default(),
            );
            return;
        }

        // Adjust scroll_offset for wrapping
        if state.selected_index >= state.scroll_offset + list_height {
            state.scroll_offset =
                state.selected_index.saturating_sub(list_height) + 1;
        } else if state.selected_index < state.scroll_offset {
            state.scroll_offset = state.selected_index;
        }

        // Ensure scroll_offset doesn't exceed max_scroll
        let max_scroll = total_items.saturating_sub(list_height);
        state.scroll_offset = state.scroll_offset.min(max_scroll);

        let visible_items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .skip(state.scroll_offset)
            .take(list_height)
            .map(|(index, item)| {
                let style = if index == state.selected_index {
                    item.style.unwrap_or(self.selected_style)
                } else {
                    item.style.unwrap_or(self.normal_style)
                };
                ListItem::new(Line::from(vec![Span::styled(
                    item.text.as_str(),
                    style,
                )]))
            })
            .collect();

        let list = List::new(visible_items)
            .block(
                Block::default()
                    .title(self.title.clone())
                    .borders(Borders::ALL),
            )
            .highlight_style(self.selected_style)
            .highlight_symbol(&self.highlight_symbol);

        let mut list_state = ListState::default();
        list_state.select(Some(
            state.selected_index.saturating_sub(state.scroll_offset),
        ));

        StatefulWidget::render(list, area, buf, &mut list_state);

        if total_items > list_height {
            self.render_scrollbar(buf, area, total_items, list_height, state);
        }
    }
}
