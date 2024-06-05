use super::exchange::ChatExchange;

#[derive(Debug, Clone)]
pub struct ChatHistory {
    exchanges: Vec<ChatExchange>,
}

impl ChatHistory {
    pub fn new() -> Self {
        ChatHistory {
            exchanges: Vec::new(),
        }
    }

    pub fn new_with_exchanges(exchanges: Vec<ChatExchange>) -> Self {
        ChatHistory { exchanges }
    }

    pub fn clear(&mut self) {
        self.exchanges.clear();
    }

    pub fn get_exchanges(&self) -> &Vec<ChatExchange> {
        &self.exchanges
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
        max_token_length: Option<usize>,
        system_prompt_length: usize,
    ) -> Vec<ChatExchange> {
        let mut result_exchanges = Vec::new();

        // instruction and new exchange should always be added,
        // calculate the remaining tokens to see how much history can be added
        let tokens_remaining = if let Some(max) = max_token_length {
            let tokens_required = new_exchange.get_token_length().unwrap_or(0)
                + system_prompt_length;
            max.saturating_sub(tokens_required)
        } else {
            usize::MAX // no limit
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
}
