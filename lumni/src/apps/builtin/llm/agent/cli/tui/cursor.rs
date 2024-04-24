#[derive(Debug, Clone)]
pub enum MoveCursor {
    Right,
    Left,
    Up,
    Down,
    BeginLine,
    EndLine,
    TopOfFile,
    EndOfFile,
}

#[derive(Debug, Clone)]
pub struct Cursor {
    pub col: u16,
    pub row: u16,
    fixed_col: u16, // fixed column for anchor
    fixed_row: u16, // fixed row for anchor
    show_cursor: bool,  // show current cursor position
    is_highlighting_enabled: bool,
}

impl Cursor {
    pub fn new(col: u16, row: u16) -> Self {
        Cursor {
            col,
            row,
            fixed_col: col,
            fixed_row: row,
            show_cursor: true,
            is_highlighting_enabled: false,
        }
    }

    pub fn show_cursor(&mut self) -> bool {
        self.show_cursor
    }

    pub fn is_highlighting_enabled(&self) -> bool {
        self.is_highlighting_enabled
    }

    pub fn toggle_highlighting(&mut self) {
        if !self.is_highlighting_enabled {
            // mark current position as new fixed position
            self.set_fixed_position();
        }
        self.is_highlighting_enabled = !self.is_highlighting_enabled;
    }

    pub fn set_highlighting(&mut self, enable: bool) {
        self.is_highlighting_enabled = enable;
    }

    pub fn move_cursor(
        &mut self,
        direction: MoveCursor,
        max_col: u16,
        max_row: u16,
    ) {
        match direction {
            MoveCursor::Right => if self.col < max_col { self.col += 1; },
            MoveCursor::Left => if self.col > 0 { self.col -= 1; },
            MoveCursor::Up => if self.row > 0 { self.row -= 1; },
            MoveCursor::Down => if self.row < max_row { self.row += 1; },
            MoveCursor::BeginLine => self.col = 0,
            MoveCursor::EndLine => self.col = max_col,
            MoveCursor::TopOfFile => { self.row = 0; self.col = 0 },
            MoveCursor::EndOfFile => { self.row = max_row; self.col = 0 },
        }
    }

    pub fn set_fixed_position(&mut self) {
        self.fixed_col = self.col;
        self.fixed_row = self.row;
    }

    pub fn get_highlight_bounds(&self) -> (usize, usize, usize, usize) {
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

    pub fn should_highlight(
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