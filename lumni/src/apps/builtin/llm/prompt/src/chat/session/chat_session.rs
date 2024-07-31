use std::io::{self, Write};
use std::sync::Arc;

use ratatui::style::Style;

use bytes::Bytes;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::db::{ConversationDbHandler, ConversationId};
use super::{
    AppUi, TextWindowTrait,
    ColorScheme, CompletionResponse, ModelServer, PromptInstruction,
    ServerManager, TextLine,
};
use crate::api::error::ApplicationError;

// max number of messages to hold before backpressure is applied
// only applies to interactive mode
const CHANNEL_QUEUE_SIZE: usize = 32;

pub struct ModelServerSession {
    server: Option<Box<dyn ServerManager>>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl ModelServerSession {
    pub fn new() -> Self {
        ModelServerSession {
            server: None,
            cancel_tx: None,
        }
    }

    pub async fn initialize_model_server(
        &mut self,
        prompt_instruction: &PromptInstruction,
        db_handler: &ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        self.server = if let Some(model_server) = prompt_instruction
            .get_completion_options()
            .model_server
            .as_ref()
        {
            let mut server: Box<dyn ServerManager> =
                Box::new(ModelServer::from_str(&model_server.to_string())?);
            // try to initialize the server
            match server.setup_and_initialize(db_handler).await {
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
        Ok(())
    }
}

pub struct ChatSession {
    prompt_instruction: PromptInstruction,
    model_server_session: ModelServerSession,
    response_sender: mpsc::Sender<Bytes>,
    response_receiver: mpsc::Receiver<Bytes>,
}

impl ChatSession {
    pub fn new(prompt_instruction: PromptInstruction) -> Self {
        let (response_sender, response_receiver) =
            mpsc::channel(CHANNEL_QUEUE_SIZE);
        ChatSession {
            prompt_instruction,
            model_server_session: ModelServerSession::new(),
            response_sender,
            response_receiver,
        }
    }

    pub async fn load_instruction(
        &mut self,
        prompt_instruction: PromptInstruction,
    ) -> Result<(), ApplicationError> {
        self.stop_server_session(); // stop a running session (if any)
        self.prompt_instruction = prompt_instruction;
        Ok(())
    }

    pub fn server_name(&self) -> Result<String, ApplicationError> {
        self.model_server_session
            .server
            .as_ref()
            .map(|s| s.server_name().to_string())
            .ok_or_else(|| {
                ApplicationError::NotReady("Server not initialized".to_string())
            })
    }

    pub fn get_conversation_id(&self) -> Option<ConversationId> {
        self.prompt_instruction.get_conversation_id()
    }

    fn stop_server_session(&mut self) {
        self.stop_chat_session();
        self.model_server_session.server = None;
    }

    pub fn stop_chat_session(&mut self) {
        if let Some(cancel_tx) = self.model_server_session.cancel_tx.take() {
            let _ = cancel_tx.send(());
            self.model_server_session.cancel_tx = None;
        }
    }

    pub fn reset(&mut self, db_handler: &mut ConversationDbHandler<'_>) {
        self.stop_server_session();
        _ = self.prompt_instruction.reset_history(db_handler);
    }

    pub fn update_last_exchange(&mut self, answer: &str) {
        self.prompt_instruction.append_last_response(answer);
    }

    pub async fn finalize_last_exchange(
        &mut self,
        db_handler: &ConversationDbHandler<'_>,
        tokens_predicted: Option<usize>,
    ) -> Result<(), ApplicationError> {
        let last_answer = self.prompt_instruction.get_last_response();
        if let Some(last_answer) = last_answer {
            let trimmed_answer = last_answer.trim();
            _ = self.prompt_instruction.put_last_response(
                trimmed_answer,
                tokens_predicted,
                db_handler,
            );
        }
        Ok(())
    }

    pub async fn message(
        &mut self,
        question: &str,
        db_handler: &ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        // Initialize the server if it's not already initialized
        if self.model_server_session.server.is_none() {
            self.model_server_session
                .initialize_model_server(&self.prompt_instruction, db_handler)
                .await
                .map_err(|e| ApplicationError::NotReady(e.to_string()))?;
        }

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
        let server =
            self.model_server_session.server.as_mut().ok_or_else(|| {
                ApplicationError::NotReady("Server not initialized".to_string())
            })?;

        let max_token_length =
            server.get_max_context_size().await.map_err(|e| {
                ApplicationError::ServerConfigurationError(e.to_string())
            })?;

        let messages = self
            .prompt_instruction
            .new_question(&user_question, max_token_length)
            .map_err(|e| ApplicationError::InvalidInput(e.to_string()))?;

        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.model_server_session.cancel_tx = Some(cancel_tx);

        server
            .completion(
                &messages,
                &model,
                Some(self.response_sender.clone()),
                Some(cancel_rx),
            )
            .await
            .map_err(|e| ApplicationError::Runtime(e.to_string()))?;

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
        self.model_server_session
            .server
            .as_mut()
            .ok_or_else(|| {
                ApplicationError::NotReady("Server not initialized".to_string())
            })
            .and_then(|server| {
                Ok(server.process_response(response, start_of_stream))
            })
    }

    pub async fn process_prompt(
        &mut self,
        question: String,
        stop_signal: Arc<Mutex<bool>>,
        db_handler: &ConversationDbHandler<'_>,
    ) -> Result<(), ApplicationError> {
        self.message(&question, db_handler).await?;
        self.handle_response(stop_signal).await?;
        self.stop_server_session();
        Ok(())
    }

    async fn handle_response(
        &mut self,
        stop_signal: Arc<Mutex<bool>>,
    ) -> Result<(), ApplicationError> {
        let mut final_received = false;

        while !final_received {
            if !*stop_signal.lock().await {
                log::debug!("Received stop signal");
                break;
            }

            match self.receive_response().await? {
                Some(response) => {
                    print!("{}", response.get_content());
                    io::stdout().flush().expect("Failed to flush stdout");
                    final_received = response.is_final;

                    if let Some(stats) = &response.stats {
                        if let Some(tokens) = stats.tokens_predicted {
                            log::debug!("Tokens predicted: {}", tokens);
                        }
                    }
                }
                None => {
                    log::debug!("No more responses");
                    break;
                }
            }
        }

        println!(); // Add a newline at the end for better formatting
        Ok(())
    }

    pub fn export_conversation(
        &self,
        color_scheme: &ColorScheme,
    ) -> Vec<TextLine> {
        self.prompt_instruction.export_conversation(color_scheme)
    }
}

impl ChatSession {
    pub async fn receive_and_process_response<'a>(
        &mut self,
        db_handler: &mut ConversationDbHandler<'a>,
        color_scheme: Option<&ColorScheme>,
        ui: Option<&mut AppUi<'a>>,
    ) -> Result<bool, ApplicationError> {
        match self.receive_response().await? {
            Some(response) => {
                self.process_chat_response(response, db_handler, color_scheme, ui).await?;
                Ok(true) // Indicates that a response was processed
            }
            None => {
                log::debug!("No more responses");
                Ok(false) // Indicates that no response was received (channel closed)
            }
        }
    }

    async fn receive_response(
        &mut self,
    ) -> Result<Option<CompletionResponse>, ApplicationError> {
        if let Some(response_bytes) = self.response_receiver.recv().await {
            self.process_response(response_bytes, true)
        } else {
            Ok(None) // Channel closed
        }
    }

    async fn process_chat_response<'a>(
        &mut self,
        response: CompletionResponse,
        db_handler: &mut ConversationDbHandler<'a>,
        color_scheme: Option<&ColorScheme>,
        mut ui: Option<&mut AppUi<'a>>,
    ) -> Result<(), ApplicationError> {
        log::debug!(
            "Received response with length {:?}",
            response.get_content().len()
        );

        let trimmed_response = response.get_content().trim_end().to_string();
        log::debug!("Trimmed response: {:?}", trimmed_response);

        if !trimmed_response.is_empty() {
            self.update_last_exchange(&trimmed_response);
            
            // Update UI if provided
            if let (Some(ui), Some(color_scheme)) = (ui.as_mut(), color_scheme) {
                ui.response
                    .text_append(
                        &trimmed_response,
                        Some(color_scheme.get_secondary_style()),
                    )
                    .map_err(|e| ApplicationError::Runtime(e.to_string()))?;
            }
        }

        if response.is_final {
            self.finalize_chat_response(response, db_handler, color_scheme, ui).await?;
        }

        Ok(())
    }

    async fn finalize_chat_response<'a>(
        &mut self,
        response: CompletionResponse,
        db_handler: &mut ConversationDbHandler<'a>,
        color_scheme: Option<&ColorScheme>,
        mut ui: Option<&mut AppUi<'a>>,
    ) -> Result<(), ApplicationError> {
        let tokens_predicted = response.stats.as_ref().and_then(|s| s.tokens_predicted);

        self.stop_chat_session();

        if let (Some(ui), Some(color_scheme)) = (ui.as_mut(), color_scheme) {
            ui.response
                .text_append("\n", Some(color_scheme.get_secondary_style()))
                .map_err(|e| ApplicationError::Runtime(e.to_string()))?;

            ui.response
                .text_append("\n", Some(Style::reset()))
                .map_err(|e| ApplicationError::Runtime(e.to_string()))?;
        }

        self.finalize_last_exchange(db_handler, tokens_predicted).await?;

        Ok(())
    }
}