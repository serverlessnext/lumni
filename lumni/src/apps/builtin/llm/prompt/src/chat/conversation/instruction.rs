use lumni::api::error::ApplicationError;

use super::db::{
    ConversationCache, ConversationDbHandler, ConversationId, Message,
    MessageId, ModelSpec, Timestamp, Workspace,
};
use super::prepare::NewConversation;
use super::{
    ChatCompletionOptions, ChatMessage, ColorScheme, PromptError, PromptRole,
    TextLine,
};
pub use crate::external as lumni;

#[derive(Debug, Clone)]
pub struct PromptInstruction {
    cache: ConversationCache,
    model: Option<ModelSpec>,
    conversation_id: ConversationId,
    completion_options: ChatCompletionOptions,
}

impl PromptInstruction {
    pub async fn new(
        new_conversation: NewConversation,
        db_handler: &mut ConversationDbHandler,
    ) -> Result<Self, ApplicationError> {
        let completion_options = match new_conversation.options {
            Some(opts) => {
                let mut options = ChatCompletionOptions::default();
                options.update(opts)?;
                options
            }
            None => ChatCompletionOptions::default(),
        };

        let workspace: Option<Workspace> = None;

        let conversation_id = db_handler
            .new_conversation(
                "New Conversation",
                new_conversation.parent.as_ref().map(|p| p.id),
                workspace,
                new_conversation.parent.as_ref().map(|p| p.fork_message_id),
                Some(serde_json::to_value(&completion_options)?),
                new_conversation.model.as_ref(),
            )
            .await?;

        let mut prompt_instruction = PromptInstruction {
            cache: ConversationCache::new(),
            model: new_conversation.model,
            conversation_id,
            completion_options,
        };

        prompt_instruction
            .cache
            .set_conversation_id(prompt_instruction.conversation_id);

        if new_conversation.parent.is_some() {
            // if parent is provided, do not evaluate system_prompt and initial_messages
            // as they are already evaluated in the parent
        } else {
            // evaluate system_prompt and initial_messages only if parent is not provided
            if let Some(messages) = new_conversation.initial_messages {
                let mut messages_to_insert = Vec::new();

                let timestamp = Timestamp::from_system_time()?.as_millis();

                for (index, mut message) in messages.into_iter().enumerate() {
                    message.id = MessageId(index as i64);
                    message.conversation_id =
                        prompt_instruction.cache.get_conversation_id();
                    message.previous_message_id = if index > 0 {
                        Some(MessageId((index - 1) as i64))
                    } else {
                        None
                    };
                    message.token_length =
                        Some(simple_token_estimator(&message.content, None));
                    message.created_at = timestamp;

                    prompt_instruction.cache.add_message(message.clone());
                    messages_to_insert.push(message);
                }
                prompt_instruction
                    .cache
                    .set_preloaded_messages(messages_to_insert.len());

                // Insert messages into the database
                db_handler.put_new_messages(&messages_to_insert).await?;
            } else if let Some(system_prompt) = new_conversation.system_prompt {
                // add system_prompt as the first message
                prompt_instruction
                    .add_system_message(system_prompt, db_handler)
                    .await?;
            }
        }
        Ok(prompt_instruction)
    }

    pub async fn from_reader(
        reader: &ConversationDbHandler,
    ) -> Result<Self, ApplicationError> {
        // if conversation_id is none, it should err
        let conversation_id =
            reader.get_conversation_id().ok_or_else(|| {
                ApplicationError::DatabaseError(
                    "No conversation found in the reader".to_string(),
                )
            })?;
        let model_spec = reader
            .fetch_model_spec()
            .await
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;

        let completion_options = reader
            .fetch_completion_options()
            .await
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;

        let completion_options: ChatCompletionOptions =
            serde_json::from_value(completion_options)?;

        let preloaded_messages = completion_options
            .assistant_options
            .as_ref()
            .map_or(0, |options| options.preloaded_messages);

        let mut prompt_instruction = PromptInstruction {
            cache: ConversationCache::new(),
            model: Some(model_spec),
            conversation_id: conversation_id,
            completion_options,
        };

        prompt_instruction
            .cache
            .set_conversation_id(conversation_id);
        prompt_instruction
            .cache
            .set_preloaded_messages(preloaded_messages);

        // Load messages
        let messages = reader
            .fetch_messages()
            .await
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
        for message in messages {
            prompt_instruction.cache.add_message(message);
        }

        // Load attachments
        let attachments = reader
            .fetch_attachments()
            .await
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;
        for attachment in attachments {
            prompt_instruction.cache.add_attachment(attachment);
        }

        Ok(prompt_instruction)
    }

    pub fn get_model(&self) -> Option<&ModelSpec> {
        self.model.as_ref()
    }

    pub fn get_conversation_id(&self) -> ConversationId {
        // return the conversation_id from an active conversation
        // use the ConversationId from this struct, and not the cache as
        // the latter can be from a non-active conversation
        self.conversation_id
    }

    pub fn get_completion_options(&self) -> &ChatCompletionOptions {
        &self.completion_options
    }

    async fn add_system_message(
        &mut self,
        content: String,
        db_handler: &ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        let timestamp = Timestamp::from_system_time()?.as_millis();
        let message = Message {
            id: MessageId(0), // system message is first message in the conversation
            conversation_id: self.cache.get_conversation_id(),
            role: PromptRole::System,
            message_type: "text".to_string(),
            has_attachments: false,
            token_length: Some(simple_token_estimator(&content, None)),
            content,
            previous_message_id: None,
            created_at: timestamp,
            vote: 0,
            include_in_prompt: true,
            is_hidden: false,
            is_deleted: false,
        };
        // put system message directly into the database
        db_handler.put_new_message(&message).await?;
        self.cache.add_message(message);
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

    pub async fn put_last_response(
        &mut self,
        answer: &str,
        tokens_predicted: Option<usize>,
        db_handler: &ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        let (user_message, assistant_message) =
            self.finalize_last_messages(answer, tokens_predicted)?;

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
        db_handler
            .put_new_messages(&messages_to_insert)
            .await
            .map_err(|e| ApplicationError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn finalize_last_messages(
        &mut self,
        answer: &str,
        tokens_predicted: Option<usize>,
    ) -> Result<(Option<Message>, Option<Message>), ApplicationError> {
        // Get the last message, which should be an unfinished assistant message
        let last_message = match self.cache.get_last_message() {
            Some(msg) if msg.role == PromptRole::Assistant => msg.clone(),
            _ => {
                return Ok((None, None));
            }
        };
        let timestamp = Timestamp::from_system_time()?.as_millis();

        let assistant_message = Message {
            id: last_message.id,
            conversation_id: last_message.conversation_id,
            role: last_message.role,
            message_type: last_message.message_type,
            has_attachments: last_message.has_attachments,
            previous_message_id: last_message.previous_message_id,
            created_at: timestamp,
            vote: last_message.vote,
            include_in_prompt: last_message.include_in_prompt,
            is_hidden: last_message.is_hidden,
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

        Ok((user_message, Some(assistant_message)))
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
            created_at: 0, // updated when finalizing the message
            vote: 0,
            include_in_prompt: true,
            is_hidden: false,
            is_deleted: false,
        };
        self.cache.add_message(message);
    }

    pub fn new_question(
        &mut self,
        question: &str,
        max_token_length: usize,
    ) -> Result<Vec<ChatMessage>, PromptError> {
        let timestamp = Timestamp::from_system_time()
            .map_err(|e| PromptError::Runtime(e.to_string()))?
            .as_millis();

        let message = Message {
            id: self.cache.new_message_id(),
            conversation_id: self.cache.get_conversation_id(),
            role: PromptRole::User,
            message_type: "text".to_string(),
            content: question.to_string(),
            has_attachments: false,
            token_length: None, // token length is computed after completion
            previous_message_id: self.cache.get_last_message_id(),
            created_at: timestamp,
            vote: 0,
            include_in_prompt: true,
            is_hidden: false,
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
        Ok(messages)
    }

    pub fn export_conversation(
        &self,
        color_scheme: &ColorScheme,
    ) -> Vec<TextLine> {
        self.cache.export_conversation(color_scheme)
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
