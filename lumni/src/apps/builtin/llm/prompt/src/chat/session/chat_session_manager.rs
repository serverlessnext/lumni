use std::collections::HashMap;
use std::sync::Arc;

use lumni::api::error::ApplicationError;
use uuid::Uuid;

use super::db::{ConversationDatabase, ConversationId};
use super::threaded_chat_session::ThreadedChatSession;
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
    pub id: Uuid,
    pub conversation_id: Option<ConversationId>,
    pub server_name: Option<String>,
}

pub struct ChatSessionManager {
    sessions: HashMap<Uuid, ThreadedChatSession>,
    pub active_session_info: SessionInfo,
}

#[allow(dead_code)]
impl ChatSessionManager {
    pub async fn new(
        initial_prompt_instruction: PromptInstruction,
        db_conn: Arc<ConversationDatabase>,
    ) -> Self {
        let session_id = Uuid::new_v4();
        let conversation_id = initial_prompt_instruction.get_conversation_id();
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
        sessions.insert(session_id, initial_session);

        Self {
            sessions,
            active_session_info: SessionInfo {
                id: session_id,
                conversation_id,
                server_name,
            },
        }
    }

    pub fn get_active_session(
        &mut self,
    ) -> Result<&mut ThreadedChatSession, ApplicationError> {
        self.sessions
            .get_mut(&self.active_session_info.id)
            .ok_or_else(|| {
                ApplicationError::Runtime(
                    "Active session not found".to_string(),
                )
            })
    }

    pub fn get_conversation_id_for_active_session(
        &self,
    ) -> Option<ConversationId> {
        self.active_session_info.conversation_id
    }

    pub fn get_active_session_id(&self) -> Uuid {
        self.active_session_info.id
    }

    pub async fn stop_session(
        &mut self,
        id: &Uuid,
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
        db_conn: Arc<ConversationDatabase>,
    ) -> Uuid {
        let session_id = Uuid::new_v4();
        let new_session = ThreadedChatSession::new(prompt_instruction, db_conn);
        self.sessions.insert(session_id, new_session);
        session_id
    }

    pub async fn set_active_session(
        &mut self,
        id: Uuid,
    ) -> Result<(), ApplicationError> {
        if let Some(session) = self.sessions.get(&id) {
            let instruction = session.get_instruction().await?;
            let conversation_id = instruction.get_conversation_id();
            let server_name = instruction
                .get_completion_options()
                .model_server
                .as_ref()
                .map(|s| s.to_string());

            self.active_session_info = SessionInfo {
                id,
                conversation_id,
                server_name,
            };
            Ok(())
        } else {
            Err(ApplicationError::InvalidInput(
                "Session not found".to_string(),
            ))
        }
    }

    pub fn stop_active_chat_session(&mut self) -> Result<(), ApplicationError> {
        if let Some(session) =
            self.sessions.get_mut(&self.active_session_info.id)
        {
            session.stop();
            Ok(())
        } else {
            Err(ApplicationError::Runtime(
                "Active session not found".to_string(),
            ))
        }
    }
}
