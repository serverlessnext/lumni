use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use lumni::HttpClient;
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

use super::prompt::Prompt;
use super::send::send_payload;
use super::{
    ChatCompletionResponse, ChatOptions, Models, PromptModel, PERSONAS,
};
use crate::apps::builtin::llm::prompt::src::model::TokenResponse;
use crate::external as lumni;

pub struct ChatSession {
    http_client: HttpClient,
    exchanges: Vec<(String, String)>, // (question, answer)
    instruction: String,              // system
    prompt_template: Option<String>,  // put question in {{ USER_QUESTION }}
    model: Box<dyn PromptModel>,
    assistant: Option<String>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl ChatSession {
    pub fn new(
        model: Option<Box<dyn PromptModel>>,
    ) -> Result<Self, Box<dyn Error>> {
        let model = match model {
            Some(m) => m,
            None => Box::new(Models::default()?) as Box<dyn PromptModel>,
        };

        Ok(ChatSession {
            http_client: HttpClient::new(),
            exchanges: Vec::new(),
            instruction: "".to_string(),
            prompt_template: None,
            model,
            assistant: None,
            cancel_tx: None,
        })
    }

    pub fn set_instruction(&mut self, instruction: String) -> &mut Self {
        self.instruction = instruction;
        self
    }

    pub fn set_assistant(&mut self, assistant: Option<String>) -> &mut Self {
        self.assistant = assistant;
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

                if let Some(prompt_template) = prompt.prompt_template() {
                    self.prompt_template = Some(prompt_template.to_string());
                }
            } else {
                return Err("Selected persona not found in the dataset".into());
            }
        }

        let prompt_start = if self.instruction.is_empty() {
            self.model.fmt_prompt_system(None)
        } else {
            self.model.fmt_prompt_system(Some(&self.instruction))
        };

        self.tokenize_and_set_n_keep(&prompt_start).await?;
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

    pub fn trim_last_exchange(&mut self) {
        // strip of trailing whitespaces or newlines from the last exchange
        if let Some(last_exchange) = self.exchanges.last_mut() {
            last_exchange.1 = last_exchange.1.trim().to_string();
        }
    }

    pub async fn tokenize(
        &self,
        content: &str,
    ) -> Result<TokenResponse, Box<dyn Error>> {
        self.model
            .tokenizer(content, &self.http_client)
            .await
            .map_err(|e| {
                eprintln!("Failed to parse JSON response: {}", e);
                format!("Failed to parse JSON documented: {}", e).into()
            })
    }

    pub async fn tokenize_and_set_n_keep(
        &mut self,
        prompt_start: &str,
    ) -> Result<(), Box<dyn Error>> {
        let token_length =
            self.tokenize(prompt_start).await?.get_tokens().len();
        self.model.set_n_keep(token_length);
        Ok(())
    }

    pub async fn message(
        &mut self,
        tx: mpsc::Sender<Bytes>,
        question: String,
    ) -> Result<(), Box<dyn Error>> {
        let prompt = self.create_final_prompt(question);
        let token_length = self.tokenize(&prompt).await?.get_tokens().len();
        eprintln!("Token length: {}", token_length);
        // TODO: token length should not exceed the maximum token

        let data_payload = self.create_payload(prompt);

        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.cancel_tx = Some(cancel_tx);

        log::debug!("Payload created:\n{:?}", data_payload);

        if let Ok(payload) = data_payload {
            send_payload(
                self.model.get_completion_endpoint().to_string(),
                self.http_client.clone(),
                tx,
                payload,
                Some(cancel_rx),
            )
            .await;
        }
        Ok(())
    }

    //    fn trim_history(&mut self) {
    //        if (self.exchanges.len() / 2) > self.max_history {
    //            let excess = self.exchanges.len() - self.max_history * 2;
    //            self.exchanges.drain(0..excess);
    //        }
    //    }

    pub fn create_final_prompt(&mut self, user_question: String) -> String {
        let user_question = user_question.trim();
        let user_question = if user_question.is_empty() {
            "continue".to_string()
        } else {
            if self.prompt_template.is_some() {
                self.prompt_template
                    .as_ref()
                    .unwrap()
                    .replace("{{ USER_QUESTION }}", user_question)
            } else {
                user_question.to_string()
            }
        };

        let mut prompt = String::new();

        // start prompt
        if self.instruction.is_empty() {
            prompt.push_str(&self.model.fmt_prompt_system(None));
        } else {
            prompt.push_str(
                &self.model.fmt_prompt_system(Some(&self.instruction)),
            );
        }

        let role_name_user = self.model.role_name_user();
        let role_name_assistant = self.model.role_name_assistant();

        // add exchange-history
        for (user_msg, model_answer) in &self.exchanges {
            prompt.push_str(
                &self.model.fmt_prompt_message(&role_name_user, user_msg),
            );
            prompt.push_str(
                &self
                    .model
                    .fmt_prompt_message(&role_name_assistant, model_answer),
            );
        }

        // Add the current user question with an empty assistant's answer
        prompt.push_str(
            &self
                .model
                .fmt_prompt_message(&role_name_user, &user_question),
        );
        prompt
            .push_str(&self.model.fmt_prompt_message(&role_name_assistant, ""));
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
            options: self.model.get_chat_options(),
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
