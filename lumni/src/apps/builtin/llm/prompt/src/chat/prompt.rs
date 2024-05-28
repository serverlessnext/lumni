use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Prompt {
    name: String,
    system_prompt: Option<String>,
    prompt_template: Option<String>,
    exchanges: Option<Vec<Exchange>>,
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

    pub fn exchanges(&self) -> Option<&Vec<Exchange>> {
        self.exchanges.as_ref()
    }
}

#[derive(Debug, Deserialize)]
pub struct Exchange {
    question: String,
    answer: String,
}

impl Exchange {
    pub fn question_and_answer(&self) -> (String, String) {
        (self.question.clone(), self.answer.clone())
    }
}
