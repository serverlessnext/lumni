use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use lumni::HttpClient;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::options::ChatOptions;
use super::prompt::Prompt;
use super::send::send_payload;
use super::{ChatCompletionResponse, PERSONAS};
use crate::external as lumni;

#[derive(Deserialize)]
struct TokenResponse {
    tokens: Vec<usize>,
}

pub struct ChatSession {
    http_client: HttpClient,
    exchanges: Vec<(String, String)>, // (question, answer)
    max_history: usize,
    n_keep: usize,
    instruction: String,         // system
    user_prompt: Option<String>, // put question in {{ USER_QUESTION }}
    model: String,
    assistant: Option<String>,
    options: ChatOptions,
}

impl ChatSession {
    pub fn new() -> ChatSession {
        ChatSession {
            http_client: HttpClient::new(),
            exchanges: Vec::new(),
            max_history: 20, // TODO: base on max tokens
            n_keep: 0,
            instruction: "".to_string(),
            user_prompt: None,
            model: "llama3".to_string(),
            assistant: None,
            options: ChatOptions::default(),
        }
    }

    pub fn set_instruction(&mut self, instruction: String) -> &mut Self {
        self.instruction = instruction;
        self
    }

    pub fn set_user_prompt(
        &mut self,
        user_prompt: Option<String>,
    ) -> &mut Self {
        self.user_prompt = user_prompt;
        self
    }

    pub fn set_model(&mut self, model: String) -> &mut Self {
        self.model = model;
        self
    }

    pub fn set_assistant(&mut self, assistant: Option<String>) -> &mut Self {
        self.assistant = assistant;
        self
    }

    pub fn set_options(&mut self, options: ChatOptions) -> &mut Self {
        self.options = options;
        self
    }

    pub async fn init(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(assistant) = self.assistant.clone() {
            // Find the selected persona by name
            let assistant_prompts: Vec<Prompt> =
                serde_yaml::from_str(PERSONAS)?;
            if let Some(prompt) = assistant_prompts
                .into_iter()
                .find(|p| p.name() == assistant)
            {
                // Set session instruction from persona's system prompt
                if let Some(system_prompt) = prompt.system_prompt() {
                    self.instruction = system_prompt.to_string();
                }

                // Load predefined exchanges from persona if available
                if let Some(exchanges) = prompt.exchanges() {
                    self.exchanges = exchanges
                        .into_iter()
                        .map(|exchange| exchange.question_and_answer())
                        .collect();
                }

                if let Some(user_prompt) = prompt.user_prompt() {
                    self.user_prompt = Some(user_prompt.to_string());
                }
            } else {
                return Err("Selected persona not found in the dataset".into());
            }
        }
        self.tokenize_and_set_n_keep().await?;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.exchanges.clear();
    }

    pub fn update_last_exchange(&mut self, answer: String) {
        if let Some(last_exchange) = self.exchanges.last_mut() {
            last_exchange.1 = answer;
            self.trim_history();
        }
    }

    pub async fn tokenize_and_set_n_keep(
        &mut self,
    ) -> Result<(), Box<dyn Error>> {
        let url = "http://localhost:8080/tokenize";
        //let body_content =
        //    serde_json::json!({ "content": self.instruction }).to_string();
        let body_content = serde_json::json!({ "content": format!("<|start_header_id|>system<|end_header_id|>\n{}\n<|eot_id|>", self.instruction) }).to_string();

        let body = Bytes::from(body_content);
        let mut headers = HashMap::new();
        headers
            .insert("Content-Type".to_string(), "application/json".to_string());

        match self
            .http_client
            .post(url, Some(&headers), None, Some(&body), None)
            .await
        {
            Ok(http_response) => {
                match http_response.json::<TokenResponse>() {
                    Ok(response) => {
                        // Successfully parsed the token response, updating n_keep
                        self.n_keep = response.tokens.len();
                        log::debug!("n_keep set to {}", self.n_keep);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Failed to parse JSON response: {}", e);
                        Err(format!("Failed to parse JSON response: {}", e)
                            .into())
                    }
                }
            }
            Err(e) => Err(format!("HTTP request failed: {}", e).into()),
        }
    }

    pub async fn message(
        &mut self,
        tx: mpsc::Sender<Bytes>,
        keep_running: Arc<AtomicBool>,
        question: String,
    ) {
        let prompt = self.create_final_prompt(question);

        let data_payload = self.create_payload(prompt);

        log::debug!("Payload created:\n{:?}", data_payload);
        if let Ok(payload) = data_payload {
            send_payload(
                "http://localhost:8080/completion".to_string(),
                self.http_client.clone(),
                tx,
                payload,
                keep_running,
            )
            .await;
        }
    }

    pub fn add_exchange(&mut self, question: String, answer: String) {
        self.exchanges.push((question, answer));
    }

    fn trim_history(&mut self) {
        if (self.exchanges.len() / 2) > self.max_history {
            let excess = self.exchanges.len() - self.max_history * 2;
            self.exchanges.drain(0..excess);
        }
    }

    // fn create_final_prompt(&self, question: String) -> String {
    //let mut prompt = format!("{}\n", self.instruction);
    //for (question, answer) in &self.exchanges {
    //prompt.push_str(&format!(
    //"\n### Human: {}\n### Assistant: {}",
    //question, answer
    //));
    //}
    //prompt.push_str(&format!("\n### Human: {}", question));
    //prompt
    //}

    pub fn create_final_prompt(&mut self, user_question: String) -> String {
        let user_question = user_question.trim();
        let user_question = if user_question.is_empty() {
            "continue".to_string()
        } else {
            if self.user_prompt.is_some() {
                self.user_prompt
                    .as_ref()
                    .unwrap()
                    .replace("{{ USER_QUESTION }}", user_question)
            } else {
                user_question.to_string()
            }
        };

        let mut prompt = String::new();

        // https://github.com/meta-llama/llama3/blob/main/llama/tokenizer.py#L202

        //        <|begin_of_text|><|start_header_id|>system<|end_header_id|>
        //
        //        You are a helpful assistant<|eot_id|><|start_header_id|>user<|end_header_id|>
        //
        //        {prompt}<|eot_id|><|start_header_id|>assistant<|end_header_id|>

        // start with system prompt
        prompt.push_str("<|begin_of_text|>");
        if !self.instruction.is_empty() {
            prompt.push_str(&format!(
                "<|start_header_id|>system<|end_header_id|>\\
                 n{}<|eot_id|>\n",
                self.instruction
            ));
        }

        // add exchange-history
        for (user_msg, model_answer) in &self.exchanges {
            prompt.push_str(&format!(
                "<|start_header_id|>user<|end_header_id|>\n{}\n<|eot_id|>\\
                 n<|start_header_id|>assistant<|end_header_id|>\n{}\\
                 n<|eot_id|>\n",
                user_msg, model_answer
            ));
        }

        // Add the current user question without an assistant's answer
        prompt.push_str(&format!(
            "<|start_header_id|>user<|end_header_id|>\n{}\n<|eot_id|>\n",
            user_question
        ));
        // Add the current user question without an assistant's answer

        // First, check if the last exchange exists and if its second element is empty
        if let Some(last_exchange) = self.exchanges.last() {
            if last_exchange.1.is_empty() {
                // Remove the last exchange because it's unanswered
                self.exchanges.pop();
            }
        }

        // Now, always add the new exchange to the list
        self.exchanges.push((user_question.clone(), "".to_string()));
        eprintln!("exchanges: {:?}", self.exchanges);
        prompt
    }

    pub fn create_payload(
        &self,
        prompt: String,
    ) -> Result<String, serde_json::Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            prompt: &'a str,
            #[serde(flatten)]
            options: &'a ChatOptions,
        }

        let payload = Payload {
            prompt: &prompt,
            options: &self.options,
        };

        serde_json::to_string(&payload)
    }
}

pub async fn process_prompt(
    chat_session: &mut ChatSession,
    question: String,
    keep_running: Arc<AtomicBool>,
) {
    let (tx, rx) = mpsc::channel(32);
    chat_session
        .message(tx, keep_running.clone(), question)
        .await;

    handle_response(rx, keep_running).await;
}

async fn handle_response(
    mut rx: mpsc::Receiver<Bytes>,
    keep_running: Arc<AtomicBool>,
) {
    while keep_running.load(Ordering::Relaxed) {
        while let Some(response) = rx.recv().await {
            let (response_content, is_final) =
                process_prompt_response(&response);
            print!("{}", response_content);
            io::stdout().flush().expect("Failed to flush stdout");

            if is_final {
                break;
            }
        }
    }
}

pub fn process_prompt_response(response: &Bytes) -> (String, bool) {
    match ChatCompletionResponse::extract_content(response) {
        Ok(chat) => (chat.content, chat.stop),
        Err(e) => (format!("Failed to parse JSON: {}", e), true),
    }
}