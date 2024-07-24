use lumni::api::error::ApplicationError;

use ratatui::style::Style;
use super::piece_table::PieceTable;
use super::text_line::{TextLine, TextSegment};

pub use crate::external as lumni;


pub trait TextDocumentTrait {
    fn append_line(&mut self, line: TextLine);

    fn is_empty(&self) -> bool;
    fn empty(&mut self);
    fn append(&mut self, text: &str, style: Option<Style>);
    fn delete(&mut self, idx: usize, len: usize);
    fn update_if_modified(&mut self);
    fn text_lines(&self) -> &[TextLine];
    fn get_text_lines_selection(&self, start: usize, end: Option<usize>) -> Option<&[TextLine]>;
    fn to_string(&self) -> String;
    fn insert(&mut self, _idx: usize, _text: &str, _style: Option<Style>) -> Result<(), ApplicationError> {
        Err(ApplicationError::NotImplemented("Operation not supported in append-only mode".to_string()))
    }
    fn undo(&mut self) -> Result<(), 
        ApplicationError> {
        Err(ApplicationError::NotImplemented("Operation not supported in append-only mode".to_string()))
    }
    fn redo(&mut self) -> Result<(), ApplicationError> {
        Err(ApplicationError::NotImplemented("Operation not supported in append-only mode".to_string()))
    }
}

#[derive(Debug)]
pub struct ReadWriteDocument {
    piece_table: PieceTable,
}

impl ReadWriteDocument {
    pub fn new() -> Self {
        Self { piece_table: PieceTable::new() }
    }

    pub fn from_text(segments: Vec<TextSegment>) -> Self {
        Self { piece_table: PieceTable::from_text(segments) }
    }
}

impl TextDocumentTrait for ReadWriteDocument {
    fn append_line(&mut self, line: TextLine) {
        self.piece_table.append(&line.to_string(), None);
        self.piece_table.update_if_modified();
    }

    fn is_empty(&self) -> bool {
        self.piece_table.is_empty()
    }

    fn empty(&mut self) {
        self.piece_table.empty()
    }

    fn append(&mut self, text: &str, style: Option<Style>) {
        self.piece_table.append(text, style);
    }

    fn delete(&mut self, idx: usize, len: usize) {
        self.piece_table.delete(idx, len);
    }

    fn update_if_modified(&mut self) {
        self.piece_table.update_if_modified();
    }

    fn text_lines(&self) -> &[TextLine] {
        self.piece_table.text_lines()
    }

    fn get_text_lines_selection(&self, start: usize, end: Option<usize>) -> Option<&[TextLine]> {
        self.piece_table.get_text_lines_selection(start, end)
    }

    fn to_string(&self) -> String {
        self.piece_table.to_string()
    }

    fn insert(&mut self, idx: usize, text: &str, style: Option<Style>) -> Result<(), ApplicationError> {
        self.piece_table.insert(idx, text, style, false);
        self.piece_table.update_if_modified();
        Ok(())
    }

    fn undo(&mut self) -> Result<(), ApplicationError> {
        self.piece_table.undo();
        Ok(())
    }

    fn redo(&mut self) -> Result<(), ApplicationError> {
        self.piece_table.redo();
        Ok(())
    }
}
