use std::collections::HashMap;
use std::sync::Arc;

use lumni::api::error::ApplicationError;

use super::db::{
    Conversation, ConversationDatabase, ConversationDbHandler, ConversationId,
};
use super::threaded_chat_session::ThreadedChatSession;
use super::PromptInstruction;
pub use crate::external as lumni;

#[derive(Debug, Clone)]
pub enum ChatEvent {
    ResponseUpdate(String),
    FinalResponse,
    Error(String),
}

pub struct ChatSessionManager {
    db_handler: ConversationDbHandler,
    sessions: HashMap<ConversationId, ThreadedChatSession>,
    pub current_conversation: Option<Conversation>,
}

#[allow(dead_code)]
impl ChatSessionManager {
    pub async fn new(
        initial_prompt_instruction: Option<PromptInstruction>,
        db_conn: Arc<ConversationDatabase>,
    ) -> Self {
        let mut sessions = HashMap::new();
        let db_handler = db_conn.get_conversation_handler(None);
        let current_conversation = if let Some(prompt_instruction) =
            initial_prompt_instruction
        {
            let conversation_id = prompt_instruction.get_conversation_id();

            // TODO: remove expect. Validate there is always a conversation, if not fix
            let conversation = db_handler
                .fetch_conversation(conversation_id)
                .await
                .ok()
                .flatten()
                .expect("Conversation not found");

            let initial_session = ThreadedChatSession::new(prompt_instruction);
            sessions.insert(conversation_id, initial_session);

            Some(conversation)
        } else {
            None
        };

        Self {
            db_handler,
            sessions,
            current_conversation,
        }
    }

    pub async fn load_conversation(
        &mut self,
        conversation: Conversation,
    ) -> Result<(), ApplicationError> {
        self.db_handler.set_conversation_id(conversation.id);

        if self.sessions.contains_key(&conversation.id) {
            // conversation already loaded - set as current
            self.current_conversation = Some(conversation);
            return Ok(());
        }

        // Load as a new session
        let prompt_instruction =
            PromptInstruction::from_reader(&self.db_handler).await?;
        let new_session = ThreadedChatSession::new(prompt_instruction);
        self.sessions.insert(conversation.id, new_session);
        self.current_conversation = Some(conversation);
        Ok(())
    }

    pub async fn is_current_session_active(
        &self,
    ) -> Result<bool, ApplicationError> {
        if let Some(ref current_conversation) = self.current_conversation {
            if let Some(session) = self.sessions.get(&current_conversation.id) {
                Ok(session.is_initialized())
            } else {
                Err(ApplicationError::NotFound(
                    "Active session not found".to_string(),
                ))
            }
        } else {
            Ok(false)
        }
    }

    pub async fn get_current_session(
        &self,
    ) -> Result<Option<&ThreadedChatSession>, ApplicationError> {
        if let Some(ref current_conversation) = self.current_conversation {
            if let Some(session) = self.sessions.get(&current_conversation.id) {
                Ok(Some(session))
            } else {
                Err(ApplicationError::Runtime(
                    "Active session not found".to_string(),
                ))
            }
        } else {
            Ok(None)
        }
    }

    pub async fn get_current_session_mut(
        &mut self,
    ) -> Result<Option<&mut ThreadedChatSession>, ApplicationError> {
        if let Some(ref current_conversation) = self.current_conversation {
            if let Some(session) =
                self.sessions.get_mut(&current_conversation.id)
            {
                if !session.is_initialized() {
                    session.initialize(self.db_handler.clone()).await?;
                }
                Ok(Some(session))
            } else {
                Err(ApplicationError::Runtime(
                    "Active session not found".to_string(),
                ))
            }
        } else {
            Ok(None)
        }
    }

    pub fn get_conversation_id_for_active_session(
        &self,
    ) -> Option<ConversationId> {
        self.current_conversation
            .as_ref()
            .and_then(|conversation| Some(conversation.id))
    }

    pub fn get_active_session_id(&self) -> Option<ConversationId> {
        self.current_conversation
            .as_ref()
            .map(|conversation| conversation.id)
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
    ) -> ConversationId {
        let conversation_id = prompt_instruction.get_conversation_id();
        let new_session = ThreadedChatSession::new(prompt_instruction);

        self.sessions.insert(conversation_id, new_session);
        conversation_id
    }

    pub fn stop_active_chat_session(&mut self) -> Result<(), ApplicationError> {
        if let Some(ref current_conversation) = self.current_conversation {
            if let Some(session) =
                self.sessions.get_mut(&current_conversation.id)
            {
                session.stop();
                Ok(())
            } else {
                Err(ApplicationError::Runtime(
                    "Active session not found".to_string(),
                ))
            }
        } else {
            Err(ApplicationError::Runtime(
                "No active session to stop".to_string(),
            ))
        }
    }
}
