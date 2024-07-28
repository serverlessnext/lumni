use std::io::{self, Write};
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::db::{ConversationDatabaseStore, ConversationId};
use super::{
    ColorScheme, CompletionResponse, ModelServer, PromptInstruction,
    ServerManager, TextLine,
};
use crate::api::error::ApplicationError;

pub struct ChatSession {
    server: Option<Box<dyn ServerManager>>,
    prompt_instruction: PromptInstruction,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl ChatSession {
    pub async fn new(
        server_name: Option<&str>,
        prompt_instruction: PromptInstruction,
        db_conn: &ConversationDatabaseStore,
    ) -> Result<Self, ApplicationError> {
        let server = if let Some(name) = server_name {
            let mut server: Box<dyn ServerManager> =
                Box::new(ModelServer::from_str(name)?);
            // try to initialize the server
            let reader = db_conn.get_conversation_reader(
                prompt_instruction.get_conversation_id(),
            );
            match server.setup_and_initialize(&reader).await {
                Ok(_) => (),
                Err(ApplicationError::NotReady(e)) => {
                    // warn only, allow to continue
                    log::warn!("Can't initialize server: {}", e);
                }
                Err(e) => {
                    return Err(e);
                }
            }
            Some(server)
        } else {
            None
        };

        Ok(ChatSession {
            server,
            prompt_instruction,
            cancel_tx: None,
        })
    }

    pub fn server_name(&self) -> Result<String, ApplicationError> {
        self.server
            .as_ref()
            .map(|s| s.server_name().to_string())
            .ok_or_else(|| {
                ApplicationError::NotReady("Server not initialized".to_string())
            })
    }

    pub fn get_conversation_id(&self) -> Option<ConversationId> {
        self.prompt_instruction.get_conversation_id()
    }

    pub fn stop(&mut self) {
        // Stop the chat session by sending a cancel signal
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }

    pub fn reset(&mut self, db: &ConversationDatabaseStore) {
        self.stop();
        _ = self.prompt_instruction.reset_history(db);
    }

    pub fn update_last_exchange(&mut self, answer: &str) {
        self.prompt_instruction.append_last_response(answer);
    }

    pub async fn finalize_last_exchange(
        &mut self,
        db: &ConversationDatabaseStore,
        tokens_predicted: Option<usize>,
    ) -> Result<(), ApplicationError> {
        let last_answer = self.prompt_instruction.get_last_response();
        if let Some(last_answer) = last_answer {
            let trimmed_answer = last_answer.trim();
            _ = self.prompt_instruction.put_last_response(
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
        // First, let's handle all operations that don't require mutable access to server
        let model =
            self.prompt_instruction
                .get_model()
                .cloned()
                .ok_or_else(|| {
                    ApplicationError::NotReady(
                        "Model not available".to_string(),
                    )
                })?;

        let user_question = self.initiate_new_exchange(question).await?;

        // Now, let's get the server and perform operations that require it
        let server = self.server.as_mut().ok_or_else(|| {
            ApplicationError::NotReady("Server not initialized".to_string())
        })?;

        let max_token_length = server.get_max_context_size().await?;

        let messages = self
            .prompt_instruction
            .new_question(&user_question, max_token_length);

        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.cancel_tx = Some(cancel_tx); // channel to cancel

        server
            .completion(&messages, &model, Some(tx), Some(cancel_rx))
            .await?;
        Ok(())
    }

    pub async fn initiate_new_exchange(
        &self,
        user_question: &str,
    ) -> Result<String, ApplicationError> {
        let user_question = user_question.trim();
        Ok(if user_question.is_empty() {
            "continue".to_string()
        } else {
            self.format_user_question(user_question)
        })
    }

    fn format_user_question(&self, user_question: &str) -> String {
        self.get_prompt_template()
            .map(|template| {
                template.replace("{{ USER_QUESTION }}", user_question)
            })
            .unwrap_or_else(|| user_question.to_string())
    }

    fn get_prompt_template(&self) -> Option<String> {
        self.prompt_instruction
            .get_completion_options()
            .get_assistant_options()
            .and_then(|opts| opts.prompt_template.clone())
    }

    pub fn process_response(
        &mut self,
        response: Bytes,
        start_of_stream: bool,
    ) -> Result<Option<CompletionResponse>, ApplicationError> {
        self.server
            .as_mut()
            .ok_or_else(|| {
                ApplicationError::NotReady("Server not initialized".to_string())
            })
            .and_then(|server| {
                Ok(server.process_response(response, start_of_stream))
            })
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

            let response = self.process_response(response, start_of_stream);
            final_received = match response {
                Ok(Some(response)) => {
                    print!("{}", response.get_content());
                    io::stdout().flush().expect("Failed to flush stdout");
                    response.is_final
                }
                Ok(None) => true, // stop if no response
                Err(e) => {
                    log::error!("Error processing response: {}", e);
                    true
                }
            };

            start_of_stream = false;
        }
        Ok(())
    }

    pub fn export_conversation(
        &self,
        color_scheme: &ColorScheme,
    ) -> Vec<TextLine> {
        self.prompt_instruction.export_conversation(color_scheme)
    }
}
