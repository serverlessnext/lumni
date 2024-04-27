use ratatui::text::Line;

#[derive(Debug, Clone)]
pub enum MoveCursor {
    Right,
    Left,
    Up,
    Down,
    StartOfLine,
    EndOfLine,
    TopOfFile,
    EndOfFile,
    EndOfFileEndOfLine,
    LinesForward(u16),
    LinesBackward(u16),
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
}

impl Cursor {
    pub fn new(col: u16, row: u16) -> Self {
        Cursor {
            col,
            row,
            fixed_col: col,
            fixed_row: row,
            show_cursor: true,
            selection_enabled: false,
            desired_col: col, // Initially, desired column is same as starting column
        }
    }

    pub fn reset(&mut self) {
        self.col = 0;
        self.row = 0;
        self.fixed_col = 0;
        self.fixed_row = 0;
        self.show_cursor = true;
        self.selection_enabled = false;
        self.desired_col = 0;
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
        display_text: &[Line],
    ) {
        let max_row = get_max_row(display_text);
        match direction {
            MoveCursor::Right => {
                let max_col = get_max_col(self.row, display_text);
                if self.col < max_col {
                    self.col += 1;
                }
                self.desired_col = self.col;
            }
            MoveCursor::Left => {
                if self.col > 0 {
                    self.col -= 1;
                }
                self.desired_col = self.col;
            }
            MoveCursor::Up => {
                if self.row > 0 {
                    self.row -= 1;
                    let max_col = get_max_col(self.row, display_text);
                    self.col = std::cmp::min(self.desired_col, max_col);
                }
            }
            MoveCursor::Down => {
                if self.row < max_row {
                    self.row += 1;
                    let max_col = get_max_col(self.row, display_text);
                    self.col = std::cmp::min(self.desired_col, max_col);
                } else {
                    // go to end of line
                    self.col = get_max_col(self.row, display_text);
                    self.desired_col = self.col;
                }
            }
            MoveCursor::StartOfLine => {
                self.col = 0;
                self.desired_col = self.col;
            }
            MoveCursor::EndOfLine => {
                self.col = get_max_col(self.row, display_text);
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
                self.col = get_max_col(self.row, display_text);
                self.desired_col = self.col;
            }
            MoveCursor::LinesBackward(n) => {
                self.row = self.row.saturating_sub(n);
                let max_col = get_max_col(self.row, display_text);
                self.col = std::cmp::min(self.desired_col, max_col);
            }
            MoveCursor::LinesForward(n) => {
                self.row = std::cmp::min(self.row.saturating_add(n), max_row);
                let max_col = get_max_col(self.row, display_text);
                self.col = std::cmp::min(self.desired_col, max_col);
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
}

fn get_max_col(row: u16, display_text: &[Line]) -> u16 {
    // Get the current row where the cursor is located.
    if let Some(line) = display_text.get(row as usize) {
        // Return the length of the line, considering all spans.
        line.spans
            .iter()
            .map(|span| span.content.len() as u16) // Calculate the length of each span
            .sum::<u16>() // Sum up the lengths of all spans
            .saturating_sub(1) // Subtract 1 because the cursor is 0-indexed
    } else {
        0 // If for some reason the line doesn't exist, return 0
    }
}

fn get_max_row(display_text: &[Line]) -> u16 {
    display_text.len() as u16 - 1
}
