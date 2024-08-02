use std::collections::HashMap;
use std::sync::Arc;

use lumni::api::error::ApplicationError;

use super::chat_session::ThreadedChatSession;
use super::db::{ConversationDatabase, ConversationId};
use super::PromptInstruction;
pub use crate::external as lumni;

// add clone
#[derive(Clone)]
pub enum ChatEvent {
    ResponseUpdate(String),
    FinalResponse,
    Error(String),
}

pub struct SessionInfo {
    pub id: ConversationId,
    pub server_name: Option<String>,
}

pub struct ChatSessionManager {
    sessions: HashMap<ConversationId, ThreadedChatSession>,
    db_conn: Arc<ConversationDatabase>,
    pub active_session_info: SessionInfo, // cache frequently accessed session info
}

impl ChatSessionManager {
    pub async fn new(
        initial_prompt_instruction: PromptInstruction,
        db_conn: Arc<ConversationDatabase>,
    ) -> Self {
        let id = initial_prompt_instruction.get_conversation_id().unwrap();

        let server_name = initial_prompt_instruction
            .get_completion_options()
            .model_server
            .as_ref()
            .map(|s| s.to_string());

        let initial_session = ThreadedChatSession::new(
            initial_prompt_instruction,
            db_conn.clone(),
        );

        let mut sessions = HashMap::new();
        sessions.insert(id.clone(), initial_session);
        Self {
            sessions,
            db_conn,
            active_session_info: SessionInfo { id, server_name },
        }
    }

    pub fn get_active_session(&mut self) -> &mut ThreadedChatSession {
        self.sessions.get_mut(&self.active_session_info.id).unwrap()
    }

    pub fn get_active_session_id(&self) -> &ConversationId {
        &self.active_session_info.id
    }

    pub async fn stop_session(
        &mut self,
        id: &ConversationId,
    ) -> Result<(), ApplicationError> {
        if let Some(session) = self.sessions.remove(id) {
            session.stop();
            Ok(())
        } else {
            Err(ApplicationError::InvalidInput(
                "Session not found".to_string(),
            ))
        }
    }

    pub async fn stop_all_sessions(&mut self) {
        for (_, session) in self.sessions.drain() {
            session.stop();
        }
    }

    pub async fn create_session(
        &mut self,
        prompt_instruction: PromptInstruction,
    ) -> Result<ConversationId, ApplicationError> {
        let id = prompt_instruction.get_conversation_id().ok_or_else(|| {
            ApplicationError::Runtime(
                "Failed to get conversation ID".to_string(),
            )
        })?;
        let new_session =
            ThreadedChatSession::new(prompt_instruction, self.db_conn.clone());
        self.sessions.insert(id.clone(), new_session);
        Ok(id)
    }

    pub async fn set_active_session(
        &mut self,
        id: ConversationId,
    ) -> Result<(), ApplicationError> {
        if self.sessions.contains_key(&id) {
            self.active_session_info.id = id;
            self.active_session_info.server_name = self
                .sessions
                .get(&id)
                .unwrap()
                .get_instruction()
                .await?
                .get_completion_options()
                .model_server
                .as_ref()
                .map(|s| s.to_string());
            Ok(())
        } else {
            Err(ApplicationError::InvalidInput(
                "Session not found".to_string(),
            ))
        }
    }

    pub fn stop_active_chat_session(&mut self) {
        if let Some(session) =
            self.sessions.get_mut(&self.active_session_info.id)
        {
            session.stop();
        }
    }
}
