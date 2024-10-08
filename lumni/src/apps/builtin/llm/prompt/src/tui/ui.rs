use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use lumni::api::error::ApplicationError;
use ratatui::widgets::Borders;

use super::modals::{ConversationListModal, FileBrowserModal, SettingsModal};
use super::{
    CommandLine, ConversationDatabase, ConversationDbHandler,
    ConversationEvent, ConversationId, ModalEvent, ModalWindowTrait,
    ModalWindowType, PromptWindow, ResponseWindow, TextLine, TextWindowTrait,
    WindowMode,
};
pub use crate::external as lumni;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ConversationUiMode {
    Chat,
    Instruction,
}

#[derive(Debug)]
pub struct ConversationUi<'a> {
    pub prompt: PromptWindow<'a>,
    pub response: ResponseWindow<'a>,
    pub mode: ConversationUiMode,
}

impl<'a> ConversationUi<'a> {
    pub fn new(conversation_text: Option<Vec<TextLine>>) -> Self {
        Self {
            prompt: PromptWindow::new().with_borders(Borders::ALL),
            response: ResponseWindow::new(conversation_text),
            mode: ConversationUiMode::Chat,
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

    pub fn set_response_window(&mut self) -> WindowMode {
        self.prompt.set_status_background();
        self.response.set_status_normal();
        self.response.scroll_to_end();
        WindowMode::Conversation(Some(ConversationEvent::Response))
    }

    pub fn set_prompt_window(&mut self, insert_mode: bool) -> WindowMode {
        self.response.set_status_background();
        if insert_mode {
            self.prompt.set_status_insert();
            WindowMode::Conversation(Some(ConversationEvent::PromptInsert))
        } else {
            self.prompt.set_status_normal();
            WindowMode::Conversation(Some(ConversationEvent::PromptRead))
        }
    }
}

pub struct AppUi<'a> {
    pub command_line: CommandLine<'a>,
    pub modal: Option<Box<dyn ModalWindowTrait>>,
    pub conversation_ui: ConversationUi<'a>,
}

impl AppUi<'_> {
    pub async fn new(conversation_text: Option<Vec<TextLine>>) -> Self {
        Self {
            command_line: CommandLine::new(),
            modal: None,
            conversation_ui: ConversationUi::new(conversation_text),
        }
    }

    pub fn init(&mut self) {
        self.conversation_ui.init();
        self.command_line.init();
    }

    pub async fn handle_key_event(
        &mut self,
        key: KeyEvent,
        window_mode: &mut WindowMode,
        handler: &ConversationDbHandler,
    ) -> Result<(), ApplicationError> {
        *window_mode = match key.code {
            KeyCode::BackTab => {
                // TODO: move in main navigation
                log::info!("BackTab");
                return Ok(());
            }
            KeyCode::Left => {
                self.modal = Some(Box::new(FileBrowserModal::new(None)));
                WindowMode::Modal(ModalEvent::PollBackGroundTask)
            }
            KeyCode::Right | KeyCode::Tab => {
                self.modal = Some(Box::new(
                    ConversationListModal::new(handler.clone()).await?,
                ));
                WindowMode::Modal(ModalEvent::UpdateUI)
            }
            KeyCode::Up => {
                // TODO: move up a block in conversation
                log::info!("Shift Up");
                return Ok(());
            }
            KeyCode::Down => {
                // TODO: move down a block in conversation
                log::info!("Shift Down");
                return Ok(());
            }
            _ => {
                return Ok(());
            }
        };
        Ok(())
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

    pub fn set_response_window(&mut self) -> WindowMode {
        self.conversation_ui.prompt.set_status_background();
        self.conversation_ui.response.set_status_normal();
        self.conversation_ui.response.scroll_to_end();
        WindowMode::Conversation(Some(ConversationEvent::Response))
    }

    pub fn set_prompt_window(&mut self, insert_mode: bool) -> WindowMode {
        self.conversation_ui.response.set_status_background();
        if insert_mode {
            self.conversation_ui.prompt.set_status_insert();
            WindowMode::Conversation(Some(ConversationEvent::PromptInsert))
        } else {
            self.conversation_ui.prompt.set_status_normal();
            WindowMode::Conversation(Some(ConversationEvent::PromptRead))
        }
    }

    pub fn clear_modal(&mut self) {
        self.modal = None;
    }

    pub async fn poll_widgets(&mut self) -> Result<bool, ApplicationError> {
        //        let mut redraw_ui = false;
        //
        //        match &mut self.selected_mode {
        //            ContentDisplayMode::FileBrowser(file_browser) => {
        //                match file_browser.poll_background_task().await? {
        //                    Some(ModalEvent::UpdateUI) => {
        //                        redraw_ui = true;
        //                    }
        //                    _ => {
        //                        // No update needed
        //                    }
        //                }
        //            }
        //            _ => {
        //                // No background task to poll
        //            }
        //        }
        //        Ok(redraw_ui)
        Ok(false)
    }
}
