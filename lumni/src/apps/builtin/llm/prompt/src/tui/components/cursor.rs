

#[derive(Debug, Clone)]
pub enum MoveCursor {
    Right(u16),
    Left(u16),
    Up(u16),
    Down(u16),
    StartOfLine,
    EndOfLine,
    TopOfFile,
    EndOfFile,
    EndOfFileEndOfLine,
}

#[derive(Debug, Clone)]
pub struct Cursor {
    pub col: u16,
    pub row: u16,
    fixed_col: u16,    // fixed column for anchor
    fixed_row: u16,    // fixed row for anchor
    show_cursor: bool, // show current cursor position
    selection_enabled: bool,
    desired_col: u16, // Desired column position, independent of actual line length
    real_position: usize, // real position of the cursor in the text buffer
}

impl Cursor {
    pub fn new(col: u16, row: u16, show_cursor: bool) -> Self {
        Cursor {
            col,
            row,
            fixed_col: col,
            fixed_row: row,
            show_cursor,
            selection_enabled: false,
            desired_col: col, // Initially, desired column is same as starting column
            real_position: 0,
        }
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

    pub fn set_selection(&mut self, enable: bool) {
        if enable {
            self.set_fixed_position();
        }
        self.selection_enabled = enable;
    }

    pub fn move_cursor(
        &mut self,
        direction: MoveCursor,
        text_lines: &[String],
        // keep cursor at desired column when jumping to next line. This is used to prevent
        // cursor from jumping to the beginning when text is wrapped during editing
        keep_desired: bool,   
    ) {
        let max_row = get_max_row(text_lines);
        match direction {
            MoveCursor::Right(steps) => {
                // Move the cursor to the right by the specified number of characters
                for _ in 0..steps {
                    let max_col = get_max_col(text_lines, self.row);
                    if self.col < max_col {
                        self.col += 1;
                    } else if self.row < get_max_row(text_lines) {
                        // Move to the beginning of the next line
                        self.row += 1;
                        if keep_desired {
                            let max_col = get_max_col(text_lines, self.row);
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
                        self.col = get_max_col(text_lines, self.row);
                    }
                }
            }
            MoveCursor::Up(lines) => {
                let current_row = self.row;
                let new_row = self.row.saturating_sub(lines);
                self.row = new_row;

                let max_col = get_max_col(text_lines, self.row);
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

                let max_col = get_max_col(text_lines, self.row);
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
                self.col = get_max_col(text_lines, self.row);
                self.desired_col = self.col;
            }
            MoveCursor::TopOfFile => {
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
                self.col = get_max_col(text_lines, self.row);
                self.desired_col = self.col;
            }
        }
    }

    pub fn set_fixed_position(&mut self) {
        self.fixed_col = self.col;
        self.fixed_row = self.row;
    }

    pub fn get_selection_bounds(&self) -> (usize, usize, usize, usize) {
        // Determine the correct order for start and end positions
        if self.row < self.fixed_row
            || (self.row == self.fixed_row && self.col < self.fixed_col)
        {
            (
                self.row as usize,
                self.col as usize,
                self.fixed_row as usize,
                self.fixed_col as usize,
            )
        } else {
            (
                self.fixed_row as usize,
                self.fixed_col as usize,
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

    pub fn update_real_position(
        &mut self,
        lines: &[String],
    ) {
        // compute the cursor position in underlying text based
        // on the current row and column
        let mut position = 0;
        for (index, line) in lines.iter().enumerate() {
            if index < self.row as usize {
                // row before the current row
                position += line.len() + 1; // account for newline character
            } else if index == self.row as usize {
                // current row
                position += self.col as usize; // add columns for the current row
                break;
            }
        }
        self.real_position = position;
    }
}

fn get_max_row(display_text: &[String]) -> u16 {
    display_text.len() as u16 - 1
}

pub fn get_max_col(lines: &[String], row: u16) -> u16 {
    // Get the maximum column of a specific row
    // check if row exists in self.lines, if not return 0
    if let Some(line) = lines.get(row as usize) {
        line.len() as u16
    } else {
        0
    }
}