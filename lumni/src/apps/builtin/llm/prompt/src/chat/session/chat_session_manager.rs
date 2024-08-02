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

pub struct ChatSessionManager {
    sessions: HashMap<ConversationId, ThreadedChatSession>,
    active_session_id: ConversationId,
    db_conn: Arc<ConversationDatabase>,
}

impl ChatSessionManager {
    pub async fn new(
        initial_prompt_instruction: PromptInstruction,
        db_conn: Arc<ConversationDatabase>,
    ) -> Self {
        let id = initial_prompt_instruction.get_conversation_id().unwrap();
        let initial_session = ThreadedChatSession::new(
            initial_prompt_instruction,
            db_conn.clone(),
        );

        let mut sessions = HashMap::new();
        sessions.insert(id.clone(), initial_session);
        Self {
            sessions,
            active_session_id: id,
            db_conn,
        }
    }

    pub fn get_active_session(&mut self) -> &mut ThreadedChatSession {
        self.sessions.get_mut(&self.active_session_id).unwrap()
    }

    pub fn get_active_session_id(&self) -> &ConversationId {
        &self.active_session_id
    }

    pub async fn process_events(&mut self) -> Vec<ChatEvent> {
        let mut events = Vec::new();
        if let Some(session) = self.sessions.get_mut(&self.active_session_id) {
            let mut receiver = session.subscribe();
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
            }
        }
        events
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

    pub fn set_active_session(
        &mut self,
        id: ConversationId,
    ) -> Result<(), ApplicationError> {
        if self.sessions.contains_key(&id) {
            self.active_session_id = id;
            Ok(())
        } else {
            Err(ApplicationError::InvalidInput(
                "Session not found".to_string(),
            ))
        }
    }

    pub fn stop_active_chat_session(&mut self) {
        if let Some(session) = self.sessions.get_mut(&self.active_session_id) {
            session.stop();
        }
    }
}
