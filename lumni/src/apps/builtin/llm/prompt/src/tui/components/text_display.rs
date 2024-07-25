use ratatui::style::Color;
use ratatui::text::{Line, Span};

use super::cursor::Cursor;

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

    pub fn push_line(
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
}
