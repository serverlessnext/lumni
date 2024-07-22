use async_trait::async_trait;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
use ratatui::Frame;

use super::components::Scroller;
use super::events::KeyTrack;
use super::widgets::SelectEndpoint;
use super::{
    ApplicationError, ChatSession, ConversationEvent, ConversationReader,
    ModelServer, ModelSpec, NewConversation, ServerManager, ServerTrait,
    WindowEvent,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModalWindowType {
    Config,
}

#[async_trait]
pub trait ModalWindowTrait {
    fn get_type(&self) -> ModalWindowType;
    fn render_on_frame(&mut self, frame: &mut Frame, area: Rect);
    async fn handle_key_event<'a>(
        &'a mut self,
        key_event: &'a mut KeyTrack,
        tab_chat: &'a mut ChatSession,
        reader: Option<&ConversationReader<'_>>,
    ) -> Result<Option<WindowEvent>, ApplicationError>;
}

pub struct ModalConfigWindow {
    widget: SelectEndpoint,
    _scroller: Option<Scroller>,
}

impl ModalConfigWindow {
    pub fn new() -> Self {
        Self {
            widget: SelectEndpoint::new(),
            _scroller: None,
        }
    }
}

#[async_trait]
impl ModalWindowTrait for ModalConfigWindow {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::Config
    }

    fn render_on_frame(&mut self, frame: &mut Frame, mut area: Rect) {
        let (max_width, max_height) = self.widget.max_area_size();
        if area.width > max_width {
            area.x = area.width.saturating_sub(max_width);
            area.width = max_width;
        };
        if area.height > max_height {
            area.height = max_height;
        };
        frame.render_widget(Clear, area);
        frame.render_widget(&mut self.widget, area);
    }

    async fn handle_key_event<'a>(
        &'a mut self,
        key_event: &'a mut KeyTrack,
        tab_chat: &'a mut ChatSession,
        reader: Option<&ConversationReader<'_>>,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Up => self.widget.key_up(),
            KeyCode::Down => self.widget.key_down(),
            KeyCode::Enter => {
                let selected_server = self.widget.current_endpoint();
                // TODO: allow model selection, + check if model changes
                if selected_server != tab_chat.server_name() {
                    let server = ModelServer::from_str(selected_server)?;

                    match server.get_default_model().await {
                        Ok(model) => {
                            let new_conversation = NewConversation::new(
                                server.server_name(),
                                model,
                                reader,
                            )?;
                            // Return the new conversation event
                            return Ok(Some(WindowEvent::PromptWindow(Some(
                                ConversationEvent::NewConversation(
                                    new_conversation,
                                ),
                            ))));
                        }
                        Err(ApplicationError::NotReady(e)) => {
                            // already a NotReady error
                            return Err(ApplicationError::NotReady(e));
                        }
                        Err(e) => {
                            // ensure each error is converted to NotReady,
                            // with additional logging as its unexpected
                            log::error!("Error: {}", e);
                            return Err(ApplicationError::NotReady(
                                e.to_string(),
                            ));
                        }
                    }
                }
                return Ok(Some(WindowEvent::PromptWindow(None)));
            }
            KeyCode::Left => {
                let server =
                    ModelServer::from_str(self.widget.current_endpoint())?;
                let _models = server.list_models().await?;
            }
            _ => {} // Ignore other keys
        }
        // stay in the modal window
        Ok(Some(WindowEvent::Modal(ModalWindowType::Config)))
    }
}
