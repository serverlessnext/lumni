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

use crate::external as lumni;

pub trait TextDocumentTrait {
    fn from_text(lines: Vec<TextLine>) -> Self;
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
    fn max_row_idx(&self) -> usize {
        let rows = self.text_lines().len();
        rows.saturating_sub(1)
    }
    fn max_col_idx(&self, row: usize) -> usize {
        // Get the maximum column of a specific row. This is the line length + 1,
        // to account for either a newline character or empty space for the cursor.
        // Because line is 0-indexed we can skip add and substract
        let lines = self.text_lines();
        if let Some(line) = lines.get(row as usize) {
            line.get_length()
        } else {
            0
        }
    }
}
