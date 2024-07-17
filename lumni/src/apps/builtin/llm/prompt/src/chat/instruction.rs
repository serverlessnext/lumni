use std::collections::HashMap;

use lumni::api::error::ApplicationError;

use super::db::{
    self, ConversationCache, ConversationDatabaseStore, ConversationId,
    Exchange, ExchangeId, Message,
};
use super::prompt::Prompt;
use super::{
    ChatCompletionOptions, ChatMessage, LLMDefinition, PromptOptions,
    PromptRole, DEFAULT_N_PREDICT, DEFAULT_TEMPERATURE, PERSONAS,
};
pub use crate::external as lumni;

pub struct PromptInstruction {
    cache: ConversationCache,
    completion_options: ChatCompletionOptions,
    prompt_options: PromptOptions,
    prompt_template: Option<String>,
}

impl Default for PromptInstruction {
    fn default() -> Self {
        let completion_options = ChatCompletionOptions::default()
            .set_temperature(DEFAULT_TEMPERATURE)
            .set_n_predict(DEFAULT_N_PREDICT)
            .set_cache_prompt(true)
            .set_stream(true);

        PromptInstruction {
            cache: ConversationCache::new(),
            completion_options,
            prompt_options: PromptOptions::default(),
            prompt_template: None,
        }
    }
}

impl PromptInstruction {
    pub fn new(
        instruction: Option<String>,
        assistant: Option<String>,
        options: Option<&String>,
        db_conn: &ConversationDatabaseStore,
    ) -> Result<Self, ApplicationError> {
        let mut prompt_instruction = PromptInstruction::default();
        if let Some(json_str) = options {
            prompt_instruction.prompt_options.update_from_json(json_str);
            prompt_instruction
                .completion_options
                .update_from_json(json_str);
        }

        // If both instruction and assistant are None, use the default assistant
        let assistant = if instruction.is_none() && assistant.is_none() {
            // for useful responses, there should either be a system prompt or an
            // assistant set. If none are given use the default assistant.
            Some("Default".to_string())
        } else {
            assistant
        };

        // Create a new Conversation in the database
        let conversation_id = {
            db_conn.new_conversation(
                "New Conversation",
                None,
                serde_json::to_value(&prompt_instruction.completion_options)
                    .ok(),
                serde_json::to_value(&prompt_instruction.prompt_options).ok(),
            )?
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
            let exchange = prompt_instruction.first_exchange(Some(instruction));
            let _result =
                db_conn.finalize_exchange(&exchange, &prompt_instruction.cache);
        };

        Ok(prompt_instruction)
    }

    pub fn import_conversation(
        &mut self,
        id: &str,
        db_conn: &ConversationDatabaseStore,
    ) -> Result<(), ApplicationError> {
        // Fetch the conversation and its messages from the database
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

        // Group messages by exchange
        let mut exchanges: HashMap<ExchangeId, Vec<Message>> = HashMap::new();
        for message in messages {
            exchanges
                .entry(message.exchange_id)
                .or_default()
                .push(message);
        }

        // Add exchanges and messages to the cache
        for (exchange_id, exchange_messages) in exchanges {
            let exchange = Exchange {
                id: exchange_id,
                conversation_id: conversation.id,
                created_at: exchange_messages
                    .first()
                    .map(|m| m.created_at)
                    .unwrap_or(0),
                previous_exchange_id: None, // You might want to store and retrieve this
                is_deleted: false,
            };

            self.cache.add_exchange(exchange);

            for message in exchange_messages {
                self.cache.add_message(message);
            }
        }
        Ok(())
    }

    pub fn reset_history(
        &mut self,
        db_conn: &ConversationDatabaseStore,
    ) -> Result<(), ApplicationError> {
        // reset by creating a new conversation
        // TODO: clone previous conversation settings
        let current_conversation_id =
            db_conn.new_conversation("New Conversation", None, None, None)?;
        self.cache.set_conversation_id(current_conversation_id);
        Ok(())
    }

    pub fn append_last_response(&mut self, answer: &str) {
        ExchangeHandler::append_response(&mut self.cache, answer);
    }

    pub fn get_last_response(&mut self) -> Option<String> {
        ExchangeHandler::get_last_response(&mut self.cache)
    }

    pub fn put_last_response(
        &mut self,
        answer: &str,
        tokens_predicted: Option<usize>,
        db_conn: &ConversationDatabaseStore,
    ) {
        let exchange = ExchangeHandler::put_last_response(
            &mut self.cache,
            answer,
            tokens_predicted,
        );
        if let Some(exchange) = exchange {
            let _result = db_conn.finalize_exchange(&exchange, &self.cache);
        }
    }

    fn first_exchange(&mut self, system_prompt: Option<String>) -> Exchange {
        let exchange = Exchange {
            id: ExchangeId(0),
            conversation_id: self.cache.get_conversation_id(),
            created_at: 0,
            previous_exchange_id: None,
            is_deleted: false,
        };

        let system_message = Message {
            id: self.cache.new_message_id(),
            conversation_id: self.cache.get_conversation_id(),
            exchange_id: exchange.id,
            role: PromptRole::System,
            message_type: "text".to_string(),
            has_attachments: false,
            token_length: Some(simple_token_estimator(
                &system_prompt.as_deref().unwrap_or(""),
                None,
            )),
            content: system_prompt.unwrap_or_else(|| "".to_string()),
            created_at: 0,
            is_deleted: false,
        };
        // add first exchange including system prompt message
        self.cache.add_message(system_message);
        self.cache.add_exchange(exchange.clone());
        exchange
    }

    pub fn subsequent_exchange(&mut self) -> Exchange {
        if let Some(last) = self.cache.get_last_exchange() {
            Exchange {
                id: self.cache.new_exchange_id(),
                conversation_id: self.cache.get_conversation_id(),
                created_at: 0,
                previous_exchange_id: Some(last.id),
                is_deleted: false,
            }
        } else {
            // should never happen as first_exchange is always added in new()
            unreachable!("subsequent_exchange called before first_exchange");
        }
    }

    pub fn new_exchange(
        &mut self,
        question: &str,
        max_token_length: usize,
    ) -> Vec<ChatMessage> {
        // token budget for the system prompt
        let system_prompt_token_length = self.get_n_keep().unwrap_or(0);

        // add the partial exchange (question) to the conversation
        let exchange = self.subsequent_exchange();

        let user_message = Message {
            id: self.cache.new_message_id(),
            conversation_id: self.cache.get_conversation_id(),
            exchange_id: exchange.id,
            role: PromptRole::User,
            message_type: "text".to_string(),
            content: question.to_string(),
            has_attachments: false,
            token_length: None,
            created_at: 0,
            is_deleted: false,
        };
        self.cache.add_exchange(exchange);
        // new prompt only has user message, answer is not yet generated
        self.cache.add_message(user_message);

        let current_exchanges = self.cache.get_exchanges();

        // Collect messages while respecting token limits
        let mut messages: Vec<ChatMessage> = Vec::new();
        let mut total_tokens = system_prompt_token_length;

        let mut system_message: Option<ChatMessage> = None;

        // Add messages from most recent to oldest, respecting token limit
        for exchange in current_exchanges.into_iter().rev() {
            for msg in self
                .cache
                .get_exchange_messages(exchange.id)
                .into_iter()
                .rev()
            {
                let msg_token_length =
                    msg.token_length.map(|len| len as usize).unwrap_or(0);

                if msg.role == PromptRole::System {
                    system_message = Some(ChatMessage {
                        role: msg.role,
                        content: msg.content.clone(),
                    });
                    continue; // system prompt is included separately
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
            if total_tokens >= max_token_length {
                break;
            }
        }

        // ensure the system prompt is always included
        // last, before reverse, so it will be at the beginning
        if let Some(system_message) = system_message {
            messages.push(system_message);
        }
        // Reverse the messages to maintain chronological order
        messages.reverse();
        messages
    }

    pub fn get_completion_options(&self) -> &ChatCompletionOptions {
        &self.completion_options
    }

    pub fn set_model(&mut self, model: &LLMDefinition) {
        self.completion_options.update_from_model(model);
    }

    pub fn get_role_prefix(&self, role: PromptRole) -> &str {
        self.prompt_options.get_role_prefix(role)
    }

    pub fn get_context_size(&self) -> Option<usize> {
        self.prompt_options.get_context_size()
    }

    pub fn set_context_size(&mut self, context_size: usize) {
        self.prompt_options.set_context_size(context_size);
    }

    pub fn get_n_keep(&self) -> Option<usize> {
        self.completion_options.get_n_keep()
    }

    pub fn get_prompt_template(&self) -> Option<&str> {
        self.prompt_template.as_deref()
    }

    pub fn get_instruction(&self) -> Option<String> {
        self.cache.get_system_prompt()
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
        //self.set_system_prompt(system_prompt.clone());

        if let Some(exchanges) = prompt.exchanges() {
            // Create a new exchange with the system prompt
            let exchange = self.first_exchange(Some(system_prompt));
            let _result = db_conn.finalize_exchange(&exchange, &self.cache);

            for loaded_exchange in exchanges.iter() {
                let exchange = self.subsequent_exchange();
                let exchange_id = exchange.id;
                let content = loaded_exchange.question.clone();
                let user_message = Message {
                    id: self.cache.new_message_id(),
                    conversation_id: self.cache.get_conversation_id(),
                    exchange_id,
                    role: PromptRole::User,
                    message_type: "text".to_string(),
                    has_attachments: false,
                    token_length: Some(simple_token_estimator(&content, None)),
                    content,
                    created_at: 0, // Use proper timestamp
                    is_deleted: false,
                };
                self.cache.add_message(user_message);

                let content = loaded_exchange.answer.clone();
                let assistant_message = Message {
                    id: self.cache.new_message_id(),
                    conversation_id: self.cache.get_conversation_id(),
                    exchange_id,
                    role: PromptRole::Assistant,
                    message_type: "text".to_string(),
                    has_attachments: false,
                    token_length: Some(simple_token_estimator(&content, None)),
                    content,
                    created_at: 0, // Use proper timestamp
                    is_deleted: false,
                };
                self.cache.add_message(assistant_message);
                // add to exchange must be done before finalizing
                // exchange that is used in finalize_exchange is a reference to version
                // it just commited to cache
                self.cache.add_exchange(exchange);
                if let Some(exchange) = self.cache.get_last_exchange() {
                    let _result =
                        db_conn.finalize_exchange(&exchange, &self.cache);
                }
            }
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

pub struct ExchangeHandler;

impl ExchangeHandler {
    pub fn append_response(cache: &mut ConversationCache, answer: &str) {
        let last_exchange = cache.get_last_exchange();

        if let Some(exchange) = last_exchange {
            let last_message =
                cache.get_last_message_of_exchange(exchange.id).cloned();

            match last_message {
                Some(msg) if msg.role == PromptRole::Assistant => {
                    // If the last message is from Assistant, append to it
                    let new_content =
                        format!("{}{}", msg.content, answer).to_string();
                    cache.update_message_by_id(msg.id, &new_content, None);
                }
                _ => {
                    // If the last message is from User or there's no message, create a new Assistant message
                    let new_message = Message {
                        id: cache.new_message_id(),
                        conversation_id: cache.get_conversation_id(),
                        exchange_id: exchange.id,
                        role: PromptRole::Assistant,
                        message_type: "text".to_string(),
                        content: answer.to_string(),
                        has_attachments: false,
                        token_length: None,
                        created_at: 0, // You might want to use a proper timestamp here
                        is_deleted: false,
                    };
                    cache.add_message(new_message);
                }
            }
        } else {
            // If there's no exchange, something went wrong
            eprintln!("Error: No exchange found when trying to append answer");
        }
    }

    pub fn get_last_response(cache: &mut ConversationCache) -> Option<String> {
        cache
            .get_last_exchange()
            .and_then(|last_exchange| {
                cache.get_last_message_of_exchange(last_exchange.id)
            })
            .and_then(|last_message| {
                if last_message.role == PromptRole::Assistant {
                    Some(last_message.content.clone())
                } else {
                    None
                }
            })
    }

    pub fn put_last_response(
        cache: &mut ConversationCache,
        answer: &str,
        tokens_predicted: Option<usize>,
    ) -> Option<Exchange> {
        // Gather all necessary information
        let exchange_data = {
            let last_exchange = cache.get_last_exchange()?;
            let messages = cache.get_exchange_messages(last_exchange.id);
            let user_message =
                messages.iter().find(|m| m.role == PromptRole::User)?;
            let assistant_message =
                messages.iter().find(|m| m.role == PromptRole::Assistant)?;

            (
                last_exchange.clone(),
                assistant_message.id,
                user_message.id,
                user_message.content.clone(),
            )
        };

        let (exchange, assistant_message_id, user_message_id, user_content) =
            exchange_data;

        // Calculate user token length
        let user_token_length = tokens_predicted.map(|tokens| {
            let chars_per_token = calculate_chars_per_token(answer, tokens);
            simple_token_estimator(&user_content, Some(chars_per_token))
        });

        // Perform all updates in a single mutable borrow
        {
            if let Some(tokens) = tokens_predicted {
                // Update assistant's message
                cache.update_message_by_id(
                    assistant_message_id,
                    answer,
                    Some(tokens as i64),
                );

                // Update user's message token length
                if let Some(length) = user_token_length {
                    cache.update_message_token_length(user_message_id, length);
                }
            } else {
                // If no tokens_predicted, just update the content without changing token length
                cache.update_message_by_id(assistant_message_id, answer, None);
            }
        }

        Some(exchange)
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
