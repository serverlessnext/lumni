use std::collections::HashMap;

use lumni::api::error::ApplicationError;

use super::db::ConversationId;
use super::ChatSession;
pub use crate::external as lumni;

pub struct ChatSessionManager {
    sessions: HashMap<ConversationId, ChatSession>,
    active_session_id: ConversationId,
}

impl ChatSessionManager {
    pub fn new(initial_session: ChatSession) -> Self {
        let id = initial_session.get_conversation_id().unwrap();
        let mut sessions = HashMap::new();
        sessions.insert(id.clone(), initial_session);
        Self {
            sessions,
            active_session_id: id,
        }
    }

    pub fn get_active_session(&mut self) -> &mut ChatSession {
        self.sessions.get_mut(&self.active_session_id).unwrap()
    }
}