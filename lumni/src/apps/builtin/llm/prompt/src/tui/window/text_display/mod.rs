mod text_render;

use ratatui::style::Color;
use ratatui::text::{Line, Span};
use text_render::DisplayWindowRenderer;

use super::cursor::Cursor;
use super::text_document::{TextLine, TextWrapper};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CodeBlock {
    pub start: u16,       // start line of the code block
    pub end: Option<u16>, // end line of the code block (if closed)
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
    ptr: u16, // refers to the code block itself
    r#type: CodeBlockLineType,
}

impl CodeBlockLine {
    pub fn new(ptr: u16, r#type: CodeBlockLineType) -> Self {
        CodeBlockLine { ptr, r#type }
    }

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
pub struct LineSegment<'a> {
    pub line: Line<'a>,              // wrapped line segment
    pub length: usize,               // length of the line segment
    pub last_segment: bool,          // last part of a line
    pub line_type: Option<LineType>, // type of line: Text or Code
    pub background: Option<Color>,   // default background color
}

impl<'a> LineSegment<'a> {
    pub fn new(
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

    pub fn spans_mut(&mut self) -> &mut Vec<Span<'a>> {
        &mut self.line.spans
    }
}

#[derive(Debug, Clone)]
pub struct TextDisplay<'a> {
    pub wrap_lines: Vec<LineSegment<'a>>, // Text (e.g., wrapped, highlighted) for display
    pub display_width: usize, // Width of the display area, used for wrapping
    pub column: usize,
    pub row: usize,
}

impl<'a> TextDisplay<'a> {
    pub fn new(display_width: usize) -> Self {
        TextDisplay {
            wrap_lines: Vec::new(),
            display_width,
            column: 0,
            row: 0,
        }
    }

    pub fn update(&mut self, text_lines: &Vec<TextLine>, cursor: &Cursor) {
        self.clear();
        let text_wrapper = TextWrapper::new(self.width());
        for (idx, line) in text_lines.iter().enumerate() {
            let text_str =
                line.segments().map(|s| s.text.as_str()).collect::<String>();
            let trailing_spaces =
                text_str.len() - text_str.trim_end_matches(' ').len();
            let wrapped_lines = text_wrapper.wrap_text_styled(line, None, None);
            if wrapped_lines.is_empty() {
                self.handle_empty_line(trailing_spaces, line.get_background());
            } else {
                // process wrapped lines
                self.process_wrapped_lines(
                    wrapped_lines,
                    idx,
                    trailing_spaces,
                    cursor,
                    line.get_background(),
                );
            }
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
                let column = cursor_position.saturating_sub(new_line_position);
                self.column = column;
                self.row = row;
                break;
            }
            new_line_position += line_length;
        }
        (self.column, self.row)
    }

    pub fn wrap_lines(&self) -> &[LineSegment<'a>] {
        &self.wrap_lines
    }

    pub fn width(&self) -> usize {
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

    pub fn set_display_width(&mut self, width: usize) {
        self.display_width = width;
    }

    pub fn get_column_row(&self) -> (usize, usize) {
        (self.column, self.row)
    }

    pub fn clear(&mut self) {
        self.wrap_lines.clear();
    }

    pub fn select_window_lines(&self, start: usize, end: usize) -> Vec<Line> {
        let renderer =
            DisplayWindowRenderer::new(self.wrap_lines(), self.width());
        renderer.render_lines(start, end)
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

            self.push_line(
                Line::from(Span::raw(spaces)),
                trailing_spaces,
                true,
                None,
                background,
            );
        } else {
            // add empty row
            self.push_line(
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
        trailing_spaces: usize,
        cursor: &Cursor,
        background: Option<Color>,
    ) {
        let (start_row, start_col, end_row, end_col) =
            cursor.get_selection_bounds();
        let mut char_pos = 0;

        let wrapped_lines_len = wrapped_lines.len();
        for (idx, line) in wrapped_lines.into_iter().enumerate() {
            let mut spans = Vec::with_capacity(line.segments.len());

            for segment in line.segments {
                let mut segment_start = 0;
                let mut current_style = segment.style;

                for (i, _) in segment.text.char_indices() {
                    let should_select = cursor.should_select(
                        unwrapped_line_index,
                        char_pos + i,
                        start_row,
                        start_col,
                        end_row,
                        end_col,
                    );

                    let effective_style = if should_select {
                        Some(current_style.unwrap_or_default().bg(Color::Blue))
                    } else {
                        current_style
                    };

                    if effective_style != current_style {
                        if segment_start < i {
                            spans.push(Span::styled(
                                segment.text[segment_start..i].to_string(),
                                current_style.unwrap_or_default(),
                            ));
                        }
                        segment_start = i;
                        current_style = effective_style;
                    }
                }

                if segment_start < segment.text.len() {
                    spans.push(Span::styled(
                        segment.text[segment_start..].to_string(),
                        current_style.unwrap_or_default(),
                    ));
                }

                char_pos += segment.text.len();
            }

            let last_segment = idx == wrapped_lines_len - 1;

            if last_segment && trailing_spaces > 0 {
                spans.push(Span::raw(" ".repeat(trailing_spaces)));
            }

            let current_line = Line::from(spans);
            let current_line_length =
                line.length + if last_segment { trailing_spaces } else { 0 };

            self.push_line(
                current_line,
                current_line_length,
                last_segment,
                None,
                background,
            );
            char_pos += 1; // account for newline character
        }
    }
}
