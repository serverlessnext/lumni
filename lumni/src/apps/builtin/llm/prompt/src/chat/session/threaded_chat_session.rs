use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::chat_session_manager::ChatEvent;
use super::db::ConversationDbHandler;
use super::{
    CompletionResponse, ModelServer, PromptError, PromptInstruction,
    PromptNotReadyReason, ServerManager,
};
use crate::api::error::ApplicationError;

// max number of messages to hold before backpressure is applied
// only applies to interactive mode
const CHANNEL_QUEUE_SIZE: usize = 32;

pub struct ThreadedChatSession {
    prompt_instruction: Arc<Mutex<PromptInstruction>>,
    inner: Option<Arc<Mutex<ThreadedChatSessionInner>>>,
    command_sender: Option<mpsc::Sender<ThreadedChatSessionCommand>>,
    pub event_receiver: Option<mpsc::Receiver<ChatEvent>>,
}

struct ThreadedChatSessionInner {
    model_server_session: ModelServerSession,
    response_sender: mpsc::Sender<Bytes>,
    response_receiver: mpsc::Receiver<Bytes>,
    event_sender: mpsc::Sender<ChatEvent>,
}

enum ThreadedChatSessionCommand {
    Message(String, oneshot::Sender<Result<(), PromptError>>),
    Stop,
}

impl ThreadedChatSession {
    pub fn new(prompt_instruction: PromptInstruction) -> Self {
        Self {
            prompt_instruction: Arc::new(Mutex::new(prompt_instruction)),
            inner: None,
            command_sender: None,
            event_receiver: None,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.inner.is_some()
    }

    pub async fn initialize(
        &mut self,
        db_handler: ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        if self.is_initialized() {
            return Ok(());
        }
        let (response_sender, response_receiver) =
            mpsc::channel(CHANNEL_QUEUE_SIZE);
        let (event_sender, event_receiver) = mpsc::channel(100);
        let (command_sender, command_receiver) = mpsc::channel(100);

        let inner = Arc::new(Mutex::new(ThreadedChatSessionInner {
            model_server_session: ModelServerSession::new(),
            response_sender,
            response_receiver,
            event_sender,
        }));

        let prompt_instruction_clone = self.prompt_instruction.clone();
        let inner_clone = inner.clone();

        tokio::spawn(async move {
            Self::run(
                inner_clone,
                command_receiver,
                db_handler,
                prompt_instruction_clone,
            )
            .await;
        });

        self.inner = Some(inner);
        self.command_sender = Some(command_sender);
        self.event_receiver = Some(event_receiver);

        Ok(())
    }

    pub async fn get_instruction(
        &self,
    ) -> Result<PromptInstruction, ApplicationError> {
        Ok(self.prompt_instruction.lock().await.clone())
    }

    async fn run(
        inner: Arc<Mutex<ThreadedChatSessionInner>>,
        mut command_receiver: mpsc::Receiver<ThreadedChatSessionCommand>,
        mut db_handler: ConversationDbHandler,
        prompt_instruction: Arc<Mutex<PromptInstruction>>,
    ) {
        let conversation_id = {
            let locked_prompt = prompt_instruction.lock().await;
            locked_prompt.get_conversation_id()
        };
        db_handler.set_conversation_id(conversation_id);

        loop {
            tokio::select! {
                Some(command) = command_receiver.recv() => {
                    let mut locked_inner = inner.lock().await;
                    match command {
                        ThreadedChatSessionCommand::Message(question, response_sender) => {
                            let result = locked_inner.handle_message(&question, &db_handler, &prompt_instruction).await;
                            let _ = response_sender.send(result);
                        }
                        ThreadedChatSessionCommand::Stop => {
                            if let Err(e) = locked_inner.finalize_last_exchange(&db_handler, None, &prompt_instruction).await {
                                log::error!("Error finalizing last exchange: {:?}", e);
                            }
                            break;
                        }
                    }
                }
                result = async {
                    let mut locked_inner = inner.lock().await;
                    locked_inner.process_next_response(&mut db_handler, &prompt_instruction).await
                } => {
                    match result {
                        Ok(true) => continue,
                        Ok(false) => break,
                        Err(e) => {
                            log::error!("Error processing response: {:?}", e);
                            break;
                        }
                    }
                }
            }
        }
    }

    pub async fn message(
        &self,
        question: &str,
    ) -> Result<(), ApplicationError> {
        let sender = self.command_sender.as_ref().ok_or_else(|| {
            ApplicationError::NotReady(
                "Chat session not initialized".to_string(),
            )
        })?;

        let (response_sender, response_receiver) = oneshot::channel();
        sender
            .send(ThreadedChatSessionCommand::Message(
                question.to_string(),
                response_sender,
            ))
            .await
            .map_err(|e| {
                ApplicationError::Runtime(format!(
                    "Failed to send message: {}",
                    e
                ))
            })?;

        response_receiver
            .await
            .map_err(|e| {
                ApplicationError::Runtime(format!(
                    "Failed to receive message response: {}",
                    e
                ))
            })?
            .map_err(|e| {
                ApplicationError::Runtime(format!("Prompt error: {}", e))
            })
    }

    pub fn stop(&self) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.try_send(ThreadedChatSessionCommand::Stop);
        }
    }
}

impl ThreadedChatSessionInner {
    async fn handle_message(
        &mut self,
        question: &str,
        db_handler: &ConversationDbHandler,
        prompt_instruction: &Arc<Mutex<PromptInstruction>>,
    ) -> Result<(), PromptError> {
        let mut locked_prompt = prompt_instruction.lock().await;

        if self.model_server_session.server.is_none() {
            self.model_server_session
                .initialize_model_server(&locked_prompt, db_handler)
                .await
                .map_err(|e| {
                    PromptError::NotReady(PromptNotReadyReason::Other(
                        e.to_string(),
                    ))
                })?;
        }

        let model = locked_prompt.get_model().cloned().ok_or_else(|| {
            PromptError::NotReady(PromptNotReadyReason::NoModelSelected)
        })?;

        let user_question =
            self.initiate_new_exchange(question, &locked_prompt).await?;
        let server =
            self.model_server_session.server.as_mut().ok_or_else(|| {
                PromptError::NotReady(PromptNotReadyReason::Other(
                    "Server not initialized".to_string(),
                ))
            })?;

        let max_token_length =
            server.get_max_context_size().await.map_err(|e| {
                PromptError::ServerConfigurationError(e.to_string())
            })?;

        let messages =
            locked_prompt.new_question(&user_question, max_token_length)?;

        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.model_server_session.cancel_tx = Some(cancel_tx);

        server
            .completion(
                &messages,
                &model,
                Some(self.response_sender.clone()),
                Some(cancel_rx),
                Some(self.event_sender.clone()),
            )
            .await
            .map_err(|e| PromptError::Runtime(e.to_string()))?;
        Ok(())
    }

    async fn initiate_new_exchange(
        &self,
        user_question: &str,
        prompt_instruction: &PromptInstruction,
    ) -> Result<String, PromptError> {
        let user_question = user_question.trim();
        Ok(if user_question.is_empty() {
            "continue".to_string()
        } else {
            self.format_user_question(user_question, prompt_instruction)
        })
    }

    fn format_user_question(
        &self,
        user_question: &str,
        prompt_instruction: &PromptInstruction,
    ) -> String {
        self.get_prompt_template(prompt_instruction)
            .map(|template| {
                template.replace("{{ USER_QUESTION }}", user_question)
            })
            .unwrap_or_else(|| user_question.to_string())
    }

    fn get_prompt_template(
        &self,
        prompt_instruction: &PromptInstruction,
    ) -> Option<String> {
        prompt_instruction
            .get_completion_options()
            .get_assistant_options()
            .and_then(|opts| opts.prompt_template.clone())
    }

    fn stop_chat_session(&mut self) {
        if let Some(cancel_tx) = self.model_server_session.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }

    async fn update_last_exchange(
        &self,
        answer: &str,
        prompt_instruction: &Arc<Mutex<PromptInstruction>>,
    ) {
        let mut locked_prompt = prompt_instruction.lock().await;
        locked_prompt.append_last_response(answer);
    }

    async fn finalize_last_exchange(
        &mut self,
        db_handler: &ConversationDbHandler,
        tokens_predicted: Option<usize>,
        prompt_instruction: &Arc<Mutex<PromptInstruction>>,
    ) -> Result<(), ApplicationError> {
        let mut locked_prompt = prompt_instruction.lock().await;
        if let Some(last_answer) = locked_prompt.get_last_response() {
            let trimmed_answer = last_answer.trim();
            _ = locked_prompt
                .put_last_response(trimmed_answer, tokens_predicted, db_handler)
                .await;
        }
        Ok(())
    }

    fn process_response(
        &mut self,
        response: Bytes,
        start_of_stream: bool,
    ) -> Result<Option<CompletionResponse>, ApplicationError> {
        // if bytes contain an error, do not continue processing but return the error

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

    async fn receive_response(
        &mut self,
    ) -> Result<Option<CompletionResponse>, ApplicationError> {
        if let Some(response_bytes) = self.response_receiver.recv().await {
            self.process_response(response_bytes, true)
        } else {
            Ok(None) // Channel closed
        }
    }

    async fn process_chat_response(
        &mut self,
        response: CompletionResponse,
        db_handler: &mut ConversationDbHandler,
        prompt_instruction: &Arc<Mutex<PromptInstruction>>,
    ) -> Result<(), ApplicationError> {
        let content = response.get_content().trim_end().to_string();
        if !content.is_empty() {
            self.update_last_exchange(&content, prompt_instruction)
                .await;
            self.event_sender
                .send(ChatEvent::ResponseUpdate(content))
                .await
                .ok();
        }

        if response.is_final {
            self.finalize_chat_response(
                response,
                db_handler,
                prompt_instruction,
            )
            .await?;
        }

        Ok(())
    }

    async fn finalize_chat_response(
        &mut self,
        response: CompletionResponse,
        db_handler: &mut ConversationDbHandler,
        prompt_instruction: &Arc<Mutex<PromptInstruction>>,
    ) -> Result<(), ApplicationError> {
        let tokens_predicted =
            response.stats.as_ref().and_then(|s| s.tokens_predicted);

        self.stop_chat_session();
        self.finalize_last_exchange(
            db_handler,
            tokens_predicted,
            prompt_instruction,
        )
        .await?;

        self.event_sender.send(ChatEvent::FinalResponse).await.ok();

        Ok(())
    }

    async fn process_next_response(
        &mut self,
        db_handler: &mut ConversationDbHandler,
        prompt_instruction: &Arc<Mutex<PromptInstruction>>,
    ) -> Result<bool, ApplicationError> {
        match self.receive_response().await {
            Ok(Some(response)) => {
                self.process_chat_response(
                    response,
                    db_handler,
                    prompt_instruction,
                )
                .await?;
                Ok(true) // Indicates that a response was processed
            }
            Ok(None) => {
                log::info!("Chat session channel closed");
                Ok(false)
            }
            Err(e) => {
                self.event_sender
                    .send(ChatEvent::Error(e.to_string()))
                    .await
                    .ok();
                Err(e)
            }
        }
    }
}

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
        db_handler: &ConversationDbHandler,
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
