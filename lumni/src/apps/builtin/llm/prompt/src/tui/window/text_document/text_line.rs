use ratatui::style::{Color, Style};

use super::simple_string::SimpleString;

#[derive(Clone, Debug, PartialEq)]
pub struct TextSegment {
    pub text: SimpleString,
    pub style: Option<Style>,
}

impl TextSegment {
    pub fn from_text<S: Into<SimpleString>>(
        text: S,
        style: Option<Style>,
    ) -> Self {
        TextSegment {
            text: text.into(),
            style,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    pub segments: Vec<TextSegment>,
    pub length: usize,
    pub background: Option<Color>,
}

impl TextLine {
    pub fn new() -> Self {
        TextLine {
            segments: Vec::new(),
            length: 0,
            background: None,
        }
    }

    pub fn from_text<S: Into<SimpleString>>(
        text: S,
        style: Option<Style>,
    ) -> Self {
        let mut line = TextLine::new();
        line.add_segment(text, style);
        line
    }

    pub fn add_segment<S: Into<SimpleString>>(
        &mut self,
        text: S,
        style: Option<Style>,
    ) {
        let text = text.into();
        self.length += text.len();
        if let Some(last) = self.segments.last_mut() {
            if last.style == style {
                // Concatenate the strings if they have the same style
                let new_text =
                    SimpleString::from(format!("{}{}", last.text, text));
                last.text = new_text;
                return;
            }
        }
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
        let mut content = String::with_capacity(self.length);
        for segment in &self.segments {
            content.push_str(segment.text.as_str());
        }
        content
    }
}
