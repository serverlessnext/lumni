use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::cursor::{Cursor, MoveCursor};
use super::piece_table::{PieceTable, TextLine};

#[derive(Debug, Clone)]
pub struct TextDisplay<'a> {
    wrap_lines: Vec<Line<'a>>, // Text (e.g., wrapped, highlighted) for display
    display_width: usize,      // Width of the display area, used for wrapping
}

impl<'a> TextDisplay<'a> {
    fn new(display_width: usize) -> Self {
        TextDisplay {
            wrap_lines: Vec::new(),
            display_width,
        }
    }

    fn wrap_lines(&self) -> &[Line<'a>] {
        &self.wrap_lines
    }

    fn wrap_lines_mut(&mut self) -> &mut Vec<Line<'a>> {
        &mut self.wrap_lines
    }

    fn width(&self) -> usize {
        self.display_width
    }

    fn push_line(&mut self, line: Line<'a>) {
        self.wrap_lines.push(line);
    }

    fn set_display_width(&mut self, width: usize) {
        self.display_width = width;
    }

    fn clear(&mut self) {
        self.wrap_lines.clear();
    }
}

#[derive(Debug, Clone)]
pub struct TextBuffer<'a> {
    text: PieceTable,         // text buffer
    placeholder: String,      // placeholder text
    display: TextDisplay<'a>, // text (e.g. wrapped,  highlighted) for display
    cursor: Cursor,
    is_editable: bool,
}

impl TextBuffer<'_> {
    pub fn new(is_editable: bool) -> Self {
        Self {
            text: PieceTable::new(),
            placeholder: String::new(),
            display: TextDisplay::new(0),
            cursor: Cursor::new(0, 0, false),
            is_editable,
        }
    }

    pub fn set_placeholder(&mut self, text: &str) {
        if self.placeholder != text {
            self.placeholder = text.to_string();
            if self.text.is_empty() {
                // trigger display update if placeholder text changed,
                // and the text buffer is empty
                self.update_display_text();
            }
        }
    }

    pub fn set_cursor_visibility(&mut self, visible: bool) {
        if self.cursor.show_cursor() != visible {
            self.cursor.set_visibility(visible);
            // update display when cursor visibility changes
            self.update_display_text();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn empty(&mut self) {
        self.display.clear();
        self.cursor = Cursor::new(0, 0, self.is_editable);
        self.text.empty();
        // update display
        self.update_display_text();
    }

    pub fn set_width(&mut self, width: usize) {
        self.display.set_display_width(width);
    }

    pub fn text_insert_add(&mut self, text: &str, style: Option<Style>) {
        // Get the current cursor position in the underlying (unwrapped) text buffer
        let idx = self.cursor.real_position();
        self.text.cache_insert(text, Some(idx), style);
        self.update_display_text();

        // Calculate the number of newlines and the length of the last line segment
        let mut newlines = 0;
        let mut last_line_length = 0;
        for ch in text.chars() {
            if ch == '\n' {
                newlines += 1;
                last_line_length = 0; // Reset line length counter after each newline
            } else {
                last_line_length += 1; // Increment line length for non-newline characters
            }
        }

        if newlines > 0 {
            // Move the cursor to the end of the inserted text
            self.move_cursor(MoveCursor::Down(newlines as u16), true);
            self.move_cursor(MoveCursor::StartOfLine, true); // Move to the start of the new line
            if last_line_length > 0 {
                // Then move right to the end of the inserted text on the last line
                self.move_cursor(
                    MoveCursor::Right(last_line_length as u16),
                    true,
                );
            }
        } else {
            // If no newlines, just move right
            self.move_cursor(MoveCursor::Right(text.len() as u16), true);
        }
    }

    pub fn text_append(&mut self, text: &str, style: Option<Style>) {
        self.text.append(text, style);
        self.update_display_text();
    }

    pub fn text_delete(&mut self, include_cursor: bool, char_count: usize) {
        // get current cursor position in the underlying (unwrapped) text buffer
        let idx = self.cursor.real_position();
        if char_count == 0 {
            return; // nothing to delete
        }

        let start_idx = if include_cursor {
            idx //  start at the highlighed (cursor) character
        } else if idx > 0 {
            idx - 1 // start at the character before the cursor
        } else {
            return;
        };

        self.text.delete(start_idx, char_count);

        if include_cursor {
            // delete rightwards from the cursor
            // extra update is required to get correct text after deletion,
            self.update_display_text();

            // check if the cursor is at the end of the line
            if self.cursor.col as usize >= self.to_string().len() {
                self.move_cursor(MoveCursor::Left(char_count as u16), false);
            }
        } else {
            // delete leftwards from the cursor
            self.move_cursor(MoveCursor::Left(char_count as u16), true);
        }
    }

    pub fn text_insert_commit(&mut self) -> String {
        self.text.commit_insert_cache()
    }

    pub fn display_text(&self) -> Vec<Line> {
        self.display.wrap_lines().to_vec()
    }

    pub fn display_lines_len(&self) -> usize {
        self.display.wrap_lines().len()
    }

    pub fn yank_selected_text(&self) -> Option<String> {
        // check if selection is active 
        if self.cursor.selection_enabled() {
            // get selection bounds
            let (start_row, start_col, end_row, end_col) = self.get_selection_bounds();
            let lines = self.text.get_text_lines_selection(start_row, Some(end_row));

            if let Some(lines) = lines {
                let mut selected_lines = Vec::new();

                // Iterate over the lines within the selection
                for (idx, line) in lines.iter().enumerate() {
                    let line_str = line.to_string();
                    if idx == 0 {
                        // First row: get the text from start_col to the end
                        selected_lines.push(line_str[start_col..].to_string());
                    } else if idx == lines.len() - 1 {
                        // Last row: get the text from 0 to end_col
                        let end_col_inclusive = (end_col + 1).min(line_str.len());
                        selected_lines.push(line_str[..end_col_inclusive].to_string());
                    } else {
                        // Middle row: take the whole line
                        selected_lines.push(line_str);
                    }
                }
                // Join the selected lines
                let selected_text = selected_lines.join("\n");
                return Some(selected_text);
            }
            return Some("".to_string());
        }
        None
    }

    pub fn move_cursor(
        &mut self,
        direction: MoveCursor,
        edit_mode: bool,
    ) -> (bool, bool) {
        let prev_col = self.cursor.col;
        let prev_row = self.cursor.row;

        let text_lines = self.text.text_lines().to_vec();
        self.cursor.move_cursor(direction, &text_lines, edit_mode);

        let column_changed = prev_col != self.cursor.col;
        let row_changed = prev_row != self.cursor.row;
        if column_changed || row_changed {
            // update the display text to reflect the change
            self.update_display_text();
        }
        (column_changed, row_changed)
    }

    pub fn set_selection_anchor(&mut self, enable: bool) {
        self.cursor.set_selection_anchor(enable);
        self.update_display_text();
    }

    pub fn update_display_text(&mut self) {
        self.text.update_lines_styled();
        self.display.clear();
        let mut text_lines = self.text.text_lines().to_vec();

        if text_lines.is_empty() && !self.placeholder.is_empty() {
            // placeholder text
            let style = Style::default().fg(Color::DarkGray);
            let mut line_styled = TextLine::new();
            line_styled.add_segment(self.placeholder.clone(), Some(style));
            text_lines.push(line_styled);
        }

        // Get the bounds of selected text, position based on unwrapped lines
        // (start_row, start_col, end_row, end_col)
        let selection_bounds = self.get_selection_bounds();
        for (idx, line) in text_lines.iter().enumerate() {
            let text_str =
                line.segments().map(|s| s.text()).collect::<String>();

            let trailing_spaces =
                text_str.len() - text_str.trim_end_matches(' ').len();

            let wrapped_lines = self.wrap_text_styled(&line);
            if wrapped_lines.is_empty() {
                self.handle_empty_line(idx, trailing_spaces);
            } else {
                // process wrapped lines
                self.process_wrapped_lines(
                    wrapped_lines,
                    idx,
                    &selection_bounds,
                    trailing_spaces,
                );
            }
        }
        self.cursor.update_real_position(&text_lines);
        self.update_cursor_style();
    }

    fn get_selection_bounds(&self) -> (usize, usize, usize, usize) {
        if self.cursor.selection_enabled() {
            self.cursor.get_selection_bounds()
        } else {
            (usize::MAX, usize::MAX, usize::MIN, usize::MIN) // No highlighting
        }
    }

    fn wrap_text_styled(&self, line: &TextLine) -> Vec<TextLine> {
        // Wrap text into lines based on the display width, keeping the style
        // of each segment, preserving leading spaces
        let mut wrapped_lines = Vec::new();
        let mut current_line = TextLine::new();

        let max_width = self.display.width().saturating_sub(2); // deduct space for padding
        if max_width < 1 {
            // No width available to wrap any text
            return wrapped_lines;
        }
        for segment in line.segments() {
            let text = segment.text();

            // Split text into words and keep leading spaces
            let mut current_text = String::new();

            // Use regex to capture leading spaces and words separately
            let re = regex::Regex::new(r"(\s*)(\S+)").unwrap();
            let mut words = Vec::new();

            for cap in re.captures_iter(text) {
                words.push((cap[1].to_string(), cap[2].to_string()));
            }

            for (mut leading_spaces, word) in words {
                // Calculate the space needed if current_text is not empty
                let space_len = if !current_text.is_empty() { 1 } else { 0 };

                // Check if the current word with leading spaces fits in the line
                if current_text.len()
                    + space_len
                    + leading_spaces.len()
                    + word.len()
                    + current_line.length()
                    > max_width
                {
                    // If adding this word would exceed max_width, push current_text to current_line
                    if !current_text.is_empty() {
                        current_line.add_segment(
                            current_text.trim_end().to_string(),
                            segment.style().clone(),
                        );
                        current_text.clear();
                    }

                    // If the word itself is too long, handle it specifically
                    if leading_spaces.len() + word.len() > max_width {
                        // Add leading spaces to the current_text before handling the word
                        current_text.push_str(&leading_spaces);

                        // Split the word into manageable pieces
                        let mut start_index = 0;
                        while start_index < word.len() {
                            let end_index = std::cmp::min(
                                (start_index + max_width)
                                    .saturating_sub(current_text.len()),
                                word.len(),
                            );
                            let slice = &word[start_index..end_index];

                            // Ensure there's no trailing line without segments
                            if !current_line.is_empty() {
                                wrapped_lines.push(current_line);
                                current_line = TextLine::new();
                            }

                            current_line.add_segment(
                                current_text.clone() + slice,
                                segment.style().clone(),
                            );
                            wrapped_lines.push(current_line);
                            current_line = TextLine::new();
                            start_index = end_index;
                            current_text.clear(); // Clear current_text after the first use
                        }
                        continue; // Continue to the next word
                    } else {
                        // Start a new line with the current word if it fits alone
                        if !leading_spaces.is_empty() {
                            leading_spaces.remove(0); // Remove one leading space
                        }
                        wrapped_lines.push(current_line);
                        current_line = TextLine::new();
                        current_text = format!("{}{}", leading_spaces, word);
                    }
                } else {
                    // Add word (including leading spaces) to current_text
                    current_text
                        .push_str(&format!("{}{}", leading_spaces, word));
                }
            }

            // After all words, if there's leftover text, add it as a segment
            if !current_text.is_empty() {
                current_line.add_segment(
                    current_text.trim_end().to_string(),
                    segment.style().clone(),
                );
            }
        }

        // Add the last line if it has segments
        if !current_line.is_empty() {
            wrapped_lines.push(current_line);
        }
        wrapped_lines
    }

    fn handle_empty_line(
        &mut self,
        current_row: usize,
        trailing_spaces: usize,
    ) {
        if trailing_spaces > 0 {
            // Add trailing spaces to the line
            let spaces = std::iter::repeat(' ')
                .take(trailing_spaces)
                .collect::<String>();
            self.display.push_line(Line::from(Span::raw(spaces)));
        } else if current_row == self.cursor.row as usize {
            // current selected row -- add a line with a single space for cursor position
            self.display.push_line(Line::from(Span::raw(" ")));
        } else {
            self.display.push_line(Line::from(Span::raw("")));
        }
    }

    fn process_wrapped_lines(
        &mut self,
        wrapped_lines: Vec<TextLine>,
        unwrapped_line_index: usize,
        selection_bounds: &(usize, usize, usize, usize),
        // trailing spaces of the unwrapped line are removed during wrapping,
        // this is added back to the first and last (wrapped) line respectively
        trailing_spaces: usize,
    ) {
        let (start_row, start_col, end_row, end_col) = *selection_bounds;
        let mut char_pos = 0;

        for (idx, line) in wrapped_lines.iter().enumerate() {
            let mut spans = Vec::new();

            // Start character position for this line from the cumulative offset
            for segment in line.segments() {
                let chars: Vec<char> = segment.text().chars().collect();
                for ch in chars.into_iter() {
                    // Adjust row based on the index in wrapped lines
                    let should_select = self.cursor.should_select(
                        unwrapped_line_index,
                        char_pos,
                        start_row,
                        start_col,
                        end_row,
                        end_col,
                    );

                    let mut effective_style =
                        segment.style().unwrap_or(Style::default());
                    if should_select {
                        effective_style = effective_style.bg(Color::Blue);
                    }
                    spans.push(Span::styled(ch.to_string(), effective_style));
                    char_pos += 1;
                }
            }

            let mut current_line = Line::from(spans);
            if trailing_spaces > 0 && idx == wrapped_lines.len() - 1 {
                // Add trailing spaces back to end of the last line
                let spaces = std::iter::repeat(' ')
                    .take(trailing_spaces)
                    .collect::<String>();
                current_line.spans.push(Span::raw(spaces));
            }
            self.display.push_line(current_line);
            char_pos += 1; // account for newline character
        }
    }

    fn update_cursor_style(&mut self) {
        if !self.cursor.show_cursor() {
            return;
        }
        // Retrieve the cursor's column and row in the wrapped display
        let (mut column, row) = self.display_column_row();

        if let Some(current_line) = self.display.wrap_lines_mut().get_mut(row) {
            let line_length = current_line
                .spans
                .iter()
                .map(|span| span.content.len())
                .sum::<usize>();
            if column >= line_length {
                current_line.spans.push(Span::styled(
                    " ",
                    Style::default().bg(Color::Yellow),
                ));
            } else {
                // Iterate through spans to find the correct position
                let mut new_spans = Vec::new();
                let mut span_offset = 0;
                for span in current_line.spans.iter() {
                    let span_length = span.content.len();

                    if column < span_length {
                        // Split the span at the cursor position
                        let before = &span.content[..column];
                        let cursor_char = &span.content[column..column + 1];
                        let after = &span.content[column + 1..];

                        if !before.is_empty() {
                            new_spans.push(Span::styled(
                                before.to_string(),
                                span.style,
                            ));
                        }

                        new_spans.push(Span::styled(
                            cursor_char.to_string(),
                            span.style.bg(Color::Yellow),
                        ));

                        if !after.is_empty() {
                            new_spans.push(Span::styled(
                                after.to_string(),
                                span.style,
                            ));
                        }

                        // Add remaining spans as is
                        new_spans.extend(
                            current_line
                                .spans
                                .iter()
                                .skip(span_offset + 1)
                                .cloned(),
                        );
                        break;
                    } else {
                        new_spans.push(span.clone());
                        column -= span_length;
                    }

                    span_offset += 1;
                }
                current_line.spans = new_spans;
            }
        }
    }

    pub fn undo(&mut self) {
        self.text.undo();
        self.update_display_text();
    }

    pub fn redo(&mut self) {
        self.text.redo();
        self.update_display_text();
    }

    pub fn display_column_row(&self) -> (usize, usize) {
        // Get the current row in the wrapped text display based on the cursor position
        let cursor_position = self.cursor.real_position();
        // eprintln!("WrappedText={:?}|", self.display.wrap_lines().iter().map(|line| line.spans.iter().map(|span| span.content.clone()).collect::<String>()).collect::<String>());
        let mut wrap_position = 0;
        for (row, line) in self.display.wrap_lines().iter().enumerate() {
            let line_length = line
                .spans
                .iter()
                .map(|span| span.content.len())
                .sum::<usize>();
            if wrap_position + line_length >= cursor_position {
                // Cursor is on this line
                let column = cursor_position - wrap_position;
                //eprintln!("wrap_position: {}, line_length: {}, cursor_position: {}, column: {}, row: {}", wrap_position, line_length, cursor_position, column, row);
                return (column, row);
            }
            wrap_position += line_length + 1; // account for newline character
        }
        //eprintln!("Cant find cursor position: {}", cursor_position);
        (0, 0) // default to (0, 0) if cursor is not found
    }

    pub fn to_string(&self) -> String {
        self.text.to_string()
    }

    pub fn yank_lines(&self, count: usize) -> Vec<String> {
        let start_row = self.cursor.row as usize;
        // decrement start_row by count - 1 because get_text_lines_selection
        // slices the index range inclusively
        let end_row = start_row.saturating_add(count.saturating_sub(1));

        if let Some(text_lines) =
            self.text.get_text_lines_selection(start_row, Some(end_row))
        {
            text_lines.iter().map(|line| line.to_string()).collect()
        } else {
            Vec::new()
        }
    }
}
