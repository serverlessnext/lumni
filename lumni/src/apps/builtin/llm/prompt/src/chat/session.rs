use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use lumni::HttpClient;
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

use super::prompt::{ChatExchange, Prompt, SystemPrompt};
use super::send::send_payload;
use super::{
    ChatCompletionOptions, ChatCompletionResponse, PromptModel,
    PromptModelTrait, PromptRole, PERSONAS,
};
use crate::apps::builtin::llm::prompt::src::model::TokenResponse;
use crate::external as lumni;

pub struct ChatSession {
    http_client: HttpClient,
    exchanges: Vec<ChatExchange>,
    system_prompt: SystemPrompt,
    prompt_template: Option<String>, // put question in {{ USER_QUESTION }}
    model: Box<dyn PromptModelTrait>,
    assistant: Option<String>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl ChatSession {
    pub fn new(
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
            exchanges: Vec::new(),
            system_prompt: SystemPrompt::default(),
            prompt_template: None,
            model,
            assistant: None,
            cancel_tx: None,
        })
    }

    pub async fn set_system_prompt(
        &mut self,
        instruction: &str,
    ) -> Result<(), Box<dyn Error>> {
        let system_prompt = if instruction.is_empty() {
            self.model.fmt_prompt_system(None)
        } else {
            self.model.fmt_prompt_system(Some(instruction))
        };
        let token_length =
            self.tokenize(&system_prompt).await?.get_tokens().len();

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
                    let system_prompt = if instruction.is_empty() {
                        self.model.fmt_prompt_system(None)
                    } else {
                        self.model.fmt_prompt_system(Some(&instruction))
                    };
                    let token_length =
                        self.tokenize(&system_prompt).await?.get_tokens().len();

                    self.system_prompt =
                        SystemPrompt::new(system_prompt, token_length);
                }

                // Load predefined exchanges from persona if available
                if let Some(exchanges) = prompt.exchanges() {
                    self.exchanges = exchanges.clone();
                }

                if let Some(prompt_template) = prompt.prompt_template() {
                    self.prompt_template = Some(prompt_template.to_string());
                }
            } else {
                return Err("Selected persona not found in the dataset".into());
            }
        }
        self.tokenize_and_set_n_keep();
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
            last_exchange.push_to_answer(answer);
        }
    }

    pub async fn finalize_last_exchange(
        &mut self,
    ) -> Result<(), Box<dyn Error>> {
        // Extract the last exchange and perform mutable operations within a smaller scope
        let token_length = if let Some(last_exchange) =
            self.exchanges.last_mut()
        {
            // Strip off trailing whitespaces or newlines from the last exchange
            let trimmed_answer = last_exchange.get_answer().trim().to_string();
            last_exchange.set_answer(trimmed_answer);

            let temp_vec = vec![&*last_exchange];
            let last_prompt_text = create_prompt_string(&self.model, temp_vec);

            // get the token length
            self.tokenize(&last_prompt_text).await?.get_tokens().len()
        } else {
            // No exchanges to finalize
            return Ok(());
        };

        if let Some(last_exchange) = self.exchanges.last_mut() {
            last_exchange.set_token_length(token_length);
        }
        Ok(())
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

    pub fn tokenize_and_set_n_keep(&mut self) {
        let token_length = self.system_prompt.get_token_length();
        self.model.set_n_keep(token_length);
    }

    pub async fn message(
        &mut self,
        tx: mpsc::Sender<Bytes>,
        question: String,
    ) -> Result<(), Box<dyn Error>> {
        let max_token_length =
            self.model.get_prompt_options().get_context_size();
        let new_exchange = self.initiate_new_exchange(question).await?;
        let prompt = self.create_final_prompt(new_exchange, max_token_length);

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
        let last_prompt_text = create_prompt_string(&self.model, temp_vec);
        new_exchange.set_token_length(
            self.tokenize(&last_prompt_text).await?.get_tokens().len(),
        );
        Ok(new_exchange)
    }

    pub fn create_final_prompt(
        &mut self,
        new_exchange: ChatExchange,
        max_token_length: Option<usize>,
    ) -> String {
        let mut prompt = String::new();

        // start prompt
        prompt.push_str(&self.system_prompt.get_instruction());

        let _tokens_remaining = if let Some(max) = max_token_length {
            let tokens_required = new_exchange.get_token_length().unwrap_or(0)
                + self.system_prompt.get_token_length();
            Some(max.saturating_sub(tokens_required))
        } else {
            None
        };

        // add exchange-history
        // TODO: take into account max_token_length, tokens_remaining
        prompt.push_str(&create_prompt_string(
            &self.model,
            self.exchanges.iter(),
        ));

        // Add the new exchange
        prompt.push_str(
            &self.model.fmt_prompt_message(
                PromptRole::User,
                new_exchange.get_question(),
            ),
        );
        prompt.push_str(&self.model.fmt_prompt_message(
            PromptRole::Assistant,
            new_exchange.get_answer(),
        ));

        // Check if the last exchange exists and if its second element is empty
        if let Some(last_exchange) = self.exchanges.last() {
            if last_exchange.get_answer().is_empty() {
                // Remove the last exchange because it's unanswered
                self.exchanges.pop();
            }
        }

        // Add the current user question without an assistant's answer
        self.exchanges.push(new_exchange);
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
            options: &'a ChatCompletionOptions,
        }

        let payload = Payload {
            prompt: &prompt,
            options: self.model.get_completion_options(),
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

fn create_prompt_string<'a, I>(
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
