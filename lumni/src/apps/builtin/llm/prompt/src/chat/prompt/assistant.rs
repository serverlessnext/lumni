use lumni::api::error::ApplicationError;

pub use super::{
    AssistantOptions, ChatCompletionOptions, ConversationId, Message,
    MessageId, Prompt, PromptRole, PERSONAS,
};
//use super::db::{ConversationId, Message, MessageId};
//use super::completion_options::{AssistantOptions, ChatCompletionOptions};
//use super::prompt::Prompt;
//use super::{PromptRole, PERSONAS};
pub use crate::external as lumni;

pub struct AssistantManager {
    initial_messages: Vec<Message>,
    completion_options: ChatCompletionOptions,
}

impl AssistantManager {
    pub fn new(
        assistant_name: Option<String>,
        user_instruction: Option<String>,
    ) -> Result<Self, ApplicationError> {
        let mut manager = AssistantManager {
            initial_messages: Vec::new(),
            completion_options: ChatCompletionOptions::default(),
        };

        // Use default assistant when both system_promt and assistant_name are None
        let assistant_name = assistant_name.or_else(|| {
            user_instruction
                .as_ref()
                .map(|instruction| {
                    manager.add_system_message(instruction.to_string());
                    None // No assistant required
                })
                .unwrap_or_else(|| {
                    // TODO: default should only apply to servers that do no handle this internally
                    Some("Default".to_string()) // Use default assistant
                })
        });

        if let Some(assistant_name) = assistant_name {
            manager.load_assistant(assistant_name, user_instruction)?;
        }

        Ok(manager)
    }

    fn add_system_message(&mut self, system_prompt: String) {
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
            vote: 0,
            include_in_prompt: true,
            is_hidden: false,
            is_deleted: false,
        });
    }

    fn load_assistant(
        &mut self,
        assistant_name: String,
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
            .find(|p| p.name() == assistant_name)
            .ok_or_else(|| {
                ApplicationError::Unexpected(format!(
                    "Assistant '{}' not found in the dataset",
                    assistant_name
                ))
            })?;

        let system_prompt = build_system_prompt(&prompt, &user_instruction);

        // Add system message
        self.add_system_message(system_prompt);

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
                    vote: 0,
                    include_in_prompt: true,
                    is_hidden: false,
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
                    vote: 0,
                    include_in_prompt: true,
                    is_hidden: false,
                    is_deleted: false,
                });
            }
        }

        let assistant_options = AssistantOptions {
            name: assistant_name,
            preloaded_messages: self.initial_messages.len() - 1, // exclude the first system message
            prompt_template: prompt.prompt_template().map(|s| s.to_string()),
        };
        self.completion_options
            .set_assistant_options(assistant_options);

        Ok(())
    }

    pub fn get_completion_options(&self) -> &ChatCompletionOptions {
        &self.completion_options
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
