mod piece_table;
mod read_document;
mod read_write_document;
mod text_line;
mod text_wrapper;

use lumni::api::error::ApplicationError;
use ratatui::style::Style;
pub use read_document::ReadDocument;
pub use read_write_document::ReadWriteDocument;
pub use text_line::{TextLine, TextSegment};
pub use text_wrapper::TextWrapper;

pub use crate::external as lumni;

pub trait TextDocumentTrait {
    fn append_line(&mut self, line: TextLine);
    fn is_empty(&self) -> bool;
    fn empty(&mut self);
    fn append(&mut self, text: &str, style: Option<Style>);
    fn update_if_modified(&mut self);
    fn text_lines(&self) -> &[TextLine];
    fn get_text_lines_selection(
        &self,
        start: usize,
        end: Option<usize>,
    ) -> Option<&[TextLine]>;
    fn to_string(&self) -> String;
    fn delete(
        &mut self,
        idx: usize,
        len: usize,
    ) -> Result<(), ApplicationError>;
    fn insert(
        &mut self,
        idx: usize,
        text: &str,
        style: Option<Style>,
    ) -> Result<(), ApplicationError>;
    fn undo(&mut self) -> Result<(), ApplicationError>;
    fn redo(&mut self) -> Result<(), ApplicationError>;
}
