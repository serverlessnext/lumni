use lumni::api::error::ApplicationError;

use super::modals::{ConversationListModal, SelectEndpointModal};
use super::{
    CommandLine, ConversationDbHandler, ModalWindowTrait, ModalWindowType,
    PromptWindow, ResponseWindow, TextLine, TextWindowTrait, WindowEvent,
    WindowKind,
};
pub use crate::external as lumni;

pub struct AppUi<'a> {
    pub prompt: PromptWindow<'a>,
    pub response: ResponseWindow<'a>,
    pub command_line: CommandLine<'a>,
    pub primary_window: WindowKind,
    pub modal: Option<Box<dyn ModalWindowTrait>>,
}

impl AppUi<'_> {
    pub fn new(conversation_text: Option<Vec<TextLine>>) -> Self {
        Self {
            prompt: PromptWindow::new(),
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
        handler: &ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        self.modal = match modal_type {
            ModalWindowType::SelectEndpoint => {
                Some(Box::new(SelectEndpointModal::new()))
            }
            ModalWindowType::ConversationList(_) => {
                Some(Box::new(ConversationListModal::new(handler).await?))
            }
        };
        Ok(())
    }

    pub fn needs_modal_update(&self, new_type: &ModalWindowType) -> bool {
        match self.modal.as_ref() {
            Some(modal) => *new_type != modal.get_type(),
            None => true,
        }
    }

    pub fn set_primary_window(&mut self, window_type: WindowKind) {
        self.primary_window = match window_type {
            WindowKind::ResponseWindow | WindowKind::PromptWindow => {
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
