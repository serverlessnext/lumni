mod select;

use async_trait::async_trait;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
use ratatui::Frame;
use select::SelectEndpoint;

use super::{
    ApplicationError, ChatSession, ConversationEvent, ConversationReader,
    KeyTrack, ModalWindowTrait, ModalWindowType, ModelServer, NewConversation,
    Scroller, ServerManager, ServerTrait, WindowEvent,
    SUPPORTED_MODEL_ENDPOINTS,
};

pub struct SelectEndpointModal {
    widget: SelectEndpoint,
    _scroller: Option<Scroller>,
}

impl SelectEndpointModal {
    pub fn new() -> Self {
        Self {
            widget: SelectEndpoint::new(),
            _scroller: None,
        }
    }
}

#[async_trait]
impl ModalWindowTrait for SelectEndpointModal {
    fn get_type(&self) -> ModalWindowType {
        ModalWindowType::SelectEndpoint
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
        reader: &mut ConversationReader<'_>,
    ) -> Result<Option<WindowEvent>, ApplicationError> {
        match key_event.current_key().code {
            KeyCode::Up => self.widget.key_up(),
            KeyCode::Down => self.widget.key_down(),
            KeyCode::Enter => {
                let selected_server = self.widget.current_endpoint();
                // TODO: allow model selection, + check if model changes
                // TODO: catch ApplicationError::NotReady, if it is assume selected_server != tab_chat.server_name()
                let should_create_new_conversation =
                    match tab_chat.server_name() {
                        Ok(current_server_name) => {
                            selected_server != current_server_name
                        }
                        Err(ApplicationError::NotReady(_)) => true, // Assume new server if NotReady
                        Err(e) => return Err(e), // Propagate other errors
                    };

                let event = if should_create_new_conversation {
                    let server = ModelServer::from_str(selected_server)?;
                    match server.get_default_model().await {
                        Ok(model) => {
                            let new_conversation = NewConversation::new(
                                server.server_name(),
                                model,
                                &reader,
                            )?;
                            // Return the new conversation event
                            Ok(Some(WindowEvent::PromptWindow(Some(
                                ConversationEvent::NewConversation(
                                    new_conversation,
                                ),
                            ))))
                        }
                        Err(ApplicationError::NotReady(e)) => {
                            // already a NotReady error
                            Err(ApplicationError::NotReady(e))
                        }
                        Err(e) => {
                            // ensure each error is converted to NotReady,
                            // with additional logging as its unexpected
                            log::error!("Error: {}", e);
                            Err(ApplicationError::NotReady(e.to_string()))
                        }
                    }
                } else {
                    Ok(Some(WindowEvent::PromptWindow(None)))
                };
                return event;
            }
            KeyCode::Left => {
                let server =
                    ModelServer::from_str(self.widget.current_endpoint())?;
                let _models = server.list_models().await?;
            }
            _ => {} // Ignore other keys
        }
        // stay in the modal window
        Ok(Some(WindowEvent::Modal(ModalWindowType::SelectEndpoint)))
    }
}
