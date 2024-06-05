use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatExchange {
    question: String,
    answer: String,
    token_length: Option<usize>,
}

impl ChatExchange {
    pub fn new(question: String, answer: String) -> Self {
        ChatExchange {
            question,
            answer,
            token_length: None,
        }
    }

    pub fn get_question(&self) -> &str {
        &self.question
    }

    pub fn get_answer(&self) -> &str {
        &self.answer
    }

    pub fn set_answer(&mut self, answer: String) {
        self.answer = answer;
    }

    pub fn push_to_answer(&mut self, text: &str) {
        self.answer.push_str(text);
    }

    pub fn get_token_length(&self) -> Option<usize> {
        self.token_length
    }

    pub fn set_token_length(&mut self, token_length: usize) {
        self.token_length = Some(token_length);
    }
}
