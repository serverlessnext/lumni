use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use lumni::HttpClient;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use super::{Models, PromptModel};
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
    cancel_tx: Option<oneshot::Sender<()>>,
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
            cancel_tx: None,
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

        let model = Models::from_str(&self.model);
        let prompt_start = if self.instruction.is_empty() {
            model.fmt_prompt_start(None)
        } else {
            model.fmt_prompt_start(Some(&self.instruction))
        };
        let body_content = serde_json::json!({ "content": prompt_start }).to_string();
        self.tokenize_and_set_n_keep(body_content).await?;
        Ok(())
    }

    pub fn reset(&mut self) {
        // Stop the chat session by sending a cancel signal
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        self.exchanges.clear();
    }

    pub fn update_last_exchange(&mut self, answer: &str) {
        if let Some(last_exchange) = self.exchanges.last_mut() {
            last_exchange.1.push_str(answer);
        }
    }

    pub async fn tokenize_and_set_n_keep(
        &mut self,
        body_content: String,
    ) -> Result<(), Box<dyn Error>> {
        let url = "http://localhost:8080/tokenize";
        let body = Bytes::from(body_content);
        let mut headers = HashMap::new();
        headers
            .insert("Content-Type".to_string(), "application/json".to_string());

        match self
            .http_client
            .post(url, Some(&headers), None, Some(&body), None, None)
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

    pub async fn message(&mut self, tx: mpsc::Sender<Bytes>, question: String) {
        let prompt = self.create_final_prompt(question);

        let data_payload = self.create_payload(prompt);

        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.cancel_tx = Some(cancel_tx);

        log::debug!("Payload created:\n{:?}", data_payload);
        if let Ok(payload) = data_payload {
            send_payload(
                "http://localhost:8080/completion".to_string(),
                self.http_client.clone(),
                tx,
                payload,
                Some(cancel_rx),
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

        // start prompt
        let model = Models::from_str(&self.model);
        if self.instruction.is_empty() {
            prompt.push_str(&model.fmt_prompt_start(None));
        } else {
            prompt.push_str(&model.fmt_prompt_start(Some(&self.instruction)));
        }

        let role_name_user = model.role_name_user();
        let role_name_assistant = model.role_name_assistant();

        // add exchange-history
        for (user_msg, model_answer) in &self.exchanges {
            prompt.push_str(&model.fmt_prompt_message(&role_name_user, user_msg));
            prompt.push_str(&model.fmt_prompt_message(&role_name_assistant, model_answer));
        }

        // Add the current user question without an assistant's answer
        prompt.push_str(&model.fmt_prompt_message(&role_name_user, &user_question));
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
    chat_session.message(tx, question).await;
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
