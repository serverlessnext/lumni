use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use lumni::HttpClient;
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::exchange::ChatExchange;
use super::history::ChatHistory;
use super::prompt::{Prompt, SystemPrompt};
use super::send::http_post;
use super::{
    ChatCompletionResponse, PromptModel, PromptModelTrait, PromptRole,
    ServerTrait, TokenResponse, PERSONAS,
};
use crate::external as lumni;

pub struct ChatSession {
    http_client: HttpClient,
    history: ChatHistory,
    system_prompt: SystemPrompt,
    prompt_template: Option<String>,
    model: Box<dyn PromptModelTrait>,
    server: Box<dyn ServerTrait>,
    assistant: Option<String>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl ChatSession {
    pub fn new(
        server: Box<dyn ServerTrait>,
        model: Option<Box<dyn PromptModelTrait>>,
    ) -> Result<Self, Box<dyn Error>> {
        let model = match model {
            Some(m) => m,
            None => {
                Box::new(PromptModel::default()?) as Box<dyn PromptModelTrait>
            }
        };

        Ok(ChatSession {
            http_client: HttpClient::new(),
            history: ChatHistory::new(),
            system_prompt: SystemPrompt::default(),
            prompt_template: None,
            model,
            server,
            assistant: None,
            cancel_tx: None,
        })
    }

    pub async fn set_system_prompt(
        &mut self,
        instruction: &str,
    ) -> Result<(), Box<dyn Error>> {
        let token_length = if instruction.is_empty() {
            Some(0)
        } else if let Some(endpoint) =
            self.server.get_endpoints().get_tokenizer()
        {
            Some(
                self.tokenize(endpoint, instruction)
                    .await?
                    .get_tokens()
                    .len(),
            )
        } else {
            None
        };
        let system_prompt = instruction.to_string();
        self.system_prompt = SystemPrompt::new(system_prompt, token_length);
        Ok(())
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
                if let Some(instruction) = prompt.system_prompt() {
                    self.set_system_prompt(instruction).await?;
                }
                // Load predefined exchanges from persona if available
                if let Some(exchanges) = prompt.exchanges() {
                    self.history =
                        ChatHistory::new_with_exchanges(exchanges.clone());
                }

                if let Some(prompt_template) = prompt.prompt_template() {
                    self.prompt_template = Some(prompt_template.to_string());
                }
            } else {
                return Err("Selected persona not found in the dataset".into());
            }
        }

        // Send the system prompt to the completion API at the start
        self.server
            .put_system_prompt(&self.system_prompt.get_instruction())
            .await?;
        self.tokenize_and_set_n_keep();

        if self
            .server
            .get_prompt_options()
            .get_context_size()
            .is_none()
        {
            // fetch the context size from the server settings
            let context_size = self.server.get_context_size().await?;
            self.server.set_context_size(context_size);
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        // Stop the chat session by sending a cancel signal
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }

    pub fn reset(&mut self) {
        self.stop();
        self.history.clear();
    }

    pub fn update_last_exchange(&mut self, answer: &str) {
        self.history.update_last_exchange(answer);
    }

    pub async fn finalize_last_exchange(
        &mut self,
    ) -> Result<(), Box<dyn Error>> {
        let mut token_length = None;

        // Extract the last exchange and perform mutable operations within a smaller scope
        if let Some(last_exchange) = self.history.get_last_exchange_mut() {
            // Strip off trailing whitespaces or newlines from the last exchange
            let trimmed_answer = last_exchange.get_answer().trim().to_string();
            last_exchange.set_answer(trimmed_answer);

            if let Some(endpoint) = self.server.get_endpoints().get_tokenizer()
            {
                let temp_vec = vec![&*last_exchange];
                let last_prompt_text =
                    exchanges_to_string(&self.model, temp_vec);
                token_length = Some(
                    self.tokenize(endpoint, &last_prompt_text)
                        .await?
                        .get_tokens()
                        .len(),
                );
            }
        };

        if let Some(token_length) = token_length {
            if let Some(last_exchange) = self.history.get_last_exchange_mut() {
                last_exchange.set_token_length(token_length);
            }
        }

        Ok(())
    }

    pub async fn tokenize(
        &self,
        endpoint: &Url,
        content: &str,
    ) -> Result<TokenResponse, Box<dyn Error>> {
        self.model
            .tokenizer(content, endpoint, &self.http_client)
            .await
            .map_err(|e| {
                eprintln!("Failed to parse JSON response: {}", e);
                format!("Failed to parse JSON documented: {}", e).into()
            })
    }

    pub fn tokenize_and_set_n_keep(&mut self) {
        if let Some(token_length) = self.system_prompt.get_token_length() {
            self.server.set_n_keep(token_length);
        };
    }

    pub async fn message(
        &mut self,
        tx: mpsc::Sender<Bytes>,
        question: String,
    ) -> Result<(), Box<dyn Error>> {
        let max_token_length =
            self.server.get_prompt_options().get_context_size();
        let new_exchange = self.initiate_new_exchange(question).await?;

        let exchanges = self.history.new_prompt(
            new_exchange,
            max_token_length,
            self.system_prompt.get_token_length(),
        );
        let prompt = exchanges_to_string(&self.model, &exchanges);
        let data_payload = self.server.completion_api_payload(prompt);
        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.cancel_tx = Some(cancel_tx);

        log::debug!("Payload created:\n{:?}", data_payload);

        if let Ok(payload) = data_payload {
            http_post(
                self.server.completion_endpoint()?,
                self.http_client.clone(),
                Some(tx),
                payload,
                Some(cancel_rx),
            )
            .await;
        }
        Ok(())
    }

    pub async fn initiate_new_exchange(
        &self,
        user_question: String,
    ) -> Result<ChatExchange, Box<dyn Error>> {
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

        let mut new_exchange = ChatExchange::new(user_question, "".to_string());
        let temp_vec = vec![&new_exchange];
        let last_prompt_text = exchanges_to_string(&self.model, temp_vec);

        if let Some(endpoint) = self.server.get_endpoints().get_tokenizer() {
            let token_response =
                self.tokenize(endpoint, &last_prompt_text).await?;
            new_exchange.set_token_length(token_response.get_tokens().len());
        }
        Ok(new_exchange)
    }
}

pub async fn process_prompt(
    chat_session: &mut ChatSession,
    question: String,
    keep_running: Arc<AtomicBool>,
) {
    let (tx, rx) = mpsc::channel(32);
    let _ = chat_session.message(tx, question).await;
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

fn exchanges_to_string<'a, I>(
    model: &Box<dyn PromptModelTrait>,
    exchanges: I,
) -> String
where
    I: IntoIterator<Item = &'a ChatExchange>,
{
    let mut prompt = String::new();
    for exchange in exchanges {
        prompt.push_str(
            &model
                .fmt_prompt_message(PromptRole::User, exchange.get_question()),
        );
        prompt.push_str(
            &model.fmt_prompt_message(
                PromptRole::Assistant,
                exchange.get_answer(),
            ),
        );
    }
    prompt
}
