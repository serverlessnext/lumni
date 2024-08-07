use lumni::api::error::ApplicationError;

use super::db::{ConversationDbHandler, Message, ModelServerName, ModelSpec};
use super::{ParentConversation, PromptRole};
pub use crate::external as lumni;

#[derive(Debug, Clone, PartialEq)]
pub struct NewConversation {
    pub server: ModelServerName,
    pub model: Option<ModelSpec>,
    pub options: Option<serde_json::Value>,
    pub system_prompt: Option<String>, // system_prompt ignored if parent is provided
    pub initial_messages: Option<Vec<Message>>, // initial_messages ignored if parent is provided
    pub parent: Option<ParentConversation>,     // forked conversation
}

impl NewConversation {
    pub async fn new(
        new_server: ModelServerName,
        new_model: ModelSpec,
        conversation_handler: &ConversationDbHandler,
    ) -> Result<NewConversation, ApplicationError> {
        match conversation_handler.get_conversation_id() {
            Some(current_conversation_id) => {
                // Fork from an existing conversation
                let mut current_completion_options =
                    conversation_handler.fetch_completion_options().await?;
                current_completion_options["model_server"] =
                    serde_json::to_value(new_server.clone())?;

                let parent = conversation_handler
                    .fetch_last_message_id()
                    .await?
                    .map(|last_message_id| ParentConversation {
                        id: current_conversation_id,
                        fork_message_id: last_message_id,
                    });

                create_new_conversation(
                    new_server,
                    new_model,
                    Some(current_completion_options),
                    parent,
                )
            }
            None => {
                // Start a new conversation
                let completion_options = serde_json::json!({
                    "model_server": new_server,
                });
                create_new_conversation(
                    new_server,
                    new_model,
                    Some(completion_options),
                    None,
                )
            }
        }
    }

    pub async fn is_equal(
        &self,
        handler: &ConversationDbHandler,
    ) -> Result<bool, ApplicationError> {
        // check if conversation settings are equal to the conversation stored in the database

        // Compare model
        let last_model = handler.fetch_model_spec().await?;
        if self.model.as_ref() != Some(&last_model) {
            return Ok(false);
        }

        // Compare completion options (which includes server name and assistant)
        let last_options = handler.fetch_completion_options().await?;
        let new_options = match &self.options {
            Some(opts) => opts.clone(),
            None => serde_json::json!({}),
        };
        if last_options != new_options {
            return Ok(false);
        }
        // Compare system prompt. If the system prompt is not set in the new conversation, we check by first system prompt in the initial messages
        let last_system_prompt = handler.fetch_system_prompt().await?;
        let new_system_prompt = match &self.system_prompt {
            Some(prompt) => Some(prompt.as_str()),
            None => self.initial_messages.as_ref().and_then(|messages| {
                messages.first().and_then(|msg| {
                    if msg.role == PromptRole::System {
                        Some(msg.content.as_str())
                    } else {
                        None
                    }
                })
            }),
        };

        if last_system_prompt.as_deref() != new_system_prompt {
            return Ok(false);
        }
        // Conversation settings are equal
        Ok(true)
    }
}

fn create_new_conversation(
    server: ModelServerName,
    model: ModelSpec,
    options: Option<serde_json::Value>,
    parent: Option<ParentConversation>,
) -> Result<NewConversation, ApplicationError> {
    Ok(NewConversation {
        server,
        model: Some(model),
        options,
        system_prompt: None,
        initial_messages: None,
        parent,
    })
}
