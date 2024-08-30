use std::sync::Arc;

use lumni::api::error::ApplicationError;
use ratatui::widgets::Borders;

use super::modals::{ConversationListModal, FileBrowserModal, SettingsModal};
use super::{
    CommandLine, ConversationDatabase, ConversationId, ModalWindowTrait,
    ModalWindowType, ResponseWindow, TextArea, TextLine, TextWindowTrait,
    WindowEvent, WindowKind,
};
pub use crate::external as lumni;

pub struct AppUi<'a> {
    pub prompt: TextArea<'a>,
    pub response: ResponseWindow<'a>,
    pub command_line: CommandLine<'a>,
    pub primary_window: WindowKind,
    pub modal: Option<Box<dyn ModalWindowTrait>>,
}

impl AppUi<'_> {
    pub fn new(conversation_text: Option<Vec<TextLine>>) -> Self {
        Self {
            prompt: TextArea::new().with_borders(Borders::ALL),
            response: ResponseWindow::new(conversation_text),
            command_line: CommandLine::new(),
            primary_window: WindowKind::ResponseWindow,
            modal: None,
        }
    }

    pub fn reload_conversation_text(
        &mut self,
        conversation_text: Vec<TextLine>,
    ) {
        self.response = ResponseWindow::new(Some(conversation_text));
    }

    pub fn init(&mut self) {
        self.response.init(); //set_status_normal(); // initialize in normal mode
        self.prompt.set_status_normal(); // initialize with defaults
        self.command_line.init(); // initialize with defaults
    }

    pub fn set_alert(&mut self, message: &str) -> Result<(), ApplicationError> {
        self.command_line.set_alert(message)
    }

    pub async fn set_new_modal(
        &mut self,
        modal_type: ModalWindowType,
        db_conn: &Arc<ConversationDatabase>,
        conversation_id: Option<ConversationId>,
    ) -> Result<(), ApplicationError> {
        self.modal = match modal_type {
            ModalWindowType::ConversationList => {
                let handler = db_conn.get_conversation_handler(conversation_id);
                Some(Box::new(ConversationListModal::new(handler).await?))
            }
            ModalWindowType::ProfileEdit => {
                let handler = db_conn.get_profile_handler(None);
                Some(Box::new(SettingsModal::new(handler).await?))
            }
            ModalWindowType::FileBrowser => {
                // TODO: get dir from profile
                Some(Box::new(FileBrowserModal::new(None)))
            }
        };
        Ok(())
    }

    pub fn set_primary_window(&mut self, window_type: WindowKind) {
        self.primary_window = match window_type {
            WindowKind::ResponseWindow | WindowKind::EditorWindow => {
                window_type
            }
            _ => {
                // only ResponseWindow and PromptWindow can be primary windows
                unreachable!("Invalid primary window type: {:?}", window_type)
            }
        };
    }

    pub fn set_response_window(&mut self) -> WindowEvent {
        self.prompt.set_status_background();
        self.response.set_status_normal();
        self.response.scroll_to_end();
        return WindowEvent::ResponseWindow;
    }

    pub fn set_prompt_window(&mut self, insert_mode: bool) -> WindowEvent {
        self.response.set_status_background();
        if insert_mode {
            self.prompt.set_status_insert();
        } else {
            self.prompt.set_status_normal();
        }
        return WindowEvent::PromptWindow(None);
    }

    pub fn clear_modal(&mut self) {
        self.modal = None;
    }
}
