use ratatui::style::{Color, Style};

#[derive(Clone, Debug, PartialEq)]
pub struct StyledText {
    pub content: String,
    pub style: Option<Style>,
}

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Insert {
        index: usize,
        length: usize,
        style: Option<Style>,
    },
    #[allow(dead_code)]
    // TODO: implement delete action
    Delete {
        index: usize,
        content: String,
        style: Option<Style>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextSegment {
    pub text: String,
    pub style: Option<Style>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    segments: Vec<TextSegment>,
    length: usize,
    background: Option<Color>,
}

impl TextLine {
    pub fn new() -> Self {
        TextLine {
            segments: Vec::new(),
            length: 0,
            background: None,
        }
    }

    pub fn add_segment(&mut self, text: String, style: Option<Style>) {
        self.length += text.len();

        if let Some(last) = self.segments.last_mut() {
            // update the length of the last segment
            if last.style == style {
                // Append text to the last segment if styles are the same
                last.text.push_str(&text);
                return;
            }
        }
        // Otherwise, create a new segment
        self.segments.push(TextSegment { text, style });
        if let Some(style) = style {
            self.background = style.bg;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn segments(&self) -> impl Iterator<Item = &TextSegment> {
        self.segments.iter()
    }

    pub fn get_length(&self) -> usize {
        self.length
    }

    pub fn get_background(&self) -> Option<Color> {
        self.background
    }

    pub fn to_string(&self) -> String {
        let mut content = String::new();
        for segment in &self.segments {
            content.push_str(&segment.text);
        }
        content
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PieceTable {
    text_lines: Vec<TextLine>, // text split into lines with styles
    add: String,               // All text that has been added
    pieces: Vec<Piece>, // Pieces of text from either original or add buffer
    undo_stack: Vec<Action>, // Stack for undoing actions
    redo_stack: Vec<Action>, // Stack for redoing actions
    modified: bool,     // Flag to indicate if the text has been modified
}

#[derive(Clone, Debug, PartialEq)]
struct Piece {
    source: SourceBuffer,
    start: usize,
    length: usize,
    style: Option<Style>,
}

#[derive(Clone, Debug, PartialEq)]
enum SourceBuffer {
    Add,
}

impl PieceTable {
    pub fn new() -> Self {
        Self {
            text_lines: Vec::new(),
            add: String::new(),
            pieces: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            modified: false,
        }
    }

    pub fn from_text(segments: Vec<TextSegment>) -> Self {
        let mut piece_table = Self::new();

        if segments.is_empty() {
            return piece_table;
        }

        let mut new_pieces = Vec::new();
        let mut new_lines = Vec::new();
        let mut current_line = TextLine::new();
        let mut add_start = 0;

        for segment in segments {
            // Append the content to the add buffer
            piece_table.add.push_str(&segment.text);

            // Create a new piece for this text
            new_pieces.push(Piece {
                source: SourceBuffer::Add,
                start: add_start,
                length: segment.text.len(),
                style: segment.style.clone(),
            });

            // Update text_lines
            for ch in segment.text.chars() {
                if ch == '\n' {
                    if !current_line.is_empty() {
                        new_lines.push(current_line);
                        current_line = TextLine::new();
                    } else {
                        // Empty line, set background if style is available
                        if let Some(s) = &segment.style {
                            current_line.background = s.bg;
                        }
                        new_lines.push(current_line);
                        current_line = TextLine::new();
                    }
                } else {
                    current_line
                        .add_segment(ch.to_string(), segment.style.clone());
                }
            }

            add_start += segment.text.len();
        }

        // Add the last line if it's not empty
        if !current_line.is_empty() {
            new_lines.push(current_line);
        }

        piece_table.pieces = new_pieces;
        piece_table.text_lines = new_lines;
        piece_table.modified = false; // It's not modified, it's the initial state

        piece_table
    }

    pub fn text_lines(&self) -> &[TextLine] {
        &self.text_lines
    }

    pub fn get_text_lines_selection(
        &self,
        start: usize,
        end: Option<usize>,
    ) -> Option<&[TextLine]> {
        if start >= self.text_lines.len() {
            return None; // start index out of bounds
        }

        let end_index = match end {
            Some(e) => (e + 1).min(self.text_lines.len()), // add one to make it inclusive
            None => self.text_lines.len(), // use the end of the text_lines
        };
        Some(&self.text_lines[start..end_index])
    }

    pub fn to_string(&self) -> String {
        let mut content = String::new();
        for piece in &self.pieces {
            let text = match piece.source {
                SourceBuffer::Add => {
                    &self.add[piece.start..piece.start + piece.length]
                }
            };
            content.push_str(text);
        }
        content
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    pub fn empty(&mut self) {
        self.add.clear();
        self.pieces.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.modified = true;
    }

    pub fn insert(
        &mut self,
        idx: usize,
        text: &str,
        style: Option<Style>,
        is_redo: bool,
    ) {
        self.modified = true;

        let add_start = self.add.len(); // start index of the text in the `add` buffer.
        self.add.push_str(&text); // append to the `add` buffer.

        let mut new_pieces = Vec::new();
        let mut offset = 0;
        let mut insertion_handled = false;

        for piece in &self.pieces {
            if offset + piece.length <= idx && !insertion_handled {
                new_pieces.push(piece.clone());
            } else if offset > idx && !insertion_handled {
                new_pieces.push(Piece {
                    source: SourceBuffer::Add,
                    start: add_start,
                    length: text.len(),
                    style: style.clone(),
                });
                insertion_handled = true;
                new_pieces.push(piece.clone());
            } else if offset <= idx && idx < offset + piece.length {
                let first_part_length = idx - offset;
                if first_part_length > 0 {
                    new_pieces.push(Piece {
                        source: piece.source.clone(),
                        start: piece.start,
                        length: first_part_length,
                        style: piece.style.clone(),
                    });
                }

                new_pieces.push(Piece {
                    source: SourceBuffer::Add,
                    start: add_start,
                    length: text.len(),
                    style: style.clone(),
                });

                if idx < offset + piece.length {
                    new_pieces.push(Piece {
                        source: piece.source.clone(),
                        start: piece.start + first_part_length,
                        length: piece.length - first_part_length,
                        style: piece.style.clone(),
                    });
                }
                insertion_handled = true;
            } else {
                new_pieces.push(piece.clone());
            }
            offset += piece.length;
        }

        if !insertion_handled {
            new_pieces.push(Piece {
                source: SourceBuffer::Add,
                start: add_start,
                length: text.len(),
                style: style.clone(),
            });
        }
        self.pieces = new_pieces;

        if !is_redo {
            self.redo_stack.clear();
        }

        self.undo_stack.push(Action::Insert {
            index: idx,
            length: text.len(),
            style,
        });
    }

    pub fn delete(&mut self, idx: usize, length: usize) {
        let mut offset = 0;
        let mut new_pieces = vec![];

        for (i, piece) in self.pieces.iter().enumerate() {
            let piece_end = offset + piece.length;

            if piece_end <= idx {
                new_pieces.push(piece.clone());
            } else if offset > idx + length {
                new_pieces.extend_from_slice(&self.pieces[i..]);
                break;
            } else {
                let start_overlap = std::cmp::max(idx, offset);
                let end_overlap = std::cmp::min(idx + length, piece_end);

                if start_overlap > offset {
                    new_pieces.push(Piece {
                        source: piece.source.clone(),
                        start: piece.start,
                        length: start_overlap - offset,
                        style: piece.style.clone(),
                    });
                }

                if end_overlap < piece_end {
                    new_pieces.push(Piece {
                        source: piece.source.clone(),
                        start: piece.start + (end_overlap - offset),
                        length: piece_end - end_overlap,
                        style: piece.style.clone(),
                    });
                }
            }

            offset += piece.length;
        }
        self.pieces = new_pieces;
        self.modified = true;
    }

    pub fn undo(&mut self) {
        if self.undo_stack.is_empty() {
            return;
        }

        if self.pieces.len() > 100 {
            self.consolidate_pieces();
        }

        if let Some(action) = self.undo_stack.pop() {
            match action {
                Action::Insert {
                    index,
                    length,
                    style,
                } => {
                    // Perform the undo by deleting the text that was inserted
                    self.delete(index, length);
                    // Move the undone insert action to the redo stack
                    self.redo_stack.push(Action::Insert {
                        index,
                        length,
                        style,
                    });
                }
                Action::Delete {
                    index,
                    content,
                    style,
                } => {
                    // Undo the delete by reinserting the deleted text
                    self.insert(index, &content, style, false);
                    // Move the undone delete action to the redo stack
                    self.redo_stack.push(Action::Delete {
                        index,
                        content,
                        style,
                    });
                }
            }
            self.modified = true;
        }
    }

    pub fn redo(&mut self) {
        if let Some(action) = self.redo_stack.pop() {
            match action {
                Action::Insert {
                    index,
                    length,
                    style,
                } => {
                    // Clone the text to be reinserted to avoid borrowing issues
                    let text_to_reinsert =
                        self.add[self.add.len() - length..].to_string();
                    // Perform the insert with the cloned text
                    self.insert(index, &text_to_reinsert, style, true);
                    // Push the action back to the undo stack
                    self.undo_stack.push(Action::Insert {
                        index,
                        length,
                        style,
                    });
                }
                Action::Delete {
                    index,
                    content,
                    style,
                } => {
                    // Redo a delete by deleting the text that was previously reinserted
                    self.delete(index, content.len());
                    // Push the action back to the undo stack
                    self.undo_stack.push(Action::Delete {
                        index,
                        content,
                        style,
                    });
                }
            }
            self.modified = true;
        }
    }

    fn consolidate_pieces(&mut self) {
        if self.pieces.len() < 10 {
            return;
        }

        let mut consolidated = Vec::new();
        let mut last = self.pieces[0].clone();

        for piece in &self.pieces[1..] {
            if last.source == piece.source
                && last.start + last.length == piece.start
                && last.style == piece.style
            // ensure style integrity
            {
                // Extend the last piece
                last.length += piece.length;
            } else {
                consolidated.push(last);
                last = piece.clone();
            }
        }

        consolidated.push(last); // push the last accumulated piece
        self.pieces = consolidated;
        self.modified = true;
    }

    pub fn append(&mut self, text: &str, style: Option<Style>) {
        // Determine the start index in the add buffer for the new text.
        let add_start = self.add.len();
        // Add the new text to the add buffer.
        self.add.push_str(text);

        // Create a new piece that represents this newly appended text.
        let new_piece = Piece {
            source: SourceBuffer::Add,
            start: add_start,
            length: text.len(),
            style,
        };

        // Append the new piece to the pieces list.
        self.pieces.push(new_piece);
        self.modified = true;
    }

    pub fn update_if_modified(&mut self) {
        // this function is called by the UI to make sure text_lines are updated,
        // updating the lines is only necessary if the text has been modified in between
        if self.modified {
            self.update_lines_styled();
            self.modified = false;
        }
    }

    fn update_lines_styled(&mut self) {
        self.text_lines.clear();
        let mut current_line_styled: Option<TextLine> = None;
        let mut current_text = String::new();
        let mut last_style: Option<Style> = None;

        // Flatten all piece data into a single list to process
        let mut pieces_data = Vec::new();
        for piece in &self.pieces {
            let text = match piece.source {
                SourceBuffer::Add => self.add
                    [piece.start..piece.start + piece.length]
                    .to_string(),
            };
            pieces_data.push((text, piece.style.clone()));
        }

        // if pieces_data is non-empty, current_line_styled will be some
        if !pieces_data.is_empty() {
            current_line_styled = Some(TextLine::new());
        }

        for (text, style) in pieces_data {
            for char in text.chars() {
                if char == '\n' {
                    // finalize previous line
                    if !current_text.is_empty() {
                        if let Some(ref mut line) = current_line_styled {
                            line.add_segment(
                                current_text.clone(),
                                last_style.clone(),
                            );
                        }
                        current_text.clear();
                    } else {
                        // previous line is empty - it will likely not have
                        // background set (done in add_segment), so set it here
                        if let Some(line) = current_line_styled.as_mut() {
                            if line.background.is_none() {
                                if let Some(style) = style {
                                    line.background = style.bg;
                                }
                            }
                        }
                    }
                    // Add the previous line to text_lines
                    if let Some(line) = current_line_styled.take() {
                        //line.background = last_style.and_then(|s| s.bg);
                        self.text_lines.push(line);
                    }
                    current_line_styled = Some(TextLine::new()); // new line
                    continue;
                }

                if last_style != style {
                    if !current_text.is_empty() {
                        if let Some(ref mut line) = current_line_styled {
                            line.add_segment(
                                current_text.clone(),
                                last_style.clone(),
                            );
                        }
                        current_text.clear();
                    }
                    last_style = style.clone();
                }
                current_text.push(char);
            }

            // After processing each piece, if there's remaining text, add it as a segment
            if !current_text.is_empty() {
                if let Some(ref mut line) = current_line_styled {
                    line.add_segment(current_text.clone(), last_style.clone());
                }
                current_text.clear();
            }
        }

        // Ensure the final line is added if it contains something
        // note this can also be an empty line
        if let Some(line) = current_line_styled {
            self.text_lines.push(line);
        }
    }
}
