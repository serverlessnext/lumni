use lumni::api::error::ApplicationError;

use super::conversation::{ConversationId, Message, MessageId};
use super::prompt::Prompt;
use super::{PromptRole, PERSONAS};
pub use crate::external as lumni;

pub struct AssistantManager {
    prompt_template: Option<String>,
    initial_messages: Vec<Message>,
}

impl AssistantManager {
    pub fn new(
        assistant_name: Option<String>,
        user_instruction: Option<String>,
    ) -> Result<Self, ApplicationError> {
        let mut manager = AssistantManager {
            prompt_template: None,
            initial_messages: Vec::new(),
        };

        // TODO: default should only apply to servers that do no handle this internally
        // Use default assistant when both system_promt and assistant_name are None
        let assistant_name = assistant_name.or_else(|| {
            if user_instruction.is_some() {
                // assistant not needed
                None
            } else {
                // no user instruction, use default assistant
                Some("Default".to_string())
            }
        });

        if let Some(assistant_name) = assistant_name {
            manager.load_assistant(assistant_name, user_instruction)?;
        }

        Ok(manager)
    }

    fn load_assistant(
        &mut self,
        assistant: String,
        user_instruction: Option<String>,
    ) -> Result<(), ApplicationError> {
        let assistant_prompts: Vec<Prompt> = serde_yaml::from_str(PERSONAS)
            .map_err(|e| {
                ApplicationError::Unexpected(format!(
                    "Failed to parse persona data: {}",
                    e
                ))
            })?;

        let prompt = assistant_prompts
            .into_iter()
            .find(|p| p.name() == assistant)
            .ok_or_else(|| {
                ApplicationError::Unexpected(format!(
                    "Assistant '{}' not found in the dataset",
                    assistant
                ))
            })?;

        let system_prompt = build_system_prompt(&prompt, &user_instruction);

        // Add system message
        self.initial_messages.push(Message {
            id: MessageId(0), // system message is always the first message
            conversation_id: ConversationId(0), // temporary conversation id
            role: PromptRole::System,
            message_type: "text".to_string(),
            content: system_prompt,
            has_attachments: false,
            token_length: None,
            previous_message_id: None,
            created_at: 0,
            is_deleted: false,
        });

        // Add exchanges if any
        if let Some(exchanges) = prompt.exchanges() {
            for (index, exchange) in exchanges.iter().enumerate() {
                // User message
                self.initial_messages.push(Message {
                    id: MessageId((index * 2 + 1) as i64),
                    conversation_id: ConversationId(0), // temporary conversation id
                    role: PromptRole::User,
                    message_type: "text".to_string(),
                    content: exchange.question.clone(),
                    has_attachments: false,
                    token_length: None,
                    previous_message_id: Some(MessageId((index * 2) as i64)),
                    created_at: 0,
                    is_deleted: false,
                });

                // Assistant message
                self.initial_messages.push(Message {
                    id: MessageId((index * 2 + 2) as i64),
                    conversation_id: ConversationId(0), // temporary conversation id
                    role: PromptRole::Assistant,
                    message_type: "text".to_string(),
                    content: exchange.answer.clone(),
                    has_attachments: false,
                    token_length: None,
                    previous_message_id: Some(MessageId(
                        (index * 2 + 1) as i64,
                    )),
                    created_at: 0,
                    is_deleted: false,
                });
            }
        }

        if let Some(prompt_template) = prompt.prompt_template() {
            self.prompt_template = Some(prompt_template.to_string());
        }
        Ok(())
    }

    pub fn get_prompt_template(&self) -> Option<String> {
        self.prompt_template.clone()
    }

    pub fn get_initial_messages(&self) -> &[Message] {
        &self.initial_messages
    }
}

fn build_system_prompt(
    prompt: &Prompt,
    user_instruction: &Option<String>,
) -> String {
    match (prompt.system_prompt(), user_instruction) {
        (Some(assistant_instruction), Some(user_instr)) => {
            format!("{} {}", assistant_instruction.trim_end(), user_instr)
        }
        (Some(assistant_instruction), None) => {
            assistant_instruction.to_string()
        }
        (None, Some(user_instr)) => user_instr.to_string(),
        (None, None) => String::new(),
    }
}
