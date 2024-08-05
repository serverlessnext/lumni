use lumni::api::error::ApplicationError;
use ratatui::style::Style;

use super::text_line::TextLine;
use super::TextDocumentTrait;
use crate::external as lumni;

#[derive(Debug)]
pub struct ReadDocument {
    lines: Vec<TextLine>,
    modified: bool,
}

impl ReadDocument {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            modified: false,
        }
    }
}

impl TextDocumentTrait for ReadDocument {
    fn from_text(lines: Vec<TextLine>) -> Self {
        Self {
            lines,
            modified: false,
        }
    }

    fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    fn empty(&mut self) {
        self.lines.clear();
        self.modified = true;
    }

    fn append(&mut self, text: &str, style: Option<Style>) {
        let mut current_line = if let Some(last) = self.lines.last_mut() {
            if !last.to_string().ends_with('\n') {
                last
            } else {
                self.lines.push(TextLine::new());
                self.lines.last_mut().unwrap()
            }
        } else {
            self.lines.push(TextLine::new());
            self.lines.last_mut().unwrap()
        };

        for ch in text.chars() {
            if ch == '\n' {
                self.lines.push(TextLine::new());
                current_line = self.lines.last_mut().unwrap();
            } else {
                current_line.add_segment(ch.to_string(), style.clone());
            }
        }
        self.modified = true;
    }

    fn update_if_modified(&mut self) {
        self.modified = false;
    }

    fn text_lines(&self) -> &[TextLine] {
        &self.lines
    }

    fn get_text_lines_selection(
        &self,
        start: usize,
        end: Option<usize>,
    ) -> Option<&[TextLine]> {
        let end = end.unwrap_or(self.lines.len());
        if start < self.lines.len() && start <= end {
            Some(&self.lines[start..end])
        } else {
            None
        }
    }

    fn to_string(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    // These methods are not supported in ReadDocument
    fn delete(
        &mut self,
        _idx: usize,
        _len: usize,
    ) -> Result<(), ApplicationError> {
        Err(ApplicationError::NotImplemented(
            "Delete not supported in ReadDocument".to_string(),
        ))
    }
    fn insert(
        &mut self,
        _idx: usize,
        _text: &str,
        _style: Option<Style>,
    ) -> Result<(), ApplicationError> {
        Err(ApplicationError::NotImplemented(
            "Insert not supported in ReadDocument".to_string(),
        ))
    }
    fn undo(&mut self) -> Result<(), ApplicationError> {
        Err(ApplicationError::NotImplemented(
            "Undo not supported in ReadDocument".to_string(),
        ))
    }
    fn redo(&mut self) -> Result<(), ApplicationError> {
        Err(ApplicationError::NotImplemented(
            "Redo not supported in ReadDocument".to_string(),
        ))
    }
}
