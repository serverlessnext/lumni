#[derive(Debug, Clone)]
pub enum FieldContentType {
    PlainText,
    Secret,
    Password,
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
