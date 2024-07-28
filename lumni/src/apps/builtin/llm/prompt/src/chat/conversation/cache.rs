use std::collections::HashMap;

use ratatui::style::Style;

use super::db::{Attachment, AttachmentId, ConversationId, Message, MessageId};
use super::{ColorScheme, PromptRole, TextLine, TextSegment};

#[derive(Debug, Clone)]
pub struct ConversationCache {
    conversation_id: ConversationId,
    messages: Vec<Message>, // messages have to be ordered
    attachments: HashMap<AttachmentId, Attachment>,
    message_attachments: HashMap<MessageId, Vec<AttachmentId>>,
    preloaded_messages: usize,
}

impl ConversationCache {
    pub fn new() -> Self {
        ConversationCache {
            conversation_id: ConversationId(-1),
            messages: Vec::new(),
            attachments: HashMap::new(),
            message_attachments: HashMap::new(),
            preloaded_messages: 0,
        }
    }

    pub fn get_conversation_id(&self) -> ConversationId {
        self.conversation_id
    }

    pub fn set_conversation_id(&mut self, conversation_id: ConversationId) {
        self.conversation_id = conversation_id;
    }

    pub fn set_preloaded_messages(&mut self, preloaded_messages: usize) {
        self.preloaded_messages = preloaded_messages;
    }

    pub fn new_message_id(&self) -> MessageId {
        MessageId(self.messages.len() as i64)
    }

    pub fn new_attachment_id(&self) -> AttachmentId {
        AttachmentId(self.attachments.len() as i64)
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn get_last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn get_last_message_id(&self) -> Option<MessageId> {
        self.get_last_message().map(|m| m.id)
    }

    pub fn get_message_by_id(&self, message_id: MessageId) -> Option<&Message> {
        self.messages.iter().find(|m| m.id == message_id)
    }

    pub fn get_conversation_messages(&self) -> Vec<&Message> {
        self.messages.iter().collect()
    }

    pub fn update_message_by_id(
        &mut self,
        message_id: MessageId,
        new_content: &str,
        new_token_length: Option<i64>,
    ) {
        if let Some(message) =
            self.messages.iter_mut().find(|m| m.id == message_id)
        {
            message.content = new_content.to_string();
            message.token_length = new_token_length;
        }
    }

    pub fn update_message_token_length(
        &mut self,
        message_id: &MessageId,
        new_token_length: i64,
    ) {
        if let Some(message) =
            self.messages.iter_mut().find(|m| m.id == *message_id)
        {
            message.token_length = Some(new_token_length);
        }
    }

    pub fn add_attachment(&mut self, attachment: Attachment) {
        self.message_attachments
            .entry(attachment.message_id)
            .or_default()
            .push(attachment.attachment_id);
        self.attachments
            .insert(attachment.attachment_id, attachment);
    }
}

impl ConversationCache {
    pub fn export_conversation(
        &self,
        color_scheme: &ColorScheme,
    ) -> Vec<TextLine> {
        let mut lines = Vec::new();
        let mut skip_count = self.preloaded_messages;
        if !self.messages.is_empty()
            && self.messages[0].role == PromptRole::System
        {
            skip_count += 1;
        }

        for message in self
            .messages
            .iter()
            .skip(skip_count)
            .filter(|m| !m.is_deleted && m.role != PromptRole::System)
        {
            let style = match message.role {
                PromptRole::User => Some(color_scheme.get_primary_style()),
                PromptRole::Assistant => {
                    Some(color_scheme.get_secondary_style())
                }
                _ => None,
            };

            // Split message content into lines
            for line in message.content.lines() {
                lines.push(TextLine {
                    segments: vec![TextSegment {
                        text: line.to_string(),
                        style: style.clone(),
                    }],
                    length: line.len(),
                    background: None,
                });
            }

            // Add an empty line after each message
            lines.push(TextLine {
                segments: vec![TextSegment {
                    text: String::new(),
                    style: Some(Style::reset()),
                }],
                length: 0,
                background: None,
            });

            // Add an extra empty line for assistant messages
            if message.role == PromptRole::Assistant {
                lines.push(TextLine {
                    segments: vec![TextSegment {
                        text: String::new(),
                        style: Some(Style::reset()),
                    }],
                    length: 0,
                    background: None,
                });
            }
        }

        lines
    }
}
