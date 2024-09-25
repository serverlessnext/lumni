use std::str::FromStr;
use std::sync::Arc;

use clap::ArgMatches;
use lumni::api::error::ApplicationError;

use super::{
    AssistantManager, ConversationDatabase, NewConversation, PromptInstruction,
    UserProfile, UserProfileDbHandler,
};
use crate::external as lumni;

pub struct PromptInstructionBuilder {
    db_conn: Arc<ConversationDatabase>,
    instruction: Option<String>,
    assistant: Option<String>,
    user_options: Option<String>,
    profile: Option<UserProfile>,
}

impl PromptInstructionBuilder {
    pub fn new(db_conn: Arc<ConversationDatabase>) -> Self {
        Self {
            db_conn,
            instruction: None,
            assistant: None,
            user_options: None,
            profile: None,
        }
    }

    pub async fn with_matches(
        mut self,
        matches: &ArgMatches,
    ) -> Result<Self, ApplicationError> {
        self.instruction = matches.get_one::<String>("system").cloned();
        self.assistant = matches.get_one::<String>("assistant").cloned();
        self.user_options = matches.get_one::<String>("options").cloned();

        if let Some(profile_selector) = matches.get_one::<String>("profile") {
            let mut profile_handler = self.db_conn.get_profile_handler(None);
            let (name, id) = parse_profile_selector(profile_selector);

            self.profile = match (name, id) {
                (Some(name), Some(id)) => {
                    select_profile_by_name_and_id(
                        &mut profile_handler,
                        name,
                        id,
                    )
                    .await?
                }
                (Some(name), None) => {
                    select_profile_by_name(&mut profile_handler, name).await?
                }
                (None, Some(id)) => {
                    select_profile_by_id(&mut profile_handler, id).await?
                }
                _ => {
                    return Err(ApplicationError::InvalidInput(
                        "Invalid profile selector format".to_string(),
                    ));
                }
            };
        } else {
            // If no profile selector is provided, use the default profile
            self = self.from_default().await?;
        }
        Ok(self)
    }

    async fn from_default(mut self) -> Result<Self, ApplicationError> {
        let profile_handler = self.db_conn.get_profile_handler(None);
        if let Some(default_profile) =
            profile_handler.get_default_profile().await?
        {
            self.profile = Some(UserProfile {
                id: default_profile.id,
                name: default_profile.name,
            });
            Ok(self)
        } else {
            Err(ApplicationError::InvalidInput(
                "No default profile available".to_string(),
            ))
        }
    }

    pub fn with_profile(mut self, profile: UserProfile) -> Self {
        self.profile = Some(profile);
        self
    }

    pub async fn build(self) -> Result<PromptInstruction, ApplicationError> {
        if self.profile.is_none() {
            return Err(ApplicationError::InvalidInput(
                "Profile required to build prompt instruction".to_string(),
            ));
        }

        let mut profile_handler =
            self.db_conn.get_profile_handler(self.profile);

        let model_backend =
            profile_handler.model_backend().await?.ok_or_else(|| {
                ApplicationError::InvalidInput(
                    "Failed to get model backend".to_string(),
                )
            })?;
        // TODO: assistant, user_options and instructions should be loaded from
        // the profile if not yet defined. Note that the current profile does not yet support
        // configuring these. Adding support within profile should be done first.

        let assistant_manager =
            AssistantManager::new(self.assistant, self.instruction.clone())?;
        let initial_messages =
            assistant_manager.get_initial_messages().to_vec();
        let mut completion_options =
            assistant_manager.get_completion_options().clone();

        let model_server_name = model_backend.server_name();
        completion_options.model_server = Some(model_server_name.clone());

        if let Some(s) = self.user_options {
            let user_options_value =
                serde_json::from_str::<serde_json::Value>(&s)?;
            completion_options.update(user_options_value)?;
        }

        let new_conversation = NewConversation {
            server: model_server_name,
            model: model_backend.model.clone(),
            options: Some(serde_json::to_value(completion_options)?),
            system_prompt: self.instruction,
            initial_messages: Some(initial_messages),
            parent: None,
        };
        let mut db_handler = self.db_conn.get_conversation_handler(None);
        let conversation_id = self.db_conn.fetch_last_conversation_id().await?;

        let prompt_instruction = if let Some(conversation_id) = conversation_id
        {
            db_handler.set_conversation_id(conversation_id);
            match new_conversation.is_equal(&db_handler).await {
                Ok(true) => {
                    log::debug!("Continuing last conversation");
                    Some(PromptInstruction::from_reader(&db_handler).await?)
                }
                Ok(_) => None,
                Err(e) => {
                    log::warn!(
                        "Failed to check if conversation is equal: {}",
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        let prompt_instruction = match prompt_instruction {
            Some(instruction) => instruction,
            None => {
                log::debug!("Starting new conversation");
                PromptInstruction::new(new_conversation, &mut db_handler)
                    .await?
            }
        };
        Ok(prompt_instruction)
    }
}

fn parse_profile_selector(selector: &str) -> (Option<&str>, Option<i64>) {
    if selector.starts_with("::") {
        // Case: ::id
        let id_str = selector.trim_start_matches("::");
        return (None, i64::from_str(id_str).ok());
    }

    let parts: Vec<&str> = selector.split("::").collect();
    match parts.as_slice() {
        [name, id] => (Some(name.trim()), i64::from_str(id.trim()).ok()),
        [name] => (Some(name.trim()), None),
        _ => (None, None),
    }
}

async fn select_profile_by_name_and_id(
    profile_handler: &mut UserProfileDbHandler,
    name: &str,
    id: i64,
) -> Result<Option<UserProfile>, ApplicationError> {
    if let Some(profile) = profile_handler.get_profile_by_id(id).await? {
        if profile.name == name {
            Ok(Some(UserProfile {
                id: profile.id,
                name: profile.name,
            }))
        } else {
            Err(ApplicationError::InvalidInput(format!(
                "Profile with id {} does not match the name '{}'",
                id, name
            )))
        }
    } else {
        Err(ApplicationError::InvalidInput(format!(
            "No profile found with id {}",
            id
        )))
    }
}

async fn select_profile_by_name(
    profile_handler: &mut UserProfileDbHandler,
    name: &str,
) -> Result<Option<UserProfile>, ApplicationError> {
    let profiles = profile_handler.get_profiles_by_name(name).await?;
    match profiles.len() {
        0 => Err(ApplicationError::InvalidInput(format!(
            "No profile found with name '{}'",
            name
        ))),
        1 => Ok(Some(UserProfile {
            id: profiles[0].id,
            name: profiles[0].name.clone(),
        })),
        _ => {
            println!(
                "Multiple profiles found with the name '{}'. Please specify \
                 the id:",
                name
            );
            for profile in profiles {
                println!("  ID: {}, Name: {}", profile.id, profile.name);
            }
            Err(ApplicationError::InvalidInput(
                "Multiple profiles found. Please specify the id.".to_string(),
            ))
        }
    }
}

async fn select_profile_by_id(
    profile_handler: &mut UserProfileDbHandler,
    id: i64,
) -> Result<Option<UserProfile>, ApplicationError> {
    if let Some(profile) = profile_handler.get_profile_by_id(id).await? {
        Ok(Some(UserProfile {
            id: profile.id,
            name: profile.name,
        }))
    } else {
        Err(ApplicationError::InvalidInput(format!(
            "No profile found with id {}",
            id
        )))
    }
}
