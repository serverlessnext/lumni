use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::cursor::{Cursor, MoveCursor};
use super::piece_table::{PieceTable, TextLine};
use super::text_wrapper::TextWrapper;

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
        self.text.insert(idx, text, style, false);
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
            let (start_row, start_col, end_row, end_col) =
                self.get_selection_bounds();
            let lines =
                self.text.get_text_lines_selection(start_row, Some(end_row));

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
                        let end_col_inclusive =
                            (end_col + 1).min(line_str.len());
                        selected_lines
                            .push(line_str[..end_col_inclusive].to_string());
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
        let prev_real_col = self.cursor.col;
        let prev_real_row = self.cursor.row;

        let text_lines = self.text.text_lines().to_vec();
        self.cursor.move_cursor(direction, &text_lines, edit_mode);

        let real_column_changed = prev_real_col != self.cursor.col;
        let real_row_changed = prev_real_row != self.cursor.row;
        if real_column_changed || real_row_changed {
            // cursor moved in the underlying text buffer
            // get the cursor position in the wrapped text display
            let (prev_display_col, prev_display_row) =
                self.display_column_row();

            // update the display text
            self.update_display_text();

            // get the cursor position in the wrapped text display after update,
            // this is used to determine if the cursor moved in the display
            let (post_display_col, post_display_row) =
                self.display_column_row();
            return (
                prev_display_col != post_display_col,
                prev_display_row != post_display_row,
            );
        }
        return (false, false);
    }

    pub fn set_selection_anchor(&mut self, enable: bool) {
        self.cursor.set_selection_anchor(enable);
        self.update_display_text();
    }

    fn apply_code_block_styling(&mut self, code_block_style: &Style) {
        let mut in_code_block = false;

        for line in self.display.wrap_lines_mut().iter_mut() {
            // Concatenate all span contents to check for code block delimiters
            let full_line_content = line
                .spans
                .iter()
                .map(|span| span.content.as_ref()) // Convert Cow<str> to &str
                .collect::<String>();
            let line_contains_code_delimiter =
                full_line_content.contains("```");

            // Toggle code block state if delimiter found
            if line_contains_code_delimiter {
                in_code_block = !in_code_block;
                // Process for altering just the delimiter span could go here
                // Optionally apply style changes here if you want the delimiters styled
            }

            // Apply styles to spans if inside a code block
            if in_code_block {
                for span in line.spans.iter_mut() {
                    let mut new_style = span.style.clone();
                    new_style.bg = Some(Color::Gray); // Set background color to gray
                    span.style = new_style;
                }
            }
        }
    }

    pub fn update_display_text(&mut self) {
        self.text.update_if_modified();
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

        let text_wrapper = TextWrapper::new(self.display.width());

        for (idx, line) in text_lines.iter().enumerate() {
            let text_str =
                line.segments().map(|s| s.text()).collect::<String>();

            let trailing_spaces =
                text_str.len() - text_str.trim_end_matches(' ').len();

            //eprintln!("Lines before wrapping: {:?}|{}", text_str, text_str.len());
            let wrapped_lines = text_wrapper.wrap_text_styled(line);

            // debug wrapped lines
            //eprintln!("Wrapped lines: {:?}", wrapped_lines.iter().map(|l| l.to_string()).collect::<Vec<String>>());

            // length of the wrapped lines content 
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

        // apply code block styling after wrapping
        //let code_block_style = Style::default().bg(Color::Gray);
        //self.apply_code_block_styling(&code_block_style);

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
        let mut new_line_position = 0;
        // TODO: there appears to be a bug, that for each wrapped line, the cursor position
        // is one character off. 
        for (row, line) in self.display.wrap_lines().iter().enumerate() {
            // debug line 
            let line_length = line
                .spans
                .iter()
                .map(|span| span.content.len())
                .sum::<usize>();
            // position_newline 
            //eprintln!("Line: {:?}|({})", line.to_string(), line_length);
            if new_line_position + line_length >= cursor_position {
                // Cursor is on this line
                let column = cursor_position.saturating_sub(new_line_position);
                //eprintln!("Cursor,r={},c={},t={},n={}", row, column, cursor_position, new_line_position);
                return (column, row);
            }
            new_line_position += line_length + 1; // account for newline character
        }
        (0, 0) // default to (0, 0) if cursor is not found
    }

    pub fn to_string(&self) -> String {
        self.text.to_string()
    }

    pub fn yank_lines(&self, count: usize) -> Vec<String> {
        let start_row = self.cursor.row as usize;
        // decrement added count by 1 because get_text_lines_selection
        // slices the index range inclusively
        // e.g. to get n lines: end_row = start_row + n - 1
        let end_row = start_row.saturating_add(count.saturating_sub(1));

        if let Some(text_lines) =
            self.text.get_text_lines_selection(start_row, Some(end_row))
        {
            text_lines.iter().map(|line| line.to_string()).collect()
        } else {
            Vec::new()
        }
    }

    pub fn trim(&mut self) {
        self.text.trim();
        self.update_display_text();
    }
}
