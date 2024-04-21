
#[derive(Debug, Clone)]
pub enum MoveCursor {
    Right,
    Left,
    Up,
    Down,
}

#[derive(Debug, Clone)]
pub struct Cursor {
    pub col: u16,
    pub row: u16,
}

impl Cursor {
    pub fn new(col: u16, row: u16) -> Self {
        Cursor { col, row }
    }

    pub fn move_cursor(&mut self, direction: MoveCursor, max_col: u16, max_row: u16) {
        match direction {
            MoveCursor::Right => if self.col < max_col { self.col += 1 },
            MoveCursor::Left => if self.col > 0 { self.col -= 1 },
            MoveCursor::Up => if self.row > 0 { self.row -= 1 },
            MoveCursor::Down => if self.row < max_row { self.row += 1 },
        }
    }
}
