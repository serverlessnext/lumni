use super::{ModelFormatterTrait, PromptRole};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Llama3 {
    name: String,
    stop_tokens: Vec<String>,
}

impl Llama3 {
    pub fn new() -> Self {
        Llama3 {
            name: "llama3".to_string(),
            stop_tokens: vec![
                "<|eot_id|>".to_string(),
                "<|end_of_text|>".to_string(),
                "### User: ".to_string(),
                "### Human: ".to_string(),
                "User: ".to_string(),
                "Human: ".to_string(),
            ],
        }
    }
}

impl ModelFormatterTrait for Llama3 {
    fn get_stop_tokens(&self) -> &Vec<String> {
        &self.stop_tokens
    }

    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        if let Some(instruction) = instruction {
            return format!(
                "<|begin_of_text|>{}",
                self.fmt_prompt_message(&PromptRole::System, instruction)
            )
            .to_string();
        } else {
            return "<|begin_of_text|>".to_string();
        }
    }

    fn fmt_prompt_message(
        &self,
        prompt_role: &PromptRole,
        message: &str,
    ) -> String {
        let role_handle = match prompt_role {
            PromptRole::User => "user",
            PromptRole::Assistant => "assistant",
            PromptRole::System => "system",
        };
        let mut prompt_message = String::new();
        prompt_message.push_str(&format!(
            "<|start_header_id|>{}<|end_header_id|>\n{}",
            role_handle,
            message
        ));
        if !message.is_empty() {
            prompt_message.push_str("<|eot_id|>\n");
        }
        prompt_message
    }
}
