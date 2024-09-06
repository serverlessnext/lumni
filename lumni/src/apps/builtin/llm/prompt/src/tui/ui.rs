use std::sync::Arc;

use lumni::api::error::ApplicationError;
use ratatui::widgets::Borders;

use super::modals::{ConversationListModal, FileBrowserModal, SettingsModal};
use super::widgets::FileBrowser;
use super::{
    CommandLine, ConversationDatabase, ConversationId, ConversationWindowEvent,
    ModalWindowTrait, ModalWindowType, PromptWindow, ResponseWindow, TextLine,
    TextWindowTrait, WindowEvent, WindowKind,
};
pub use crate::external as lumni;
#[derive(Debug)]
pub enum NavigationMode {
    Conversation(ConversationUi<'static>),
    File,
}

#[derive(Debug)]
pub struct ConversationUi<'a> {
    pub prompt: PromptWindow<'a>,
    pub response: ResponseWindow<'a>,
    pub primary_window: WindowKind,
}

impl<'a> ConversationUi<'a> {
    pub fn new(conversation_text: Option<Vec<TextLine>>) -> Self {
        Self {
            prompt: PromptWindow::new().with_borders(Borders::ALL),
            response: ResponseWindow::new(conversation_text),
            primary_window: WindowKind::ResponseWindow,
        }
    }

    pub fn reload_conversation_text(
        &mut self,
        conversation_text: Vec<TextLine>,
    ) {
        self.response = ResponseWindow::new(Some(conversation_text));
    }

    pub fn init(&mut self) {
        self.response.init();
        self.prompt.set_status_normal();
    }

    pub fn set_response_window(&mut self) -> WindowEvent {
        self.prompt.set_status_background();
        self.response.set_status_normal();
        self.response.scroll_to_end();
        WindowEvent::Conversation(ConversationWindowEvent::Response)
    }

    pub fn set_prompt_window(&mut self, insert_mode: bool) -> WindowEvent {
        self.response.set_status_background();
        if insert_mode {
            self.prompt.set_status_insert();
        } else {
            self.prompt.set_status_normal();
        }
        WindowEvent::Conversation(ConversationWindowEvent::Prompt(None))
    }

    pub fn set_primary_window(&mut self, window_type: WindowKind) {
        self.primary_window = match window_type {
            WindowKind::ResponseWindow | WindowKind::EditorWindow => {
                window_type
            }
            _ => unreachable!("Invalid primary window type: {:?}", window_type),
        };
    }
}
pub struct AppUi<'a> {
    pub command_line: CommandLine<'a>,
    pub modal: Option<Box<dyn ModalWindowTrait>>,
    pub selected_mode: NavigationMode,
    pub file_browser: FileBrowser,
}

impl AppUi<'_> {
    pub fn new(conversation_text: Option<Vec<TextLine>>) -> Self {
        Self {
            command_line: CommandLine::new(),
            modal: None,
            //selected_mode: NavigationMode::Conversation(ConversationUi::new(
            //    conversation_text,
            //)),
            selected_mode: NavigationMode::File,
            file_browser: FileBrowser::new(None),
        }
    }

    pub fn init(&mut self) {
        if let NavigationMode::Conversation(conv_ui) = &mut self.selected_mode {
            conv_ui.init();
        }
        self.command_line.init();
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

    pub fn set_response_window(&mut self) -> WindowEvent {
        if let NavigationMode::Conversation(conv_ui) = &mut self.selected_mode {
            conv_ui.prompt.set_status_background();
            conv_ui.response.set_status_normal();
            conv_ui.response.scroll_to_end();
        } else {
            unimplemented!("TODO: switch to ResponseWindow");
        }
        WindowEvent::Conversation(ConversationWindowEvent::Response)
    }

    pub fn set_prompt_window(&mut self, insert_mode: bool) -> WindowEvent {
        if let NavigationMode::Conversation(conv_ui) = &mut self.selected_mode {
            conv_ui.response.set_status_background();
            if insert_mode {
                conv_ui.prompt.set_status_insert();
            } else {
                conv_ui.prompt.set_status_normal();
            }
        } else {
            unimplemented!("TODO: switch to PromptWindow");
        }
        WindowEvent::Conversation(ConversationWindowEvent::Prompt(None))
    }

    pub fn clear_modal(&mut self) {
        self.modal = None;
    }

    pub fn switch_to_conversation_mode(
        &mut self,
        conversation_text: Option<Vec<TextLine>>,
    ) {
        self.selected_mode = NavigationMode::Conversation(ConversationUi::new(
            conversation_text,
        ));
    }
}
