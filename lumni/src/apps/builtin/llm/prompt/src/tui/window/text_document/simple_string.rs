use std::borrow::Cow;
use std::ops::Deref;

use ratatui::text::Span;
use ratatui::style::Style;
use super::text_line::{TextLine, TextSegment};
use super::text_wrapper::TextWrapper;


#[derive(Clone, Debug, PartialEq)]
pub enum SimpleString {
    Owned(String),
    Borrowed(&'static str),
}

impl SimpleString {
    pub fn new<S: Into<SimpleString>>(s: S) -> Self {
        s.into()
    }

    pub fn as_str(&self) -> &str {
        match self {
            SimpleString::Owned(s) => s,
            SimpleString::Borrowed(s) => s,
        }
    }

    pub fn into_owned(self) -> String {
        match self {
            SimpleString::Owned(s) => s,
            SimpleString::Borrowed(s) => s.to_owned(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        SimpleString::Owned(s.to_owned())
    }

    pub fn from_string(s: String) -> Self {
        SimpleString::Owned(s)
    }
}

impl From<String> for SimpleString {
    fn from(s: String) -> Self {
        SimpleString::Owned(s)
    }
}

impl From<&'static str> for SimpleString {
    fn from(s: &'static str) -> Self {
        SimpleString::Borrowed(s)
    }
}

impl From<Cow<'static, str>> for SimpleString {
    fn from(s: Cow<'static, str>) -> Self {
        match s {
            Cow::Borrowed(b) => SimpleString::Borrowed(b),
            Cow::Owned(o) => SimpleString::Owned(o),
        }
    }
}

impl From<&String> for SimpleString {
    fn from(s: &String) -> Self {
        SimpleString::Owned(s.clone())
    }
}

impl Deref for SimpleString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl From<SimpleString> for String {
    fn from(s: SimpleString) -> Self {
        s.into_owned()
    }
}

impl std::fmt::Display for SimpleString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl SimpleString {
    pub fn wrapped_spans(&self, width: usize, style: Option<Style>) -> Vec<Vec<Span<'static>>> {
        let wrapper = TextWrapper::new(width);
        let text_line = TextLine {
            segments: vec![TextSegment {
                text: self.clone(),
                style,
            }],
            length: self.len(),
            background: None,
        };
        let wrapped_lines = wrapper.wrap_text_styled(&text_line, None);

        wrapped_lines
            .into_iter()
            .map(|line| 
                line.segments
                    .into_iter()
                    .map(|segment| 
                        Span::styled(segment.text.into_owned(), segment.style.unwrap_or_default())
                    )
                    .collect()
            )
            .collect()
    }
}