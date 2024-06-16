use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Masked};

use super::cursor::{Cursor, MoveCursor};
use super::piece_table::{PieceTable, TextLine};
use super::text_wrapper::TextWrapper;


#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodeBlock {
    start: u16, // start line of the code block
    end: Option<u16>,   // end line of the code block (if closed)
}

impl CodeBlock {
    pub fn is_closed(&self) -> bool {
        self.end.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodeBlockLineType {
    Start,
    End,
    Line,
}

#[derive(Debug, Clone, Copy)]
pub struct CodeBlockLine {
    ptr: u16,   // refers to the code block itself
    r#type: CodeBlockLineType,
}

impl CodeBlockLine {
    pub fn get_ptr(&self) -> u16 {
        self.ptr
    }

    pub fn get_type(&self) -> CodeBlockLineType {
        self.r#type
    }
    
    pub fn is_end(&self) -> bool {
        self.r#type == CodeBlockLineType::End
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LineType {
    Text,
    Code(CodeBlockLine),
}

#[derive(Debug, Clone)]
struct LineSegment<'a> {
    line: Line<'a>,             // wrapped line segment
    length: usize,              // length of the line segment
    last_segment: bool,         // last part of a line
    line_type: Option<LineType>,    // type of line: Text or Code
    background: Option<Color>,  // default background color
}

impl<'a> LineSegment<'a> {
    fn new(
        line: Line<'a>,
        length: usize,
        last_segment: bool,
        line_type: Option<LineType>,
        background: Option<Color>,
    ) -> Self {
        LineSegment {
            line,
            length,
            last_segment,
            line_type,
            background,
        }
    }

    fn spans_mut(&mut self) -> &mut Vec<Span<'a>> {
        &mut self.line.spans
    }
}

#[derive(Debug, Clone)]
pub struct TextDisplay<'a> {
    wrap_lines: Vec<LineSegment<'a>>, // Text (e.g., wrapped, highlighted) for display
    display_width: usize, // Width of the display area, used for wrapping
    column: usize,
    row: usize,
}

impl<'a> TextDisplay<'a> {
    fn new(display_width: usize) -> Self {
        TextDisplay {
            wrap_lines: Vec::new(),
            display_width,
            column: 0,
            row: 0,
        }
    }

    pub fn update_column_row(&mut self, cursor: &Cursor) -> (usize, usize) {
        // Get the current row in the wrapped text display based on the cursor position
        let cursor_position = cursor.real_position();
        let mut new_line_position = 0;

        self.column = 0;
        self.row = 0;

        let last_line = self.wrap_lines.len().saturating_sub(1);

        for (row, line) in self.wrap_lines.iter().enumerate() {
            let line_length = if line.last_segment {
                line.length + 1 // account for end of line/ cursor space
            } else {
                line.length
            };

            // position_newline
            if new_line_position + line_length > cursor_position
                || row == last_line
            {
                // Cursor is on this line
                //eprintln!("{}:Line: {:?}|({})", line.idx, line.to_string(), line_length);
                let column = cursor_position.saturating_sub(new_line_position);
                //eprintln!("Cursor,r={},c={},t={},n={}", row, column, cursor_position, new_line_position);
                self.column = column;
                self.row = row;
                break;
            }
            new_line_position += line_length;
        }
        (self.column, self.row)
    }

    fn wrap_lines(&self) -> &[LineSegment<'a>] {
        &self.wrap_lines
    }

    fn wrap_lines_mut(&mut self) -> &mut Vec<LineSegment<'a>> {
        &mut self.wrap_lines
    }

    fn width(&self) -> usize {
        self.display_width
    }

    fn push_line(
        &mut self,
        line: Line<'a>,
        length: usize,
        last_segment: bool,
        line_type: Option<LineType>,
        background: Option<Color>,
    ) {
        self.wrap_lines.push(LineSegment::new(
            line,
            length,
            last_segment,
            line_type,
            background,
        ));
    }

    fn set_display_width(&mut self, width: usize) {
        self.display_width = width;
    }

    fn get_column_row(&self) -> (usize, usize) {
        (self.column, self.row)
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
    code_blocks: Vec<CodeBlock>,    // code blocks
    is_editable: bool,
}

impl TextBuffer<'_> {
    pub fn new(is_editable: bool) -> Self {
        Self {
            text: PieceTable::new(),
            placeholder: String::new(),
            display: TextDisplay::new(0),
            cursor: Cursor::new(0, 0, false),
            code_blocks: Vec::new(),
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

    pub fn get_column_row(&self) -> (usize, usize) {
        self.display.get_column_row()
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

    pub fn row_line_type(&self, row: usize) -> Option<LineType> {
        let lines = self.display.wrap_lines();
        if row >= lines.len() {
            return None;    // out of bounds
        }
        lines[row].line_type
    }

    pub fn get_code_block(&self, ptr: u16) -> Option<&CodeBlock> {
        self.code_blocks.get(ptr as usize)
    }

    pub fn display_window_lines(&self, start: usize, end: usize) -> Vec<Line> {
        let lines = self.display.wrap_lines();
        let length = lines.len();

        if start >= length {
            return Vec::new(); // out of bounds
        }

        // Convert inclusive end to exclusive end for slicing
        let exclusive_end = (end + 1).min(length);

        if start > end {
            return Vec::new(); // invalid range
        }

        let window_width = self.display.width();

        lines[start..exclusive_end]
            .iter()
            .map(|line_segment| {
                let mut line = line_segment.line.clone();
                let text_width = line.width();

                let line_type = line_segment.line_type;

                let line_bg = if let Some(bg) = line_segment.background {
                    if bg == Color::Reset {
                        Color::Black    // switch to black as default 
                    } else {
                        bg  // keep original background color
                    }
                } else {
                    Color::Black    // default background color
                };

                let background = match line_type {
                    Some(LineType::Text) => {
                        if line_segment.background.is_some() {
                            for span in &mut line.spans {
                                // fill background color if not already set
                                if span.style.bg.is_none() || span.style.bg == Some(Color::Reset) {
                                    span.style.bg = Some(line_bg);
                                }
                            }
                        }
                        Some(line_bg)
                    }
                    Some(LineType::Code(block_line)) => {
                        match block_line.get_type() {
                            CodeBlockLineType::Line => {
                                // fill background color if not already set
                                let background = Some(Color::Rgb(80, 80, 80));
                                for span in &mut line.spans {
                                    if span.style.bg.is_none() || span.style.bg == Some(Color::Reset) {
                                        span.style.bg = background;
                                    }
                                }
                                background
                            },
                            CodeBlockLineType::Start => {
                                // hide line using modifier style
                                let masked = Masked::new("```", '>');
                                if let Some(background) = line_segment.background {
                                    line.spans = vec![Span::styled(masked, Style::default().bg(background))];
                                } else {
                                    line.spans = vec![Span::raw(masked)];
                                }
                                line_segment.background
                            }
                            CodeBlockLineType::End => {
                                // hide line using modifier style
                                let masked = Masked::new("```", '<');
                                if let Some(background) = line_segment.background {
                                    line.spans = vec![Span::styled(masked, Style::default().bg(background))];
                                } else {
                                    line.spans = vec![Span::raw(masked)];
                                }
                                line_segment.background
                            }
                        }
                    }
                    None => {
                        // default background color
                        line_segment.background
                    }
                };

                // left padding
                line.spans.insert(0, Span::styled(
                    " ",
                    Style::default().bg(background.unwrap_or(Color::Black)),
                ));
 
                if text_width < window_width {
                    // add 2 to account for left and right padding
                    let spaces_needed = window_width.saturating_sub(text_width + 2);
                    let spaces = " ".repeat(spaces_needed);

                    if let Some(bg_color) = background {
                        // fill background color with default if not already set
                       line.spans.push(Span::styled(
                            spaces,
                            Style::default().bg(bg_color),
                        ));
                    } else {
                        line.spans.push(Span::raw(spaces));
                    }
                };

                // right padding
                line.spans.push(Span::styled(
                    " ",
                    Style::default().bg(background.unwrap_or(Color::Black)),
                ));
 

                line
            })
            .collect()
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
                self.display.get_column_row();

            // update the display text
            self.update_display_text();

            // get the cursor position in the wrapped text display after update,
            // this is used to determine if the cursor moved in the display
            let (post_display_col, post_display_row) =
                self.display.get_column_row();
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

    fn mark_code_blocks(&mut self) {
        let mut in_code_block = false;
        let mut current_code_block_start: Option<u16> = None;
        let mut code_block_ptr = 0;

        self.code_blocks.clear();
        let reset = Style::reset();

        for (line_number, line) in self.display.wrap_lines_mut().iter_mut().enumerate() {
            let line_number = line_number as u16;

            if in_code_block && line.background == reset.bg {
                // ensure code block does not persist across different text blocks
                in_code_block = false;
            }

            // check length first to avoid unnecessary comparison
            if line.length == 3 && line.line.to_string() == "```" {
                if in_code_block {
                    // end of code block
                    in_code_block = false;

                    if let Some(_) = current_code_block_start {
                        // close the last code block
                        if let Some(last_code_block) = self.code_blocks.last_mut() {
                            last_code_block.end = Some(line_number);
                        }
                    }

                    line.line_type = Some(LineType::Code(CodeBlockLine {
                        ptr: code_block_ptr,
                        r#type: CodeBlockLineType::End,
                    }));
                    code_block_ptr += 1;    // increment for the next code block
                } else {
                    // start of code block
                    in_code_block = true;
                    current_code_block_start = Some(line_number);
                    self.code_blocks.push(CodeBlock {
                        start: line_number,
                        end: None,
                    });
                    line.line_type = Some(LineType::Code(CodeBlockLine {
                        ptr: code_block_ptr,    // ptr to the code block
                        r#type: CodeBlockLineType::Start,
                    }));
                }
            } else {
                // mark line as code or text based on wether it is in a code block
                line.line_type = Some(if in_code_block {
                    // remove default background color if set
                    // keep other background colors (e.g. selection, cursor highlighting)
                    let bg_default = line.background;
                    for span in line.spans_mut() {
                        if span.style.bg == bg_default {
                            span.style.bg = None;
                        }
                    }
                    LineType::Code(CodeBlockLine {
                        ptr: code_block_ptr,
                        r#type: CodeBlockLineType::Line,
                    })
                } else {
                    LineType::Text
                });
            }

            if !in_code_block {
                current_code_block_start = None;
            }
        }

        // TODO: for each codeblock, add syntax styling
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
        // debug text lines including newlines
        //let total_length: usize = text_lines.iter().map(|l| l.length() + 1).sum::<usize>().saturating_sub(1);
        //eprintln!("Text lines:\n{}|{}", text_lines.iter().map(|l| l.to_string()).collect::<Vec<String>>().join("\n"), total_length);

        for (idx, line) in text_lines.iter().enumerate() {
            let text_str =
                line.segments().map(|s| s.text()).collect::<String>();

            let trailing_spaces =
                text_str.len() - text_str.trim_end_matches(' ').len();

            let wrapped_lines = text_wrapper.wrap_text_styled(line);

            // debug wrapped lines
            //eprintln!("Wrapped lines: {:?}", wrapped_lines.iter().map(|l| l.to_string()).collect::<Vec<String>>());

            // length of the wrapped lines content
            if wrapped_lines.is_empty() {
                self.handle_empty_line(
                    trailing_spaces,
                    line.get_background(),
                );
            } else {
                // process wrapped lines
                self.process_wrapped_lines(
                    wrapped_lines,
                    idx,
                    &selection_bounds,
                    trailing_spaces,
                    line.get_background(),
                );
            }
        }

        self.mark_code_blocks();
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
        trailing_spaces: usize,
        background: Option<Color>,
    ) {
        if trailing_spaces > 0 {
            // Add trailing spaces to the line
            let spaces = std::iter::repeat(' ')
                .take(trailing_spaces)
                .collect::<String>();

            self.display.push_line(
                Line::from(Span::raw(spaces)),
                trailing_spaces,
                true,
                None,
                background,
            );
        } else {
            // add empty row
            self.display.push_line(
                Line::from(Span::raw("")),
                0,
                true,
                None,
                background,
            );
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
        background: Option<Color>,
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

            let last_segment = idx == wrapped_lines.len() - 1;

            if last_segment && trailing_spaces > 0 {
                // Add trailing spaces back to end of the last line
                let spaces = std::iter::repeat(' ')
                    .take(trailing_spaces)
                    .collect::<String>();
                current_line.spans.push(Span::raw(spaces));
            }
            let current_line_length = current_line
                .spans
                .iter()
                .map(|span| span.content.len())
                .sum::<usize>();
            self.display.push_line(
                current_line,
                current_line_length,
                last_segment,
                None,
                background,
            );
            char_pos += 1; // account for newline character
        }
    }

    fn update_cursor_style(&mut self) {
        if !self.cursor.show_cursor() {
            return;
        }
        // Retrieve the cursor's column and row in the wrapped display
        let (column, row) = self.display.update_column_row(&self.cursor);
        let mut line_column = column;

        if let Some(current_line) = self.display.wrap_lines_mut().get_mut(row) {
            let line_length = current_line.length;
            let last_segment = current_line.last_segment;
            let spans = current_line.spans_mut();
            if line_column >= line_length && last_segment {
                spans
                    .push(Span::styled("_", Style::default().bg(Color::Green)));
            } else {
                // Iterate through spans to find the correct position
                let mut new_spans = Vec::new();
                let mut span_offset = 0;
                for span in spans.iter() {
                    let span_length = span.content.len();
                    if line_column < span_length {
                        // Split the span at the cursor position
                        let before = &span.content[..line_column];
                        let cursor_char =
                            &span.content[line_column..line_column + 1];
                        let after = &span.content[line_column + 1..];

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
                            spans.iter().skip(span_offset + 1).cloned(),
                        );
                        break;
                    } else {
                        new_spans.push(span.clone());
                        line_column -= span_length;
                    }

                    span_offset += 1;
                }
                *spans = new_spans;
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
}
