use lumni::api::error::ApplicationError;

use super::db::{
    ConversationCache, ConversationDatabase, ConversationId, Exchange,
    ExchangeId, Message, ModelId,
};
use super::prompt::Prompt;
use super::{
    ChatCompletionOptions, ChatMessage, PromptOptions, PromptRole,
    DEFAULT_N_PREDICT, DEFAULT_TEMPERATURE, PERSONAS,
};
pub use crate::external as lumni;

pub struct PromptInstruction {
    cache: ConversationCache,
    completion_options: ChatCompletionOptions,
    prompt_options: PromptOptions, // TODO: get from db
    system_prompt: SystemPrompt,
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
            system_prompt: SystemPrompt::default(),
            prompt_template: None,
        }
    }
}

impl PromptInstruction {
    pub fn new(
        instruction: Option<String>,
        assistant: Option<String>,
        options: Option<&String>,
        db_conn: &ConversationDatabase,
    ) -> Result<Self, ApplicationError> {
        let mut prompt_instruction = PromptInstruction::default();
        if let Some(json_str) = options {
            prompt_instruction
                .get_prompt_options_mut()
                .update_from_json(json_str);
            prompt_instruction
                .get_completion_options_mut()
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

        if let Some(assistant) = assistant {
            prompt_instruction.preload_from_assistant(
                assistant,
                instruction, // add user-instruction with assistant
            )?;
        } else if let Some(instruction) = instruction {
            prompt_instruction.set_system_prompt(instruction);
        };

        // Create a new Conversation in the database
        let conversation_id =
            { db_conn.new_conversation("New Conversation", None)? };
        prompt_instruction.cache.set_conversation_id(conversation_id);

        Ok(prompt_instruction)
    }

    pub fn reset_history(
        &mut self,
        db_conn: &ConversationDatabase,
    ) -> Result<(), ApplicationError> {
        // reset by creating a new conversation
        let current_conversation_id =
            db_conn.new_conversation("New Conversation", None)?;
        self.cache.set_conversation_id(current_conversation_id);
        Ok(())
    }

    pub fn append_last_response(&mut self, answer: &str) {
        ExchangeHandler::append_response(
            &mut self.cache,
            answer,
        );
    }

    pub fn get_last_response(&mut self) -> Option<String> {
        ExchangeHandler::get_last_response(&mut self.cache)
    }

    pub fn put_last_response(
        &mut self,
        answer: &str,
        tokens_predicted: Option<usize>,
        db_conn: &ConversationDatabase,
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

    fn first_exchange(&self) -> Exchange {
        Exchange {
            id: ExchangeId(0),
            conversation_id: self.cache.get_conversation_id(),
            model_id: ModelId(0),
            system_prompt: Some(self.system_prompt.instruction.clone()),
            completion_options: serde_json::to_value(&self.completion_options)
                .ok(),
            prompt_options: serde_json::to_value(&self.prompt_options).ok(),
            completion_tokens: None,
            prompt_tokens: None,
            created_at: 0,
            previous_exchange_id: None,
            is_deleted: false,
        }
    }

    pub fn subsequent_exchange(&mut self) -> Exchange {
        if let Some(last) = self.cache.get_last_exchange() {
            Exchange {
                id: self.cache.new_exchange_id(),
                conversation_id: self.cache.get_conversation_id(),
                model_id: last.model_id,
                system_prompt: last.system_prompt.clone(),
                completion_options: last.completion_options.clone(),
                prompt_options: last.prompt_options.clone(),
                completion_tokens: None,
                prompt_tokens: None,
                created_at: 0,
                previous_exchange_id: Some(last.id),
                is_deleted: false,
            }
        } else {
            // create first exchange
            let exchange = self.first_exchange();
            // add system prompt
            let system_message = Message {
                id: self.cache.new_message_id(),
                conversation_id: self.cache.get_conversation_id(),
                exchange_id: exchange.id,
                role: PromptRole::System,
                message_type: "text".to_string(),
                content: self.system_prompt.get_instruction().to_string(),
                has_attachments: false,
                token_length: self
                    .system_prompt
                    .get_token_length()
                    .map(|len| len as i32),
                created_at: 0,
                is_deleted: false,
            };
            // add first exchange including system prompt message
            self.cache.add_message(system_message);
            self.cache.add_exchange(exchange.clone());

            // return subsequent exchange
            Exchange {
                id: self.cache.new_exchange_id(),
                prompt_tokens: None,
                completion_tokens: None,
                previous_exchange_id: Some(exchange.id),
                ..exchange
            }
        }
    }

    pub fn new_exchange(
        &mut self,
        question: &str,
        token_length: Option<usize>,
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
            token_length: token_length.map(|len| len as i32),
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
        // after reverse, the system prompt will be at the beginning
        messages.push(ChatMessage {
            role: PromptRole::System,
            content: self.system_prompt.get_instruction().to_string(),
        });

        // Reverse the messages to maintain chronological order
        messages.reverse();
        messages
    }

    pub fn get_completion_options(&self) -> &ChatCompletionOptions {
        // no need to change this yet
        &self.completion_options
    }

    pub fn get_completion_options_mut(&mut self) -> &mut ChatCompletionOptions {
        // no need to change this yet
        &mut self.completion_options
    }

    pub fn get_prompt_options(&self) -> &PromptOptions {
        // no need to change this yet
        &self.prompt_options
    }

    pub fn get_prompt_options_mut(&mut self) -> &mut PromptOptions {
        // no need to change this yet
        &mut self.prompt_options
    }

    pub fn get_n_keep(&self) -> Option<usize> {
        // no need to change this yet
        self.completion_options.get_n_keep()
    }

    pub fn set_system_prompt(&mut self, instruction: String) {
        self.system_prompt = SystemPrompt::new(instruction);
    }

    pub fn get_system_token_length(&self) -> Option<usize> {
        self.system_prompt.get_token_length()
    }

    pub fn set_system_token_length(&mut self, token_length: Option<usize>) {
        self.system_prompt.set_token_length(token_length);
    }

    pub fn get_prompt_template(&self) -> Option<&str> {
        self.prompt_template.as_deref()
    }

    pub fn get_instruction(&self) -> &str {
        self.system_prompt.get_instruction()
    }

    pub fn preload_from_assistant(
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
        self.set_system_prompt(system_prompt.clone());

        if let Some(exchanges) = prompt.exchanges() {
            // Create a new exchange with the system prompt
            let exchange = self.first_exchange();
            let system_message = Message {
                id: self.cache.new_message_id(),
                conversation_id: self.cache.get_conversation_id(),
                exchange_id: exchange.id,
                role: PromptRole::System,
                message_type: "text".to_string(),
                content: system_prompt,
                has_attachments: false,
                token_length: None,
                created_at: 0,
                is_deleted: false,
            };
            self.cache.add_message(system_message);
            self.cache.add_exchange(exchange);

            for loaded_exchange in exchanges.iter() {
                let exchange = self.subsequent_exchange();

                let user_message = Message {
                    id: self.cache.new_message_id(),
                    conversation_id: self.cache.get_conversation_id(),
                    exchange_id: exchange.id,
                    role: PromptRole::User,
                    message_type: "text".to_string(),
                    content: loaded_exchange.question.clone(),
                    has_attachments: false,
                    token_length: None,
                    created_at: 0, // Use proper timestamp
                    is_deleted: false,
                };
                let assistant_message = Message {
                    id: self.cache.new_message_id(),
                    conversation_id: self.cache.get_conversation_id(),
                    exchange_id: exchange.id,
                    role: PromptRole::Assistant,
                    message_type: "text".to_string(),
                    content: loaded_exchange.answer.clone(),
                    has_attachments: false,
                    token_length: None,
                    created_at: 0, // Use proper timestamp
                    is_deleted: false,
                };
                self.cache.add_message(user_message);
                self.cache.add_message(assistant_message);
                self.cache.add_exchange(exchange);
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
    pub fn append_response(
        cache: &mut ConversationCache,
        answer: &str,
    ) {
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
                        // TODO: need to implement model based counting
                        // simplified (in-accurate, but better than keeping it at 0) version
                        token_length: Some(answer.len() as i32),
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
        // Capture the necessary message and exchange IDs in a separate scope
        let (message_id, exchange) = {
            //let cache = db_conn.cache.lock().unwrap();
            let last_exchange = cache.get_last_exchange()?;
            let last_message =
                cache.get_last_message_of_exchange(last_exchange.id)?;

            // Check if the last message was from an assistant, and capture IDs
            if last_message.role == PromptRole::Assistant {
                (Some(last_message.id), Some(last_exchange.clone()))
            } else {
                (None, None)
            }
        };

        // Only proceed if we have valid IDs
        if let (Some(message_id), Some(exchange)) = (message_id, exchange) {
            // Perform the update in a separate cache lock scope
            let token_length = tokens_predicted.map(|t| t as i32);
            //let mut cache = db_conn.cache.lock().unwrap();
            cache.update_message_by_id(message_id, answer, token_length);
            Some(exchange)
        } else {
            None
        }
    }
}

struct SystemPrompt {
    instruction: String,
    token_length: Option<usize>,
}

impl SystemPrompt {
    fn default() -> Self {
        SystemPrompt {
            instruction: "".to_string(),
            token_length: Some(0),
        }
    }

    fn new(instruction: String) -> Self {
        SystemPrompt {
            instruction,
            token_length: None,
        }
    }

    fn get_instruction(&self) -> &str {
        &self.instruction
    }

    fn get_token_length(&self) -> Option<usize> {
        self.token_length
    }

    fn set_token_length(&mut self, token_length: Option<usize>) {
        self.token_length = token_length;
    }
}
