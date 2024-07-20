use lumni::api::error::ApplicationError;

use super::db::{
    ConversationCache, ConversationDatabaseStore, ConversationId,
    Message, MessageId, Model, ModelIdentifier, ModelServerName,
};
use super::prompt::Prompt;
use super::{ChatCompletionOptions, ChatMessage, PromptRole, PERSONAS};
pub use crate::external as lumni;

pub struct PromptInstruction {
    cache: ConversationCache,
    prompt_template: Option<String>,
}

impl PromptInstruction {
    pub fn new(
        instruction: Option<String>,
        assistant: Option<String>,
        options: Option<&String>,
        db_conn: &ConversationDatabaseStore,
    ) -> Result<Self, ApplicationError> {
        // If both instruction and assistant are None, use the default assistant
        let assistant = if instruction.is_none() && assistant.is_none() {
            Some("Default".to_string())
        } else {
            assistant
        };
        let completion_options = match options {
            Some(opts) => {
                let mut options = ChatCompletionOptions::default();
                options.update_from_json(opts)?;
                serde_json::to_value(options)?
            }
            None => serde_json::to_value(ChatCompletionOptions::default())?,
        };
        // Create a new Conversation in the database
        let model = Model::new(
            ModelIdentifier::new("foo-provider", "bar-model"),
        );

        let conversation_id = {
            db_conn.new_conversation(
                "New Conversation",
                None, // parent_id, None for new conversation
                None, // fork_message_id, None for new conversation
                Some(completion_options),   // completion_options
                model,
                ModelServerName("ollama".to_string()),
            )?
        };

        let mut prompt_instruction = PromptInstruction {
            cache: ConversationCache::new(),
            prompt_template: None,
        };

        prompt_instruction
            .cache
            .set_conversation_id(conversation_id);

        if let Some(assistant) = assistant {
            prompt_instruction.preload_from_assistant(
                assistant,
                instruction, // add user-instruction with assistant
                db_conn,
            )?;
        } else if let Some(instruction) = instruction {
            prompt_instruction.add_system_message(instruction, db_conn)?;
        }

        Ok(prompt_instruction)
    }

    pub fn get_conversation_id(&self) -> ConversationId {
        self.cache.get_conversation_id()
    }

    fn add_system_message(
        &mut self,
        content: String,
        db_conn: &ConversationDatabaseStore,
    ) -> Result<(), ApplicationError> {
        let message = Message {
            id: MessageId(0), // system message is first message in the conversation
            conversation_id: self.cache.get_conversation_id(),
            role: PromptRole::System,
            message_type: "text".to_string(),
            has_attachments: false,
            token_length: Some(simple_token_estimator(&content, None)),
            content,
            previous_message_id: None,
            created_at: 0,
            is_deleted: false,
        };
        // put system message directly into the database
        db_conn.put_new_message(&message)?;
        self.cache.add_message(message);
        Ok(())
    }

    pub fn import_conversation(
        &mut self,
        id: &str,
        db_conn: &ConversationDatabaseStore,
    ) -> Result<(), ApplicationError> {
        let conversation_id = ConversationId(id.parse().map_err(|_| {
            ApplicationError::NotFound(format!(
                "Conversation {id} not found in database"
            ))
        })?);

        let (conversation, messages) = db_conn
            .fetch_conversation(Some(conversation_id), None)?
            .ok_or_else(|| {
                ApplicationError::NotFound("Conversation not found".to_string())
            })?;

        // Clear the existing ConversationCache
        self.cache = ConversationCache::new();

        // Set the conversation ID
        self.cache.set_conversation_id(conversation.id);

        // Add messages to the cache
        for message in messages {
            // Fetch and add attachments for each message
            let attachments = db_conn.fetch_message_attachments(message.id)?;
            for attachment in attachments {
                self.cache.add_attachment(attachment);
            }
            self.cache.add_message(message);
        }

        Ok(())
    }

    pub fn reset_history(
        &mut self,
        db_conn: &ConversationDatabaseStore,
    ) -> Result<(), ApplicationError> {
        // reset by creating a new conversation
        // TODO: clone previous conversation settings
        let model = Model::new(
            ModelIdentifier::new("foo-provider", "bar-model"),
        );

        let current_conversation_id =
            db_conn.new_conversation(
                "New Conversation",
                None,
                None,
                None,
                model,
                ModelServerName("ollama".to_string()),
            )?;
        self.cache.set_conversation_id(current_conversation_id);
        Ok(())
    }

    pub fn append_last_response(&mut self, answer: &str) {
        if let Some(last_message) = self.cache.get_last_message() {
            if last_message.role == PromptRole::Assistant {
                let new_content = format!("{}{}", last_message.content, answer);
                self.cache.update_message_by_id(
                    last_message.id,
                    &new_content,
                    None,
                );
            } else {
                self.add_assistant_message(answer);
            }
        } else {
            unreachable!("Cannot append response to an empty conversation");
        }
    }

    pub fn get_last_response(&self) -> Option<String> {
        self.cache
            .get_last_message()
            .filter(|msg| msg.role == PromptRole::Assistant)
            .map(|msg| msg.content.clone())
    }

    pub fn put_last_response(
        &mut self,
        answer: &str,
        tokens_predicted: Option<usize>,
        db_conn: &ConversationDatabaseStore,
    ) -> Result<(), ApplicationError> {
        let (user_message, assistant_message) =
            self.prepare_last_messages(answer, tokens_predicted);

        // Prepare messages for database insertion
        let mut messages_to_insert = Vec::new();
        if let Some(user_msg) = user_message {
            messages_to_insert.push(user_msg);
        }
        if let Some(assistant_msg) = assistant_message {
            messages_to_insert.push(assistant_msg);
        } else {
            return Ok(()); // No messages to update
        }

        // Insert messages into the database
        db_conn
            .put_new_messages(&messages_to_insert)
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn prepare_last_messages(
        &mut self,
        answer: &str,
        tokens_predicted: Option<usize>,
    ) -> (Option<Message>, Option<Message>) {
        // Get the last message, which should be an unfinished assistant message
        let last_message = match self.cache.get_last_message() {
            Some(msg) if msg.role == PromptRole::Assistant => msg.clone(),
            _ => return (None, None),
        };

        let assistant_message = Message {
            id: last_message.id,
            conversation_id: last_message.conversation_id,
            role: last_message.role,
            message_type: last_message.message_type,
            has_attachments: last_message.has_attachments,
            previous_message_id: last_message.previous_message_id,
            created_at: last_message.created_at,
            is_deleted: last_message.is_deleted,
            content: answer.to_string(),
            token_length: tokens_predicted.map(|t| t as i64),
        };

        // Update the cache with the finalized assistant message
        self.cache.update_message_by_id(
            assistant_message.id,
            &assistant_message.content,
            assistant_message.token_length,
        );

        // Get and prepare the user message
        let user_message = last_message
            .previous_message_id
            .and_then(|id| self.cache.get_message_by_id(id).cloned())
            .filter(|msg| msg.role == PromptRole::User)
            .map(|mut msg| {
                let user_token_length = tokens_predicted.map(|tokens| {
                    let chars_per_token =
                        calculate_chars_per_token(answer, tokens);
                    simple_token_estimator(&msg.content, Some(chars_per_token))
                });

                // Update the cache with the user message's new token length
                if let Some(user_token_length) = user_token_length {
                    msg.token_length = Some(user_token_length);
                    self.cache.update_message_token_length(
                        &msg.id,
                        user_token_length,
                    );
                }
                msg
            });

        (user_message, Some(assistant_message))
    }

    fn add_assistant_message(&mut self, content: &str) {
        let message = Message {
            id: self.cache.new_message_id(),
            conversation_id: self.cache.get_conversation_id(),
            role: PromptRole::Assistant,
            message_type: "text".to_string(),
            content: content.to_string(),
            has_attachments: false,
            token_length: None,
            previous_message_id: self.cache.get_last_message_id(),
            created_at: 0,
            is_deleted: false,
        };
        self.cache.add_message(message);
    }

    pub fn new_question(
        &mut self,
        question: &str,
        max_token_length: usize,
    ) -> Vec<ChatMessage> {
        let message = Message {
            id: self.cache.new_message_id(),
            conversation_id: self.cache.get_conversation_id(),
            role: PromptRole::User,
            message_type: "text".to_string(),
            content: question.to_string(),
            has_attachments: false,
            token_length: None, // token length is computed after completion
            previous_message_id: self.cache.get_last_message_id(),
            created_at: 0,
            is_deleted: false,
        };
        self.cache.add_message(message);

        // Collect messages while respecting token limits
        let mut messages: Vec<ChatMessage> = Vec::new();
        let mut total_tokens = 0;

        let mut system_message: Option<ChatMessage> = None;

        // Add messages from most recent to oldest, respecting token limit
        for msg in self.cache.get_conversation_messages().into_iter().rev() {
            let msg_token_length =
                msg.token_length.map(|len| len as usize).unwrap_or(0);

            if msg.role == PromptRole::System {
                // store system_prompt for later insertion at the beginning
                system_message = Some(ChatMessage {
                    role: msg.role,
                    content: msg.content.clone(),
                });
                // system prompt is always included
                total_tokens += msg_token_length;
                continue;
            }
            if total_tokens + msg_token_length <= max_token_length {
                total_tokens += msg_token_length;
                messages.push(ChatMessage {
                    role: msg.role,
                    content: msg.content.clone(),
                });
            } else {
                // reached token limit
                break;
            }
        }

        // ensure the system prompt is always included
        if let Some(system_message) = system_message {
            messages.push(system_message);
        }
        // Reverse the messages to maintain chronological order
        messages.reverse();
        messages
    }

    pub fn get_prompt_template(&self) -> Option<&str> {
        self.prompt_template.as_deref()
    }

    pub fn preload_from_assistant(
        &mut self,
        assistant: String,
        user_instruction: Option<String>,
        db_conn: &ConversationDatabaseStore,
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

        // Add system prompt
        self.add_system_message(system_prompt, db_conn)?;

        if let Some(exchanges) = prompt.exchanges() {
            let mut messages = Vec::new();
            let conversation_id = self.cache.get_conversation_id();
            let current_timestamp = 0;

            for loaded_exchange in exchanges.iter() {
                let previous_message_id = self.cache.get_last_message_id();

                let user_message = Message {
                    id: self.cache.new_message_id(),
                    conversation_id,
                    role: PromptRole::User,
                    message_type: "text".to_string(),
                    content: loaded_exchange.question.clone(),
                    has_attachments: false,
                    token_length: Some(simple_token_estimator(
                        &loaded_exchange.question,
                        None,
                    )),
                    previous_message_id,
                    created_at: current_timestamp,
                    is_deleted: false,
                };
                messages.push(user_message.clone());
                let user_message_id = user_message.id;
                self.cache.add_message(user_message);

                // Assistant message
                let assistant_message = Message {
                    id: self.cache.new_message_id(),
                    conversation_id,
                    role: PromptRole::Assistant,
                    message_type: "text".to_string(),
                    content: loaded_exchange.answer.clone(),
                    has_attachments: false,
                    token_length: Some(simple_token_estimator(
                        &loaded_exchange.answer,
                        None,
                    )),
                    previous_message_id: Some(user_message_id),
                    created_at: current_timestamp,
                    is_deleted: false,
                };
                messages.push(assistant_message.clone());
                self.cache.add_message(assistant_message);
            }

            // Batch insert messages
            db_conn.put_new_messages(&messages)?;
        }

        if let Some(prompt_template) = prompt.prompt_template() {
            self.prompt_template = Some(prompt_template.to_string());
        }
        Ok(())
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

fn simple_token_estimator(input: &str, chars_per_token: Option<f32>) -> i64 {
    // Simple but fast token estimator based on character count
    let chars_per_token = chars_per_token.unwrap_or(4.0);
    let chars_count = input.chars().count() as f32;
    (chars_count / chars_per_token).ceil() as i64
}

fn calculate_chars_per_token(answer: &str, tokens_predicted: usize) -> f32 {
    let char_count = answer.chars().count() as f32;
    char_count / tokens_predicted as f32
}
