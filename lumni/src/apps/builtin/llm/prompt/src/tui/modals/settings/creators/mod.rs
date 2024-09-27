mod profile;
mod prompt;
mod provider;

use std::fmt;

pub use profile::{ProfileCreationStep, ProfileCreator};
pub use prompt::{PromptCreationStep, PromptCreator};
pub use provider::{ProviderCreationStep, ProviderCreator};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProfileSection {
    Provider,
    Prompt,
}

impl ProfileSection {
    fn as_str(&self) -> &'static str {
        match self {
            ProfileSection::Provider => "provider",
            ProfileSection::Prompt => "prompt",
        }
    }

    fn from_config_tab(tab: &ConfigTab) -> Option<Self> {
        match tab {
            ConfigTab::Providers => Some(ProfileSection::Provider),
            ConfigTab::Prompts => Some(ProfileSection::Prompt),
            _ => None,
        }
    }
}

impl fmt::Display for ProfileSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
