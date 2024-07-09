use std::sync::{Arc, Mutex, MutexGuard};

use lumni::api::error::ApplicationError;

use super::prompt::Prompt;
use super::schema::{
    ConversationId, Exchange, InMemoryDatabase, Message, ModelId,
};
use super::{
    ChatCompletionOptions, ChatMessage, PromptOptions, PromptRole,
    DEFAULT_N_PREDICT, DEFAULT_TEMPERATURE, PERSONAS,
};
pub use crate::external as lumni;

pub struct PromptInstruction {
    completion_options: ChatCompletionOptions,
    prompt_options: PromptOptions, // TODO: get from db
    system_prompt: SystemPrompt,
    prompt_template: Option<String>,
    pub db: Arc<Mutex<InMemoryDatabase>>,
    current_conversation_id: ConversationId,
}

impl Default for PromptInstruction {
    fn default() -> Self {
        let completion_options = ChatCompletionOptions::default()
            .set_temperature(DEFAULT_TEMPERATURE)
            .set_n_predict(DEFAULT_N_PREDICT)
            .set_cache_prompt(true)
            .set_stream(true);

        PromptInstruction {
            completion_options,
            prompt_options: PromptOptions::default(),
            system_prompt: SystemPrompt::default(),
            prompt_template: None,
            db: Arc::new(Mutex::new(InMemoryDatabase::new())),
            current_conversation_id: ConversationId(0),
        }
    }
}

impl PromptInstruction {
    pub fn new(
        instruction: Option<String>,
        assistant: Option<String>,
        options: Option<&String>,
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
        let conversation_id = {
            let mut db_lock = prompt_instruction.db.lock().unwrap();
            db_lock.new_conversation("New Conversation", None)
        };
        prompt_instruction.current_conversation_id = conversation_id;

        Ok(prompt_instruction)
    }

    pub fn reset_history(&mut self) {
        // Create a new Conversation in the database
        let new_conversation_id = {
            let mut db_lock = self.db.lock().unwrap();
            db_lock.new_conversation(
                "New Conversation",
                Some(self.current_conversation_id),
            )
        };
        self.current_conversation_id = new_conversation_id;
    }

    pub fn append_last_response(&mut self, answer: &str) {
        ExchangeHandler::append_response(
            &mut self.db.lock().unwrap(),
            self.current_conversation_id,
            answer,
        );
    }

    pub fn get_last_response(&self) -> Option<String> {
        ExchangeHandler::get_last_response(
            &self.db.lock().unwrap(),
            self.current_conversation_id,
        )
    }

    pub fn put_last_response(
        &mut self,
        answer: &str,
        tokens_predicted: Option<usize>,
    ) {
        ExchangeHandler::put_last_response(
            &mut self.db.lock().unwrap(),
            self.current_conversation_id,
            answer,
            tokens_predicted,
        );
    }

    fn first_exchange(
        &self,
        db_lock: &mut MutexGuard<'_, InMemoryDatabase>,
    ) -> Exchange {
        Exchange {
            id: db_lock.new_exchange_id(),
            conversation_id: self.current_conversation_id,
            model_id: ModelId(0),
            system_prompt: self.system_prompt.instruction.clone(),
            completion_options: serde_json::to_value(&self.completion_options)
                .unwrap_or_default(),
            prompt_options: serde_json::to_value(&self.prompt_options)
                .unwrap_or_default(),
            completion_tokens: 0,
            prompt_tokens: 0,
            created_at: 0,
            previous_exchange_id: None,
        }
    }

    pub fn subsequent_exchange(
        &mut self,
        question: &str,
        token_length: Option<usize>,
        max_token_length: usize,
    ) -> Vec<ChatMessage> {
        let mut db_lock = self.db.lock().unwrap();

        // token budget for the system prompt
        let system_prompt_token_length = self.get_n_keep().unwrap_or(0);

        // add the partial exchange (question) to the conversation
        let last_exchange =
            db_lock.get_last_exchange(self.current_conversation_id);

        let exchange = if let Some(last) = last_exchange {
            // add exchange based on the last one
            Exchange {
                id: db_lock.new_exchange_id(),
                conversation_id: self.current_conversation_id,
                model_id: last.model_id,
                system_prompt: last.system_prompt.clone(), // copy from previous exchange
                completion_options: last.completion_options.clone(), // copy from previous exchange
                prompt_options: last.prompt_options.clone(), // copy from previous exchange
                completion_tokens: 0,
                prompt_tokens: 0,
                created_at: 0,
                previous_exchange_id: Some(last.id),
            }
        } else {
            // create first exchange
            let exchange = self.first_exchange(&mut db_lock);
            let system_message = Message {
                id: db_lock.new_message_id(),
                conversation_id: self.current_conversation_id,
                exchange_id: exchange.id,
                role: PromptRole::System,
                message_type: "text".to_string(),
                content: self.system_prompt.get_instruction().to_string(),
                has_attachments: false,
                token_length: self.system_prompt.get_token_length().unwrap_or(0) as i32,
                created_at: 0,
            };
            db_lock.add_message(system_message);
            exchange
        };

        let user_message = Message {
            id: db_lock.new_message_id(),
            conversation_id: self.current_conversation_id,
            exchange_id: exchange.id,
            role: PromptRole::User,
            message_type: "text".to_string(),
            content: question.to_string(),
            has_attachments: false,
            token_length: token_length.unwrap_or(0) as i32,
            created_at: 0,
        };

        db_lock.add_exchange(exchange);
        // new_prompt only has user question, answer is added later
        db_lock.add_message(user_message);

        let current_exchanges =
            db_lock.get_conversation_exchanges(self.current_conversation_id);

        // Collect messages while respecting token limits
        let mut messages: Vec<ChatMessage> = Vec::new();
        let mut total_tokens = system_prompt_token_length;

        // Add messages from most recent to oldest, respecting token limit
        for exchange in current_exchanges.into_iter().rev() {
            for msg in
                db_lock.get_exchange_messages(exchange.id).into_iter().rev()
            {
                if msg.role == PromptRole::System {
                    continue; // system prompt is included separately
                }
                if total_tokens + msg.token_length as usize <= max_token_length
                {
                    total_tokens += msg.token_length as usize;
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
            let mut db_lock = self.db.lock().unwrap();

            // Create a new exchange with the system prompt
            let exchange = self.first_exchange(&mut db_lock);
            let system_message = Message {
                id: db_lock.new_message_id(),
                conversation_id: self.current_conversation_id,
                exchange_id: exchange.id,
                role: PromptRole::System,
                message_type: "text".to_string(),
                content: system_prompt,
                has_attachments: false,
                token_length: 0, // TODO: compute token length
                created_at: 0,
            };
            db_lock.add_message(system_message);

            for loaded_exchange in exchanges.iter() {
                let user_message = Message {
                    id: db_lock.new_message_id(),
                    conversation_id: self.current_conversation_id,
                    exchange_id: exchange.id,
                    role: PromptRole::User,
                    message_type: "text".to_string(),
                    content: loaded_exchange.question.clone(),
                    has_attachments: false,
                    token_length: 0, // Implement proper token counting
                    created_at: 0,   // Use proper timestamp
                };
                let assistant_message = Message {
                    id: db_lock.new_message_id(),
                    conversation_id: self.current_conversation_id,
                    exchange_id: exchange.id,
                    role: PromptRole::Assistant,
                    message_type: "text".to_string(),
                    content: loaded_exchange.answer.clone(),
                    has_attachments: false,
                    token_length: 0, // Implement proper token counting
                    created_at: 0,   // Use proper timestamp
                };
                db_lock.add_message(user_message);
                db_lock.add_message(assistant_message);
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
        db_lock: &mut MutexGuard<InMemoryDatabase>,
        current_conversation_id: ConversationId,
        answer: &str,
    ) {
        let last_exchange = db_lock.get_last_exchange(current_conversation_id);

        if let Some(exchange) = last_exchange {
            let last_message =
                db_lock.get_last_message_of_exchange(exchange.id).cloned();

            match last_message {
                Some(msg) if msg.role == PromptRole::Assistant => {
                    // If the last message is from Assistant, append to it
                    let new_content =
                        format!("{}{}", msg.content, answer).to_string();
                    db_lock.update_message_by_id(msg.id, &new_content, None);
                }
                _ => {
                    // If the last message is from User or there's no message, create a new Assistant message
                    let new_message = Message {
                        id: db_lock.new_message_id(),
                        conversation_id: current_conversation_id,
                        exchange_id: exchange.id,
                        role: PromptRole::Assistant,
                        message_type: "text".to_string(),
                        content: answer.to_string(),
                        has_attachments: false,
                        token_length: answer.len() as i32, // Simplified token count
                        created_at: 0, // You might want to use a proper timestamp here
                    };
                    db_lock.add_message(new_message);
                }
            }
        } else {
            // If there's no exchange, something went wrong
            eprintln!("Error: No exchange found when trying to append answer");
        }
    }

    pub fn get_last_response(
        db_lock: &MutexGuard<InMemoryDatabase>,
        current_conversation_id: ConversationId,
    ) -> Option<String> {
        db_lock
            .get_last_exchange(current_conversation_id)
            .and_then(|last_exchange| {
                db_lock.get_last_message_of_exchange(last_exchange.id)
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
        db_lock: &mut MutexGuard<InMemoryDatabase>,
        current_conversation_id: ConversationId,
        answer: &str,
        tokens_predicted: Option<usize>,
    ) {
        let (message_id, is_assistant) = if let Some(last_exchange) =
            db_lock.get_last_exchange(current_conversation_id)
        {
            if let Some(last_message) =
                db_lock.get_last_message_of_exchange(last_exchange.id)
            {
                // Check the role directly here and only pass on the ID if it's an assistant's message
                (
                    Some(last_message.id),
                    last_message.role == PromptRole::Assistant,
                )
            } else {
                (None, false)
            }
        } else {
            (None, false)
        };

        // Perform the update if the message is from an assistant
        if let (Some(id), true) = (message_id, is_assistant) {
            let token_length = tokens_predicted.map(|t| t as i32);
            db_lock.update_message_by_id(id, answer, token_length);
        }
    }
}

struct SystemPrompt {
    instruction: String,
    token_length: Option<usize>,
}

impl SystemPrompt {
    pub fn default() -> Self {
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
