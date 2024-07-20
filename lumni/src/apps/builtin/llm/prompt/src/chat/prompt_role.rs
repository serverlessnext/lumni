use std::fmt::Display;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromptRole {
    User,
    Assistant,
    System,
}

impl PromptRole {
    pub fn to_string(&self) -> String {
        match self {
            PromptRole::User => "user",
            PromptRole::Assistant => "assistant",
            PromptRole::System => "system",
        }
        .to_string()
    }
}

impl Display for PromptRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromSql for PromptRole {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "user" => Ok(PromptRole::User),
            "assistant" => Ok(PromptRole::Assistant),
            "system" => Ok(PromptRole::System),
            _ => Err(FromSqlError::InvalidType.into()),
        }
    }
}