use std::mem;

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Insert {
        index: usize,
        length: usize,
        author: String,
    },
    Delete {
        index: usize,
        content: String,
        author: String,
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
    author: Option<String>,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            content: String::new(),
            insert_idx: None,
            author: None,
        }
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.insert_idx = None;
        self.author = None;
    }

    pub fn is_active(&self) -> bool {
        self.insert_idx.is_some()
    }

    pub fn start(&mut self, idx: usize, author: &str) {
        self.content.clear();
        self.insert_idx = Some(idx);
        self.author = Some(author.to_string());
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PieceTable {
    original: String,        // The original unmodified text
    lines: Vec<String>,      // text split into lines
    add: String,             // All text that has been added
    pieces: Vec<Piece>, // Pieces of text from either original or add buffer
    cache: Cache,       // Temporary buffer for caching many (small) insertions
    undo_stack: Vec<Action>, // Stack for undoing actions
    redo_stack: Vec<Action>, // Stack for redoing actions
}

#[derive(Clone, Debug, PartialEq)]
struct Piece {
    source: SourceBuffer,
    start: usize,
    length: usize,
    author: String,
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
            lines: Vec::new(),
            add: String::new(),
            pieces: Vec::new(),
            cache: Cache::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn empty(&mut self) {
        self.original.clear();
        self.add.clear();
        self.pieces.clear();
        self.cache.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    fn insert(&mut self, idx: usize, text: &str, author: &str, is_redo: bool) {
        let add_start = self.add.len();
        self.add.push_str(text);
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
                    author: author.to_string(),
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
                        author: piece.author.clone(),
                    });
                }

                new_pieces.push(Piece {
                    source: SourceBuffer::Add,
                    start: add_start,
                    length: text.len(),
                    author: author.to_string(),
                });

                if idx < offset + piece.length {
                    new_pieces.push(Piece {
                        source: piece.source.clone(),
                        start: piece.start + first_part_length,
                        length: piece.length - first_part_length,
                        author: piece.author.clone(),
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
                author: author.to_string(),
            });
        }
        self.pieces = new_pieces;

        if !is_redo {
            self.redo_stack.clear();
        }

        self.undo_stack.push(Action::Insert {
            index: idx,
            length: text.len(),
            author: author.to_string(),
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
                        author: piece.author.clone(),
                    });
                }

                if end_overlap < piece_end {
                    new_pieces.push(Piece {
                        source: piece.source.clone(),
                        start: piece.start + (end_overlap - offset),
                        length: piece_end - end_overlap,
                        author: piece.author.clone(),
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
                    author,
                } => {
                    // Perform the undo by deleting the text that was inserted
                    self.delete(index, length);
                    // Move the undone insert action to the redo stack
                    self.redo_stack.push(Action::Insert {
                        index,
                        length,
                        author,
                    });
                }
                Action::Delete {
                    index,
                    content,
                    author,
                } => {
                    // Undo the delete by reinserting the deleted text
                    self.insert(index, &content, &author, false);
                    // Move the undone delete action to the redo stack
                    self.redo_stack.push(Action::Delete {
                        index,
                        content,
                        author,
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
                    author,
                } => {
                    // Clone the text to be reinserted to avoid borrowing issues
                    let text_to_reinsert =
                        self.add[self.add.len() - length..].to_string();
                    // Perform the insert with the cloned text
                    self.insert(index, &text_to_reinsert, &author, true);
                    // Push the action back to the undo stack
                    self.undo_stack.push(Action::Insert {
                        index,
                        length,
                        author,
                    });
                }
                Action::Delete {
                    index,
                    content,
                    author,
                } => {
                    // Redo a delete by deleting the text that was previously reinserted
                    self.delete(index, content.len());
                    // Push the action back to the undo stack
                    self.undo_stack.push(Action::Delete {
                        index,
                        content,
                        author,
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
                && last.author == piece.author
            // ensure author integrity
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

    fn start_insert_cache(&mut self, mode: InsertMode, author: &str) {
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
        self.cache.start(insert_idx, author);
    }

    pub fn cache_insert(
        &mut self,
        text: &str,
        idx: Option<usize>,
        author: &str,
    ) {
        // Check if there is an active cache with matching author
        if self
            .cache
            .author
            .as_ref()
            .map_or(false, |current_author| current_author != author)
        {
            if self.cache.is_active() {
                self.commit_insert_cache();
            }
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
                    self.commit_insert_cache();
                }

                let initial_text: String;
                if self.committed_content_length() < new_idx {
                    // insert exceeds the current content length, fill with spaces
                    let diff = new_idx - self.committed_content_length();
                    let fill_text = " ".repeat(diff);
                    self.start_insert_cache(
                        InsertMode::Insert(self.committed_content_length()),
                        author,
                    );
                    initial_text = format!("{}{}", fill_text, text);
                } else {
                    self.start_insert_cache(
                        InsertMode::Insert(new_idx),
                        author,
                    );
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
                self.start_insert_cache(InsertMode::Append, author);
                self.cache.content.push_str(text);
            }
        }
    }

    pub fn commit_insert_cache(&mut self) -> String {
        if let (Some(idx), Some(author)) =
            (self.cache.insert_idx, self.cache.author.clone())
        {
            if !self.cache.content.is_empty() {
                // Take ownership of insert_cache, leaving an empty string behind
                let cache_content = mem::take(&mut self.cache.content);
                self.insert(idx, &cache_content, &author, false);
                self.cache.clear();
                return cache_content;
            }
        }
        String::new() // return an empty string if no cache was committed
    }

    pub fn append(&mut self, text: &str, author: &str) {
        // Determine the start index in the add buffer for the new text.
        let add_start = self.add.len();
        // Add the new text to the add buffer.
        self.add.push_str(text);

        // Create a new piece that represents this newly appended text.
        let new_piece = Piece {
            source: SourceBuffer::Add,
            start: add_start,
            length: text.len(),
            author: author.to_string(),
        };

        // Append the new piece to the pieces list.
        self.pieces.push(new_piece);
    }

    pub fn update_lines(&mut self) {
        // Collect the content of each piece into a single string
        let mut content_string = self
            .pieces
            .iter()
            .map(|p| match p.source {
                SourceBuffer::Original => {
                    &self.original[p.start..p.start + p.length]
                }
                SourceBuffer::Add => &self.add[p.start..p.start + p.length],
            })
            .collect::<String>();

        // Check if there is an active insert cache and an index where it should be inserted
        if let Some(idx) = self.cache.insert_idx {
            if !self.cache.content.is_empty() {
                // Insert the cache content at the appropriate index within the content_string
                if idx >= content_string.len() {
                    // If the index is at or beyond the end of the current content, simply append it
                    content_string.push_str(&self.cache.content);
                } else {
                    // Otherwise, insert the cached text at the specified index
                    let (start, end) = content_string.split_at(idx);
                    content_string = [start, &self.cache.content, end].concat();
                }
            }
        }
        if content_string.is_empty() {
            self.lines = Vec::new();
        } else {
            self.lines = content_string
                .lines()
                .map(|line| line.to_string())
                .collect::<Vec<String>>();
        }
    }
}
