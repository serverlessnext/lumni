#[derive(Debug, Clone)]
pub enum FieldType {
    Text,
    TextArea,
    Secret,
    Password,
}

impl FieldType {
    pub fn is_secret(&self) -> bool {
        matches!(self, Self::Secret)
    }

    pub fn is_password(&self) -> bool {
        matches!(self, Self::Password)
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }

    pub fn is_text_area(&self) -> bool {
        matches!(self, Self::TextArea)
    }
}

impl Default for FieldType {
    fn default() -> Self {
        Self::Text
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
