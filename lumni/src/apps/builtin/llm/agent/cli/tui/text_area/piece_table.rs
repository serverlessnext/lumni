use std::mem;

#[derive(Clone, Debug)]
pub enum InsertMode {
    Append,
    Insert(usize), // Include the index where the insertion starts
}

#[derive(Clone, Debug, PartialEq)]
pub struct PieceTable {
    original: String,                // The original unmodified text
    add: String,                     // All text that has been added
    pieces: Vec<Piece>, // Pieces of text from either original or add buffer
    insert_cache: String, // Temporary buffer for caching many (small) insertions
    cache_insert_idx: Option<usize>, // The index at which to commit the cached inserts
}

#[derive(Clone, Debug, PartialEq)]
struct Piece {
    source: SourceBuffer, // Enum to reference either original or add buffer
    start: usize,         // Start index in the source
    length: usize,        // Length of the piece
}

#[derive(Clone, Debug, PartialEq)]
enum SourceBuffer {
    Original,
    Add,
}

impl PieceTable {
    pub fn new(text: &str) -> Self {
        Self {
            original: text.to_string(),
            add: String::new(),
            pieces: vec![Piece {
                source: SourceBuffer::Original,
                start: 0,
                length: text.len(),
            }],
            insert_cache: String::new(),
            cache_insert_idx: None,
        }
    }

    fn committed_content_length(&self) -> usize {
        self.pieces.iter().map(|p| p.length).sum()
    }

    pub fn start_insert_cache(&mut self, mode: InsertMode) {
        match mode {
            InsertMode::Append => {
                // Set the index to the current length of content
                self.cache_insert_idx = Some(self.committed_content_length());
            }
            InsertMode::Insert(idx) => {
                // Set the index to the specified index
                self.cache_insert_idx = Some(idx);
            }
        }
        self.insert_cache.clear();
    }

    pub fn cache_insert(&mut self, text: &str) {
        if self.cache_insert_idx.is_none() {
            // Automatically start an append cache if no index has been set
            self.start_insert_cache(InsertMode::Append);
        }
        self.insert_cache.push_str(text);
    }

    pub fn commit_insert_cache(&mut self) -> String {
        if let Some(idx) = self.cache_insert_idx {
            if !self.insert_cache.is_empty() {
                let cache_content = mem::take(&mut self.insert_cache); // Take ownership of insert_cache, leaving an empty string behind
                self.insert(idx, &cache_content); // Now we pass the owned string, no borrow of self here
                self.cache_insert_idx = None; // Reset the index after committing
                return cache_content;
            }
        }
        String::new() // return an empty string if no cache was committed
    }

    pub fn insert(&mut self, idx: usize, text: &str) {
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
                    });
                }

                new_pieces.push(Piece {
                    source: SourceBuffer::Add,
                    start: add_start,
                    length: text.len(),
                });

                if idx < offset + piece.length {
                    new_pieces.push(Piece {
                        source: piece.source.clone(),
                        start: piece.start + first_part_length,
                        length: piece.length - first_part_length,
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
            });
        }

        self.pieces = new_pieces;
    }

    pub fn append(&mut self, text: &str) {
        // Determine the start index in the add buffer for the new text.
        let add_start = self.add.len();
        // Add the new text to the add buffer.
        self.add.push_str(text);

        // Create a new piece that represents this newly appended text.
        let new_piece = Piece {
            source: SourceBuffer::Add,
            start: add_start,
            length: text.len(),
        };

        // Append the new piece to the pieces list.
        self.pieces.push(new_piece);
    }

    pub fn delete(&mut self, idx: usize, length: usize) {
        let mut new_pieces = Vec::new();
        let mut offset = 0;
        let mut remaining_length = length;

        for piece in &self.pieces {
            let end_idx = offset + piece.length;
            if end_idx <= idx || offset >= idx + remaining_length {
                new_pieces.push(piece.clone());
            } else {
                let start_overlap = std::cmp::max(idx, offset);
                let end_overlap =
                    std::cmp::min(idx + remaining_length, end_idx);

                if start_overlap > offset {
                    new_pieces.push(Piece {
                        source: piece.source.clone(),
                        start: piece.start,
                        length: start_overlap - offset,
                    });
                }
                if end_overlap < end_idx {
                    new_pieces.push(Piece {
                        source: piece.source.clone(),
                        start: piece.start + (end_overlap - offset),
                        length: end_idx - end_overlap,
                    });
                }
                remaining_length -= end_overlap - start_overlap;
            }
            offset += piece.length;
        }

        self.pieces = new_pieces;
    }

    pub fn content(&self) -> String {
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
        if let Some(idx) = self.cache_insert_idx {
            if !self.insert_cache.is_empty() {
                // Insert the cache content at the appropriate index within the content_string
                if idx >= content_string.len() {
                    // If the index is at or beyond the end of the current content, simply append it
                    content_string.push_str(&self.insert_cache);
                } else {
                    // Otherwise, insert the cached text at the specified index
                    let (start, end) = content_string.split_at(idx);
                    content_string = [start, &self.insert_cache, end].concat();
                }
            }
        }
        content_string
    }
}
