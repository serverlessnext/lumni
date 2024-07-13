use std::io::{self, Write};
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::{
    ConversationDatabase, LLMDefinition, PromptInstruction, PromptRole,
    ServerManager,
};
use crate::api::error::ApplicationError;

pub struct ChatSession {
    server: Box<dyn ServerManager>,
    prompt_instruction: PromptInstruction,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl ChatSession {
    pub async fn new(
        mut server: Box<dyn ServerManager>,
        mut prompt_instruction: PromptInstruction,
        model: Option<LLMDefinition>,
    ) -> Result<Self, ApplicationError> {
        // initialize server directly if a model is available
        // otherwise, must be done from the terminal window
        if let Some(model) = model {
            server
                .setup_and_initialize(model, &mut prompt_instruction)
                .await?;
        }

        Ok(ChatSession {
            server,
            prompt_instruction,
            cancel_tx: None,
        })
    }

    pub fn server_name(&self) -> &str {
        self.server.server_name()
    }

    pub async fn select_server(
        &mut self,
        mut server: Box<dyn ServerManager>,
    ) -> Result<(), ApplicationError> {
        log::debug!("switching server: {}", server.server_name());
        self.stop();

        let model = server.get_default_model().await;
        if let Some(model) = model {
            server
                .setup_and_initialize(model, &mut self.prompt_instruction)
                .await?;
        }
        self.server = server;
        Ok(())
    }

    pub fn stop(&mut self) {
        // Stop the chat session by sending a cancel signal
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }

    pub fn reset(&mut self, db: &ConversationDatabase) {
        self.stop();
        _ = self.prompt_instruction.reset_history(db);
    }

    pub fn update_last_exchange(&mut self, answer: &str) {
        self.prompt_instruction.append_last_response(answer);
    }

    pub async fn finalize_last_exchange(
        &mut self,
        db: &ConversationDatabase,
        _tokens_predicted: Option<usize>,
    ) -> Result<(), ApplicationError> {
        let last_answer = self.prompt_instruction.get_last_response();

        if let Some(last_answer) = last_answer {
            let trimmed_answer = last_answer.trim();
            let tokens_predicted = if let Some(response) =
                self.server.tokenizer(trimmed_answer).await?
            {
                Some(response.get_tokens().len())
            } else {
                None
            };
            self.prompt_instruction.put_last_response(
                trimmed_answer,
                tokens_predicted,
                db,
            );
        }
        Ok(())
    }

    pub async fn message(
        &mut self,
        tx: mpsc::Sender<Bytes>,
        question: &str,
    ) -> Result<(), ApplicationError> {
        let max_token_length = self
            .server
            .get_context_size(&mut self.prompt_instruction)
            .await?;
        let (user_question, token_length) =
            self.initiate_new_exchange(question).await?;
        let messages = self.prompt_instruction.new_exchange(
            &user_question,
            token_length,
            max_token_length,
        );

        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.cancel_tx = Some(cancel_tx); // channel to cancel

        self.server
            .completion(
                &messages,
                &self.prompt_instruction,
                Some(tx),
                Some(cancel_rx),
            )
            .await?;
        Ok(())
    }

    pub async fn initiate_new_exchange(
        &self,
        user_question: &str,
    ) -> Result<(String, Option<usize>), ApplicationError> {
        let user_question = user_question.trim();
        let user_question = if user_question.is_empty() {
            "continue".to_string()
        } else {
            if let Some(prompt_template) =
                self.prompt_instruction.get_prompt_template()
            {
                prompt_template.replace("{{ USER_QUESTION }}", user_question)
            } else {
                user_question.to_string()
            }
        };

        let model = self.server.get_selected_model()?;
        let formatter = model.get_formatter();
        let last_prompt_text =
            formatter.fmt_prompt_message(&PromptRole::User, &user_question);
        let token_length = if let Some(token_response) =
            self.server.tokenizer(&last_prompt_text).await?
        {
            Some(token_response.get_tokens().len())
        } else {
            None
        };
        Ok((user_question, token_length))
    }

    pub fn process_response(
        &mut self,
        response: Bytes,
        start_of_stream: bool,
    ) -> (Option<String>, bool, Option<usize>) {
        self.server.process_response(response, start_of_stream)
    }

    // used in non-interactive mode
    pub async fn process_prompt(
        &mut self,
        question: String,
        stop_signal: Arc<Mutex<bool>>,
    ) -> Result<(), ApplicationError> {
        let (tx, rx) = mpsc::channel(32);
        let _ = self.message(tx, &question).await;
        self.handle_response(rx, stop_signal).await?;
        self.stop();
        Ok(())
    }

    async fn handle_response(
        &mut self,
        mut rx: mpsc::Receiver<Bytes>,
        stop_signal: Arc<Mutex<bool>>,
    ) -> Result<(), ApplicationError> {
        let mut final_received = false;
        let mut start_of_stream = true;
        while let Some(response) = rx.recv().await {
            // check if the session must be kept running
            if !*stop_signal.lock().await {
                log::debug!("Received stop signal");
                break;
            }

            if final_received {
                // consume stream until its empty, as server may send additional events
                // (e.g. stats, or logs) after the stop event.
                // for now these are ignored.
                continue;
            }
            let (response_content, is_final, _) =
                self.process_response(response, start_of_stream);
            if let Some(response_content) = response_content {
                print!("{}", response_content);
            }
            io::stdout().flush().expect("Failed to flush stdout");

            start_of_stream = false;
            if is_final {
                final_received = true;
            }
        }
        Ok(())
    }
}
