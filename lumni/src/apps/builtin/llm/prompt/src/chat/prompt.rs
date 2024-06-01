use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Prompt {
    name: String,
    system_prompt: Option<String>,
    prompt_template: Option<String>,
    exchanges: Option<Vec<ChatExchange>>,
}

impl Prompt {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    pub fn prompt_template(&self) -> Option<&str> {
        self.prompt_template.as_deref()
    }

    pub fn exchanges(&self) -> Option<&Vec<ChatExchange>> {
        self.exchanges.as_ref()
    }
}

pub struct SystemPrompt {
    instruction: String,
    token_length: usize,
}

impl SystemPrompt {
    pub fn default() -> Self {
        SystemPrompt {
            instruction: "".to_string(),
            token_length: 0,
        }
    }

    pub fn new(instruction: String, token_length: usize) -> Self {
        SystemPrompt {
            instruction,
            token_length,
        }
    }

    pub fn get_instruction(&self) -> &str {
        &self.instruction
    }

    pub fn get_token_length(&self) -> usize {
        self.token_length
    }
}

#[derive(Debug, Deserialize, Clone)]
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
