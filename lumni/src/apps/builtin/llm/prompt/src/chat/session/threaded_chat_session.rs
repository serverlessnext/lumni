use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};

use super::chat_session_manager::ChatEvent;
use super::db::{ConversationDatabase, ConversationDbHandler};
use super::{
    CompletionResponse, ModelServer, PromptError, PromptInstruction,
    PromptNotReadyReason, ServerManager,
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

pub struct ThreadedChatSession {
    command_sender: mpsc::Sender<ThreadedChatSessionCommand>,
    event_receiver: broadcast::Receiver<ChatEvent>,
}

struct ThreadedChatSessionInner {
    prompt_instruction: PromptInstruction,
    model_server_session: ModelServerSession,
    response_sender: mpsc::Sender<Bytes>,
    response_receiver: mpsc::Receiver<Bytes>,
    event_sender: broadcast::Sender<ChatEvent>,
}

#[derive(Debug)]
enum ThreadedChatSessionCommand {
    Message(String, oneshot::Sender<Result<(), PromptError>>),
    LoadInstruction(PromptInstruction),
    GetInstruction(
        oneshot::Sender<Result<PromptInstruction, ApplicationError>>,
    ),
    Stop,
}

impl ThreadedChatSession {
    pub fn new(
        prompt_instruction: PromptInstruction,
        db_conn: Arc<ConversationDatabase>,
    ) -> Self {
        let (response_sender, response_receiver) =
            mpsc::channel(CHANNEL_QUEUE_SIZE);
        let (event_sender, event_receiver) = broadcast::channel(100);
        let (command_sender, command_receiver) = mpsc::channel(100);

        let inner = Arc::new(Mutex::new(ThreadedChatSessionInner {
            prompt_instruction,
            model_server_session: ModelServerSession::new(),
            response_sender,
            response_receiver,
            event_sender,
        }));

        let inner_clone = inner.clone();
        tokio::spawn(async move {
            Self::run(inner_clone, command_receiver, db_conn).await;
        });

        Self {
            command_sender,
            event_receiver,
        }
    }

    async fn run(
        inner: Arc<Mutex<ThreadedChatSessionInner>>,
        mut command_receiver: mpsc::Receiver<ThreadedChatSessionCommand>,
        db_conn: Arc<ConversationDatabase>,
    ) {
        let conversation_id = {
            let locked_inner = inner.lock().await;
            locked_inner.prompt_instruction.get_conversation_id()
        };
        let mut db_handler = db_conn.get_conversation_handler(conversation_id);

        loop {
            tokio::select! {
                Some(command) = command_receiver.recv() => {
                    let mut locked_inner = inner.lock().await;
                    match command {
                        ThreadedChatSessionCommand::Message(question, response_sender) => {
                            let result = locked_inner.handle_message(&question, &db_handler).await;
                            let _ = response_sender.send(result);
                        }
                        ThreadedChatSessionCommand::LoadInstruction(prompt_instruction) => {
                            if let Err(e) = locked_inner.load_instruction(prompt_instruction).await {
                                log::error!("Error loading instruction: {:?}", e);
                            }
                        }
                        ThreadedChatSessionCommand::GetInstruction(response_sender) => {
                            let result = locked_inner.prompt_instruction.clone();
                            if let Err(e) = response_sender.send(Ok(result)) {
                                log::error!("Failed to send instruction response: {:?}", e);
                            }
                        }
                        ThreadedChatSessionCommand::Stop => {
                            if let Err(e) = locked_inner.finalize_last_exchange(&db_handler, None).await {
                                log::error!("Error finalizing last exchange: {:?}", e);
                            }
                            break;
                        }
                    }
                }
                result = async {
                    let mut locked_inner = inner.lock().await;
                    locked_inner.process_next_response(&mut db_handler).await
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

    pub async fn message(&self, question: &str) -> Result<(), PromptError> {
        let (response_sender, response_receiver) = oneshot::channel();

        self.command_sender
            .send(ThreadedChatSessionCommand::Message(
                question.to_string(),
                response_sender,
            ))
            .await
            .map_err(|e| {
                PromptError::Runtime(format!("Failed to send message: {}", e))
            })?;

        response_receiver.await.map_err(|e| {
            PromptError::Runtime(format!(
                "Failed to receive message response: {}",
                e
            ))
        })?
    }

    pub fn stop(&self) {
        let _ = self
            .command_sender
            .try_send(ThreadedChatSessionCommand::Stop);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ChatEvent> {
        self.event_receiver.resubscribe()
    }

    pub async fn load_instruction(
        &self,
        prompt_instruction: PromptInstruction,
    ) -> Result<(), ApplicationError> {
        self.command_sender
            .send(ThreadedChatSessionCommand::LoadInstruction(
                prompt_instruction,
            ))
            .await
            .map_err(|e| {
                ApplicationError::Runtime(format!(
                    "Failed to send load instruction command: {}",
                    e
                ))
            })
    }

    pub async fn get_instruction(
        &self,
    ) -> Result<PromptInstruction, ApplicationError> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.command_sender
            .send(ThreadedChatSessionCommand::GetInstruction(response_sender))
            .await
            .map_err(|e| {
                ApplicationError::Runtime(format!(
                    "Failed to send get instruction command: {}",
                    e
                ))
            })?;

        response_receiver.await.map_err(|e| {
            ApplicationError::Runtime(format!(
                "Failed to receive instruction response: {}",
                e
            ))
        })?
    }
}

impl ThreadedChatSessionInner {
    async fn handle_message(
        &mut self,
        question: &str,
        db_handler: &ConversationDbHandler,
    ) -> Result<(), PromptError> {
        if self.model_server_session.server.is_none() {
            self.model_server_session
                .initialize_model_server(&self.prompt_instruction, db_handler)
                .await
                .map_err(|e| {
                    PromptError::NotReady(PromptNotReadyReason::Other(
                        e.to_string(),
                    ))
                })?;
        }

        let model =
            self.prompt_instruction
                .get_model()
                .cloned()
                .ok_or_else(|| {
                    PromptError::NotReady(PromptNotReadyReason::NoModelSelected)
                })?;

        let user_question = self.initiate_new_exchange(question).await?;
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

        let messages = self
            .prompt_instruction
            .new_question(&user_question, max_token_length)?;

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
            .map_err(|e| PromptError::Runtime(e.to_string()))?;
        Ok(())
    }

    async fn load_instruction(
        &mut self,
        prompt_instruction: PromptInstruction,
    ) -> Result<(), ApplicationError> {
        self.stop_server_session();
        self.prompt_instruction = prompt_instruction;
        Ok(())
    }

    async fn initiate_new_exchange(
        &self,
        user_question: &str,
    ) -> Result<String, PromptError> {
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

    fn stop_server_session(&mut self) {
        self.stop_chat_session();
        self.model_server_session.server = None;
    }

    fn stop_chat_session(&mut self) {
        if let Some(cancel_tx) = self.model_server_session.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }

    fn update_last_exchange(&mut self, answer: &str) {
        self.prompt_instruction.append_last_response(answer);
    }

    async fn finalize_last_exchange(
        &mut self,
        db_handler: &ConversationDbHandler,
        tokens_predicted: Option<usize>,
    ) -> Result<(), ApplicationError> {
        if let Some(last_answer) = self.prompt_instruction.get_last_response() {
            let trimmed_answer = last_answer.trim();
            _ = self
                .prompt_instruction
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
    ) -> Result<(), ApplicationError> {
        let content = response.get_content().trim_end().to_string();
        if !content.is_empty() {
            self.update_last_exchange(&content);
            self.event_sender
                .send(ChatEvent::ResponseUpdate(content))
                .ok();
        }

        if response.is_final {
            self.finalize_chat_response(response, db_handler).await?;
        }

        Ok(())
    }

    async fn finalize_chat_response(
        &mut self,
        response: CompletionResponse,
        db_handler: &mut ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        let tokens_predicted =
            response.stats.as_ref().and_then(|s| s.tokens_predicted);

        self.stop_chat_session();
        self.finalize_last_exchange(db_handler, tokens_predicted)
            .await?;

        self.event_sender.send(ChatEvent::FinalResponse).ok();

        Ok(())
    }

    async fn process_next_response(
        &mut self,
        db_handler: &mut ConversationDbHandler,
    ) -> Result<bool, ApplicationError> {
        match self.receive_response().await {
            Ok(Some(response)) => {
                self.process_chat_response(response, db_handler).await?;
                Ok(true) // Indicates that a response was processed
            }
            Ok(None) => {
                log::info!("Chat session channel closed");
                Ok(false)
            }
            Err(e) => {
                self.event_sender.send(ChatEvent::Error(e.to_string())).ok();
                Err(e)
            }
        }
    }
}
