use std::mem;

use ratatui::style::Style;

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Insert {
        index: usize,
        length: usize,
        style: Option<Style>,
    },
    Delete {
        index: usize,
        content: String,
        style: Option<Style>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum InsertMode {
    Append,
    Insert(usize), // Include the index where the insertion starts
}

#[derive(Clone, Debug, PartialEq)]
struct Cache {
    content: String,
    insert_idx: Option<usize>,
    style: Option<Style>,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            content: String::new(),
            insert_idx: None,
            style: None,
        }
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.insert_idx = None;
        self.style = None;
    }

    pub fn is_active(&self) -> bool {
        self.insert_idx.is_some()
    }

    pub fn start(&mut self, idx: usize, style: Option<Style>) {
        self.content.clear();
        self.insert_idx = Some(idx);
        self.style = style;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextSegment {
    text: String,
    style: Option<Style>,
}

impl TextSegment {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn style(&self) -> Option<Style> {
        self.style
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    segments: Vec<TextSegment>,
    length: usize,
}

impl TextLine {
    pub fn new() -> Self {
        TextLine {
            segments: Vec::new(),
            length: 0,
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
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn segments(&self) -> impl Iterator<Item = &TextSegment> {
        self.segments.iter()
    }

    pub fn length(&self) -> usize {
        self.length
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PieceTable {
    original: String, // The original unmodified text
    text_lines: Vec<TextLine>, // text split into lines with styles
    add: String,      // All text that has been added
    pieces: Vec<Piece>, // Pieces of text from either original or add buffer
    cache: Cache,     // Temporary buffer for caching many (small) insertions
    undo_stack: Vec<Action>, // Stack for undoing actions
    redo_stack: Vec<Action>, // Stack for redoing actions
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
    Original,
    Add,
}

impl PieceTable {
    pub fn new() -> Self {
        Self {
            original: "".to_string(),
            text_lines: Vec::new(),
            add: String::new(),
            pieces: Vec::new(),
            cache: Cache::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn text_lines(&self) -> &[TextLine] {
        &self.text_lines
    }

    pub fn to_string(&self) -> String {
        let mut content = String::new();
        for piece in &self.pieces {
            let text = match piece.source {
                SourceBuffer::Original => {
                    &self.original[piece.start..piece.start + piece.length]
                }
                SourceBuffer::Add => {
                    &self.add[piece.start..piece.start + piece.length]
                }
            };
            content.push_str(text);
        }
        content
    }

    pub fn empty(&mut self) {
        self.original.clear();
        self.add.clear();
        self.pieces.clear();
        self.cache.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    fn insert(
        &mut self,
        idx: usize,
        text: &str,
        style: Option<Style>,
        is_redo: bool,
    ) {
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

        if !self.cache.content.is_empty() {
            self.commit_insert_cache();
        }

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
    }

    fn committed_content_length(&self) -> usize {
        self.pieces.iter().map(|p| p.length).sum()
    }

    fn start_insert_cache(&mut self, mode: InsertMode, style: Option<Style>) {
        let insert_idx = match mode {
            InsertMode::Append => {
                // Set the index to the current length of content
                self.committed_content_length()
            }
            InsertMode::Insert(idx) => {
                // Set the index to the specified index
                idx
            }
        };
        self.cache.start(insert_idx, style);
    }

    pub fn cache_insert(
        &mut self,
        text: &str,
        idx: Option<usize>,
        style: Option<Style>,
    ) {
        // Check if there is an active cache with matching style
        let style_change = self.cache.style.as_ref() != style.as_ref();
        if self.cache.is_active() && style_change {
            // If styles differ and there is active content in the cache, commit it
            self.commit_insert_cache();
        }

        // Determine the current end index of the cached content
        let current_end_idx = self.cache.insert_idx.map_or(None, |start_idx| {
            Some(start_idx + self.cache.content.len())
        });

        match (idx, current_end_idx) {
            (Some(new_idx), Some(end_idx)) if new_idx == end_idx => {
                // If the new index matches exactly where the current cache ends, just append
                self.cache.content.push_str(text);
            }
            (Some(new_idx), _) => {
                if !self.cache.content.is_empty() {
                    // commit existing cache because new will be created
                    self.commit_insert_cache();
                }
                let initial_text: String;
                if self.committed_content_length() < new_idx {
                    // insert exceeds the current content length, fill with spaces
                    let diff = new_idx - self.committed_content_length();
                    let fill_text = " ".repeat(diff);
                    self.start_insert_cache(
                        InsertMode::Insert(self.committed_content_length()),
                        style,
                    );
                    initial_text = format!("{}{}", fill_text, text);
                } else {
                    self.start_insert_cache(InsertMode::Insert(new_idx), style);
                    initial_text = text.to_string();
                }

                self.cache.content.push_str(&initial_text);
            }
            (None, Some(_)) => {
                // If no specific index is provided but a cache exists, append to it
                self.cache.content.push_str(text);
            }
            (None, None) => {
                // If no specific index and no existing cache, start appending at the end of the content
                self.start_insert_cache(InsertMode::Append, style);
                self.cache.content.push_str(text);
            }
        }
    }

    pub fn commit_insert_cache(&mut self) -> String {
        if let (Some(idx), style) =
            (self.cache.insert_idx, self.cache.style.clone())
        {
            if !self.cache.content.is_empty() {
                // Take ownership of insert_cache, leaving an empty string behind
                let cache_content = mem::take(&mut self.cache.content);
                self.insert(idx, &cache_content, style, false);
                self.cache.clear();
                return cache_content;
            }
            self.cache.clear();
        }
        String::new() // return an empty string if no cache was committed
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
    }

    pub fn update_lines_styled(&mut self) {
        // ensure pending cache is committed before updating lines
        self.commit_insert_cache();
        self.text_lines.clear();
        let mut current_line_styled = TextLine::new();
        let mut current_text = String::new();
        let mut last_style: Option<Style> = None;

        // Flatten all piece data into a single list to process
        let mut pieces_data = Vec::new();
        for piece in &self.pieces {
            let text = match piece.source {
                SourceBuffer::Original => self.original
                    [piece.start..piece.start + piece.length]
                    .to_string(),
                SourceBuffer::Add => self.add
                    [piece.start..piece.start + piece.length]
                    .to_string(),
            };
            pieces_data.push((text, piece.style.clone()));
        }

        for (text, style) in pieces_data {
            for char in text.chars() {
                if char == '\n' {
                    // Handle empty lines: push current text or a truly empty segment
                    if current_text.is_empty() && current_line_styled.is_empty()
                    {
                        current_line_styled
                            .add_segment(String::new(), last_style.clone()); // Adding an empty segment for an empty line
                    } else if !current_text.is_empty() {
                        current_line_styled.add_segment(
                            current_text.clone(),
                            last_style.clone(),
                        );
                        current_text.clear();
                    }
                    self.text_lines.push(current_line_styled.clone());
                    current_line_styled = TextLine::new(); // Start a new styled line
                    continue;
                }

                if last_style != style {
                    if !current_text.is_empty() {
                        current_line_styled.add_segment(
                            current_text.clone(),
                            last_style.clone(),
                        );
                        current_text.clear();
                    }
                    last_style = style.clone();
                }
                current_text.push(char);
            }
        }

        // Append any remaining text to the last line
        if !current_text.is_empty() {
            current_line_styled.add_segment(current_text, last_style);
        }

        // Ensure the final line is added if it contains segments
        if !current_line_styled.is_empty() {
            self.text_lines.push(current_line_styled);
        }
    }
}
