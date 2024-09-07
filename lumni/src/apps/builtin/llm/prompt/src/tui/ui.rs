use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use lumni::api::error::ApplicationError;
use ratatui::widgets::Borders;

use super::modals::{ConversationListModal, FileBrowserModal, SettingsModal};
use super::widgets::FileBrowser;
use super::{
    CommandLine, ConversationDatabase, ConversationEvent, ConversationId,
    ModalEvent, ModalWindowTrait, ModalWindowType, PromptWindow,
    ResponseWindow, TextLine, TextWindowTrait, WindowKind, WindowMode,
};
pub use crate::external as lumni;
#[derive(Debug)]
pub enum ContentDisplayMode {
    Conversation(ConversationUi<'static>),
    FileBrowser(FileBrowser),
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
        } else {
            self.prompt.set_status_normal();
        }
        WindowMode::Conversation(Some(ConversationEvent::Prompt))
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
    pub selected_mode: ContentDisplayMode,
}

impl AppUi<'_> {
    pub async fn new(conversation_text: Option<Vec<TextLine>>) -> Self {
        Self {
            command_line: CommandLine::new(),
            modal: None,
            selected_mode: ContentDisplayMode::Conversation(
                ConversationUi::new(conversation_text),
            ),
        }
    }

    pub fn init(&mut self) {
        if let ContentDisplayMode::Conversation(conv_ui) =
            &mut self.selected_mode
        {
            conv_ui.init();
        }
        self.command_line.init();
    }

    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        window_mode: &mut WindowMode,
    ) {
        *window_mode = match key.code {
            KeyCode::Left | KeyCode::BackTab => {
                self.switch_tab(TabDirection::Left)
            }
            KeyCode::Right | KeyCode::Tab => {
                self.switch_tab(TabDirection::Right)
            }
            _ => {
                return;
            }
        };
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
        if let ContentDisplayMode::Conversation(conv_ui) =
            &mut self.selected_mode
        {
            conv_ui.prompt.set_status_background();
            conv_ui.response.set_status_normal();
            conv_ui.response.scroll_to_end();
        } else {
            unimplemented!("TODO: switch to ResponseWindow");
        }
        WindowMode::Conversation(Some(ConversationEvent::Response))
    }

    pub fn set_prompt_window(&mut self, insert_mode: bool) -> WindowMode {
        if let ContentDisplayMode::Conversation(conv_ui) =
            &mut self.selected_mode
        {
            conv_ui.response.set_status_background();
            if insert_mode {
                conv_ui.prompt.set_status_insert();
            } else {
                conv_ui.prompt.set_status_normal();
            }
        } else {
            unimplemented!("TODO: switch to PromptWindow");
        }
        WindowMode::Conversation(Some(ConversationEvent::Prompt))
    }

    pub fn clear_modal(&mut self) {
        self.modal = None;
    }

    pub fn switch_to_conversation_mode(
        &mut self,
        conversation_text: Option<Vec<TextLine>>,
    ) {
        self.selected_mode = ContentDisplayMode::Conversation(
            ConversationUi::new(conversation_text),
        );
    }

    fn switch_tab(&mut self, direction: TabDirection) -> WindowMode {
        self.selected_mode = match self.selected_mode {
            ContentDisplayMode::Conversation(_) => {
                // as there are only two tabs currently, both directions are currently the same
                let display_mode =
                    ContentDisplayMode::FileBrowser(FileBrowser::new(None));
                match direction {
                    TabDirection::Right => display_mode,
                    TabDirection::Left => display_mode,
                }
            }
            ContentDisplayMode::FileBrowser(_) => {
                let display_mode =
                    ContentDisplayMode::Conversation(ConversationUi::new(None));
                match direction {
                    TabDirection::Right => display_mode,
                    TabDirection::Left => display_mode,
                }
            }
        };
        WindowMode::Select
    }

    pub async fn poll_widgets(&mut self) -> Result<bool, ApplicationError> {
        let mut redraw_ui = false;

        match &mut self.selected_mode {
            ContentDisplayMode::FileBrowser(file_browser) => {
                match file_browser.poll_background_task().await? {
                    Some(ModalEvent::UpdateUI) => {
                        redraw_ui = true;
                    }
                    _ => {
                        // No update needed
                    }
                }
            }
            _ => {
                // No background task to poll
            }
        }
        Ok(redraw_ui)
    }
}
enum TabDirection {
    Left,
    Right,
}
