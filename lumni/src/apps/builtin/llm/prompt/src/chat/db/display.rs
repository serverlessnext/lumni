use lumni::api::error::ApplicationError;

use super::schema::{
    Conversation, ConversationId, Message,
};
use super::ConversationDatabaseStore;
pub use crate::external as lumni;

impl ConversationDatabaseStore {
    pub async fn print_last_conversation(
        &self,
    ) -> Result<(), ApplicationError> {
        if let Some((conversation, messages)) =
            self.fetch_conversation(None, None)?
        {
            display_conversation_with_messages(&conversation, &messages);
        } else {
            println!("No conversations found.");
        }
        Ok(())
    }

    pub async fn print_conversation_list(
        &self,
        limit: usize,
    ) -> Result<(), ApplicationError> {
        let conversations = self.fetch_conversation_list(limit)?;
        for conversation in conversations {
            println!(
                "ID: {}, Name: {}, Updated: {}",
                conversation.id.0, conversation.name, conversation.updated_at
            );
        }
        Ok(())
    }

    pub async fn print_conversation_by_id(
        &self,
        id: &str,
    ) -> Result<(), ApplicationError> {
        let conversation_id = ConversationId(id.parse().map_err(|_| {
            ApplicationError::NotFound(
                format!("Conversation {id} not found in database"),
        )})?);

        if let Some((conversation, messages)) =
            self.fetch_conversation(Some(conversation_id), None)?
        {
            display_conversation_with_messages(&conversation, &messages);
        } else {
            println!("Conversation not found.");
        }
        Ok(())
    }
}

fn display_conversation_with_messages(
    conversation: &Conversation,
    messages: &[Message],
) {
    println!(
        "Conversation: {} (ID: {})",
        conversation.name, conversation.id.0
    );
    println!("Updated at: {}", conversation.updated_at);

    if !messages.is_empty() {
        println!("Messages:");
        for message in messages {
            println!("  Role: {}", message.role);
            println!("  Content: {}", message.content);
            println!("  ---");
        }
    } else {
        println!("No messages");
    }
    println!("===============================");
}
