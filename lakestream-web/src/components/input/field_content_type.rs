#[derive(Debug, Clone)]
pub enum FieldContentType {
    PlainText,
    Secret,   // can be hidden and un-hidden
    Password, // can not be unhidden
    TextArea, // TextArea is PlainText but with larger buffer/ field
}

impl FieldContentType {
    pub fn is_secret(&self) -> bool {
        matches!(self, Self::Secret)
    }

    pub fn is_password(&self) -> bool {
        matches!(self, Self::Password)
    }

    pub fn is_plain_text(&self) -> bool {
        matches!(self, Self::PlainText)
    }

    pub fn is_text_area(&self) -> bool {
        matches!(self, Self::TextArea)
    }
}

impl Default for FieldContentType {
    fn default() -> Self {
        Self::PlainText
    }
}

#[derive(Debug, Clone)]
pub enum DocumentType {
    Text,
    Html,
}

impl DocumentType {
    pub fn is_html(&self) -> bool {
        matches!(self, Self::Html)
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }
}

impl Default for DocumentType {
    fn default() -> Self {
        Self::Html
    }
}
