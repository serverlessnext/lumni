#[derive(Debug, Clone)]
pub enum FieldType {
    Text,
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
}

impl Default for FieldType {
    fn default() -> Self {
        Self::Text
    }
}
