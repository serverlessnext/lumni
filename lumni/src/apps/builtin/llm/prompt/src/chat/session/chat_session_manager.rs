use std::collections::HashMap;

use lumni::api::error::ApplicationError;
use ratatui::style::Style;

use tokio::sync::mpsc;

use super::db::ConversationId;
use super::{App, ChatSession, TextWindowTrait};
pub use crate::external as lumni;

pub struct ChatSessionManager {
    sessions: HashMap<ConversationId, ChatSession>,
    active_session_id: ConversationId,
    ui_sender: mpsc::Sender<UiUpdate>,
    ui_receiver: mpsc::Receiver<UiUpdate>,
}

pub struct UiUpdate {
    pub content: String,
    pub style: Option<Style>,
}

impl ChatSessionManager {
    pub async fn new(initial_session: ChatSession) -> Self {
        let id = initial_session.get_conversation_id().unwrap();
        let mut sessions = HashMap::new();
        let (ui_sender, ui_receiver) = mpsc::channel(100);
        
        initial_session.set_ui_sender(Some(ui_sender.clone())).await;
        sessions.insert(id.clone(), initial_session);

        Self {
            sessions,
            active_session_id: id,
            ui_sender,
            ui_receiver,
        }
    }

    pub async fn switch_active_session(&mut self, id: ConversationId) -> Result<(), ApplicationError> {
        if self.sessions.contains_key(&id) {
            // Remove UI sender from the previous active session
            if let Some(prev_session) = self.sessions.get_mut(&self.active_session_id) {
                prev_session.set_ui_sender(None).await;
            }

            // Set UI sender for the new active session
            if let Some(new_session) = self.sessions.get_mut(&id) {
                new_session.set_ui_sender(Some(self.ui_sender.clone())).await;
            }

            self.active_session_id = id;
            Ok(())
        } else {
            Err(ApplicationError::InvalidInput("Session not found".to_string()))
        }
    }

    pub fn get_active_session(&mut self) -> &mut ChatSession {
        self.sessions.get_mut(&self.active_session_id).unwrap()
    }

    pub fn process_ui_updates(&mut self) -> Vec<UiUpdate> {
        let mut updates = Vec::new();
        while let Ok(update) = self.ui_receiver.try_recv() {
            updates.push(update);
        }
        updates
    }
}