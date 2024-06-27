use std::error::Error;

use serde::{Deserialize, Serialize};

use super::exchange::ChatExchange;
use super::{LLMDefinition, PromptRole};

#[derive(Debug, Clone)]
pub struct ChatHistory {
    exchanges: Vec<ChatExchange>,
    keep_n: Option<usize>,  // keep n exchanges in history if reset
}

impl ChatHistory {
    pub fn new() -> Self {
        ChatHistory {
            exchanges: Vec::new(),
            keep_n: None,
        }
    }

    pub fn new_with_exchanges(exchanges: Vec<ChatExchange>) -> Self {
        let keep_n = Some(exchanges.len()); // keep initial exchanges if reset
        ChatHistory { exchanges, keep_n }
    }

    pub fn reset(&mut self) {
        if let Some(keep_n) = self.keep_n {
            self.exchanges.truncate(keep_n);
        } else {
            self.exchanges.clear();
        }
    }

    pub fn get_last_exchange_mut(&mut self) -> Option<&mut ChatExchange> {
        self.exchanges.last_mut()
    }

    pub fn update_last_exchange(&mut self, answer: &str) {
        if let Some(last_exchange) = self.exchanges.last_mut() {
            last_exchange.push_to_answer(answer);
        }
    }

    pub fn new_prompt(
        &mut self,
        new_exchange: ChatExchange,
        max_token_length: usize,
        system_prompt_length: Option<usize>,
    ) -> Vec<ChatExchange> {
        let mut result_exchanges = Vec::new();

        // instruction and new exchange should always be added,
        // calculate the remaining tokens to see how much history can be added
        let tokens_remaining = {
            let tokens_required = new_exchange.get_token_length().unwrap_or(0)
                + system_prompt_length.unwrap_or(0);
            max_token_length.saturating_sub(tokens_required)
        };

        // cleanup last exchange if second (answer) element is un-answered (empty)
        if let Some(last_exchange) = self.exchanges.last() {
            if last_exchange.get_answer().is_empty() {
                self.exchanges.pop();
            }
        }

        let mut history_tokens = 0;

        for exchange in self.exchanges.iter().rev() {
            let exchange_tokens = exchange.get_token_length().unwrap_or(0);
            if history_tokens + exchange_tokens > tokens_remaining {
                break;
            }
            history_tokens += exchange_tokens;
            result_exchanges.insert(0, exchange.clone());
        }

        // add the new exchange to both the result and the history
        result_exchanges.push(new_exchange.clone());
        self.exchanges.push(new_exchange);
        result_exchanges
    }

    pub fn exchanges_to_string<'a, I>(
        model: &LLMDefinition,
        exchanges: I,
    ) -> Result<String, Box<dyn Error>>
    where
        I: IntoIterator<Item = &'a ChatExchange>,
    {
        let mut prompt = String::new();
        let formatter = model.get_formatter();

        for exchange in exchanges {
            prompt.push_str(
                &formatter.fmt_prompt_message(
                    PromptRole::User,
                    exchange.get_question(),
                ),
            );
            prompt.push_str(&formatter.fmt_prompt_message(
                PromptRole::Assistant,
                exchange.get_answer(),
            ));
        }
        Ok(prompt)
    }

    pub fn exchanges_to_messages<'a, I>(
        exchanges: I,
        system_prompt: Option<&str>,
        fn_role_name: &dyn Fn(PromptRole) -> &'static str,
    ) -> Vec<ChatMessage>
    where
        I: IntoIterator<Item = &'a ChatExchange>,
    {
        let mut messages = Vec::new();

        if let Some(system_prompt) = system_prompt {
            messages.push(ChatMessage {
                role: fn_role_name(PromptRole::System).to_string(),
                content: system_prompt.to_string(),
            });
        }

        for exchange in exchanges {
            messages.push(ChatMessage {
                role: fn_role_name(PromptRole::User).to_string(),
                content: exchange.get_question().to_string(),
            });

            // dont add empty answers
            let content = exchange.get_answer().to_string();
            if content.is_empty() {
                continue;
            }
            messages.push(ChatMessage {
                role: fn_role_name(PromptRole::Assistant).to_string(),
                content,
            });
        }
        messages
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    role: String,
    content: String,
}
