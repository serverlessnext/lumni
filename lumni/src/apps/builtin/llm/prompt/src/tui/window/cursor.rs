use super::{text_document::TextLine, TextDocumentTrait};

#[derive(Debug, Clone)]
pub enum MoveCursor {
    Right(usize),
    Left(usize),
    Up(usize),
    Down(usize),
    StartOfLine,
    EndOfLine,
    StartOfFile,
    EndOfFile,
    EndOfFileEndOfLine,
}

#[derive(Debug, Clone)]
pub struct Cursor {
    pub col: usize,
    pub row: usize,
    anchor_col: usize,   // column for anchor, start of selection
    anchor_row: usize,   // row for anchor, start of selection
    show_cursor: bool, // show current cursor position
    selection_enabled: bool,
    desired_col: usize, // Desired column position, independent of actual line length
    real_position: usize, // real position of the cursor in the text buffer
}

impl Cursor {
    pub fn new() -> Self {
        Cursor {
            col: 0,
            row: 0,
            anchor_col: 0,
            anchor_row: 0,
            show_cursor: false,
            selection_enabled: false,
            desired_col: 0,
            real_position: 0,
        }
    }

    pub fn reset(&mut self) {
        // reset attributes that relate to cursor position back to defaults,
        // (note that show_cursor is not reset, as it is a display attribute)
        self.col = 0;
        self.row = 0;
        self.anchor_col = 0;
        self.anchor_row = 0;
        self.selection_enabled = false;
        self.desired_col = 0;
        self.real_position = 0;
    }

    pub fn real_position(&self) -> usize {
        self.real_position
    }

    pub fn set_visibility(&mut self, visible: bool) {
        self.show_cursor = visible;
    }

    pub fn show_cursor(&mut self) -> bool {
        self.show_cursor
    }

    pub fn selection_enabled(&self) -> bool {
        self.selection_enabled
    }

    pub fn set_selection_anchor(&mut self, enable: bool) {
        if enable {
            self.set_anchor_position();
        }
        self.selection_enabled = enable;
    }

    pub fn move_cursor<T>(
        &mut self,
        direction: MoveCursor,
        text_document: &T,
        // keep cursor at desired column when jumping to next line. This is used to prevent
        // cursor from jumping to the beginning when text is wrapped during editing
        keep_desired: bool,
    ) where
        T: TextDocumentTrait,
    {
        let max_row = text_document.max_row_idx();
        match direction {
            MoveCursor::Right(steps) => {
                // Move the cursor to the right by the specified number of characters
                for _ in 0..steps {
                    let max_col = text_document.max_col_idx(self.row);
                    if self.col < max_col {
                        self.col += 1;
                    } else if self.row < max_row {
                        // Move to the beginning of the next line
                        self.row += 1;
                        if keep_desired {
                            let max_col = text_document.max_col_idx(self.row);
                            self.col = std::cmp::min(self.desired_col, max_col);
                        } else {
                            self.col = 0;
                        }
                    } else {
                        // cursor is at the end of the last line
                    }
                }
                self.desired_col = self.col;
            }
            MoveCursor::Left(chars) => {
                // Move the cursor to the left by the specified number of characters
                for _ in 0..chars {
                    if self.col > 0 {
                        self.col -= 1;
                    } else if self.row > 0 {
                        // Move to the end of the previous line
                        self.row -= 1;
                        self.col = text_document.max_col_idx(self.row);
                    }
                }
            }
            MoveCursor::Up(lines) => {
                let current_row = self.row;
                let new_row = self.row.saturating_sub(lines);
                self.row = new_row;

                let max_col = text_document.max_col_idx(self.row);
                self.col = std::cmp::min(self.desired_col, max_col);

                // If moving up a single line and the cursor cannot move further up,
                // ensure the cursor moves to the start of the line
                if lines == 1 && new_row == 0 && current_row == new_row {
                    self.col = 0;
                    self.desired_col = 0;
                }
            }
            MoveCursor::Down(lines) => {
                let current_row = self.row;
                let new_row =
                    std::cmp::min(self.row.saturating_add(lines), max_row);
                self.row = new_row;

                let max_col = text_document.max_col_idx(self.row);
                self.col = std::cmp::min(self.desired_col, max_col);

                // when moving down a single line, and cant move further,
                // move cursor to the end of the line
                if lines == 1 && new_row == max_row && current_row == new_row {
                    self.col = max_col;
                    self.desired_col = max_col;
                }
            }
            MoveCursor::StartOfLine => {
                self.col = 0;
                self.desired_col = self.col;
            }
            MoveCursor::EndOfLine => {
                self.col = text_document.max_col_idx(self.row);
                self.desired_col = self.col;
            }
            MoveCursor::StartOfFile => {
                self.row = 0;
                self.col = 0;
                self.desired_col = self.col
            }
            MoveCursor::EndOfFile => {
                self.row = max_row;
                self.col = 0;
                self.desired_col = self.col
            }
            MoveCursor::EndOfFileEndOfLine => {
                self.row = max_row;
                self.col = text_document.max_col_idx(self.row);
                self.desired_col = self.col;
            }
        }
    }

    pub fn set_anchor_position(&mut self) {
        self.anchor_col = self.col;
        self.anchor_row = self.row;
    }

    pub fn get_selection_bounds(&self) -> (usize, usize, usize, usize) {
        // Determine the correct order for start and end positions
        if self.row < self.anchor_row
            || (self.row == self.anchor_row && self.col < self.anchor_col)
        {
            (
                self.row as usize,
                self.col as usize,
                self.anchor_row as usize,
                self.anchor_col as usize,
            )
        } else {
            (
                self.anchor_row as usize,
                self.anchor_col as usize,
                self.row as usize,
                self.col as usize,
            )
        }
    }

    pub fn should_select(
        &self,
        current_row: usize,
        j: usize,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) -> bool {
        (current_row > start_row && current_row < end_row)
            || (current_row == start_row
                && current_row == end_row
                && j >= start_col
                && j <= end_col)
            || (current_row == start_row
                && j >= start_col
                && current_row < end_row)
            || (current_row == end_row
                && j <= end_col
                && current_row > start_row)
    }

    pub fn update_real_position(&mut self, lines: &[TextLine]) {
        // compute the cursor position in underlying text based
        // on the current row and column
        let mut position = 0;
        for (index, line) in lines.iter().enumerate() {
            if index < self.row {
                // row before the current row
                position += line.get_length() + 1; // account for newline character
            } else if index == self.row {
                // current row
                position += self.col; // add columns for the current row
                break;
            }
        }
        self.real_position = position;
    }
}