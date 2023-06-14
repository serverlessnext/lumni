#[derive(Debug, Clone)]
pub enum FieldType {
    Text { is_enabled: bool },
    Secret { is_enabled: bool },
    Password { is_enabled: bool },
}

impl FieldType {
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Text { is_enabled } => *is_enabled,
            Self::Secret { is_enabled } => *is_enabled,
            Self::Password { is_enabled } => *is_enabled,
        }
    }

    pub fn is_secret(&self) -> bool {
        matches!(self, Self::Secret { .. })
    }

    pub fn is_password(&self) -> bool {
        matches!(self, Self::Password { .. })
    }
}

impl Default for FieldType {
    fn default() -> Self {
        Self::Text { is_enabled: true }
    }
}
