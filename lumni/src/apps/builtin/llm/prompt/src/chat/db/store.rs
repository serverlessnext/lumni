use std::path::PathBuf;

use rusqlite::Error as SqliteError;

use super::connector::DatabaseConnector;
use super::schema::{
    Attachment, AttachmentData, Conversation, ConversationId,
    Exchange, Message,
};

pub struct ConversationDatabaseStore {
    db: DatabaseConnector,
}

impl ConversationDatabaseStore {
    pub fn new(sqlite_file: &PathBuf) -> Result<Self, SqliteError> {
        Ok(Self {
            db: DatabaseConnector::new(sqlite_file)?,
        })
    }

    pub fn store_new_conversation(
        &mut self,
        conversation: &Conversation,
    ) -> Result<ConversationId, SqliteError> {
        let conversation_sql = format!(
            "INSERT INTO conversations (name, metadata, \
             parent_conversation_id, fork_exchange_id, schema_version, \
             created_at, updated_at, is_deleted)
            VALUES ('{}', {}, {}, {}, {}, {}, {}, {});",
            conversation.name.replace("'", "''"),
            serde_json::to_string(&conversation.metadata)
                .map(|s| format!("'{}'", s.replace("'", "''")))
                .unwrap_or_else(|_| "NULL".to_string()),
            conversation
                .parent_conversation_id
                .map_or("NULL".to_string(), |id| id.0.to_string()),
            conversation
                .fork_exchange_id
                .map_or("NULL".to_string(), |id| id.0.to_string()),
            conversation.schema_version,
            conversation.created_at,
            conversation.updated_at,
            conversation.is_deleted
        );

        self.db.queue_operation(conversation_sql);

        self.db.process_queue_with_result(|tx| {
            let id = tx.last_insert_rowid();
            Ok(ConversationId(id))
        })
    }

    pub fn store_finalized_exchange(
        &mut self,
        exchange: &Exchange,
        messages: &[Message],
        attachments: &[Attachment],
    ) -> Result<(), SqliteError> {
        // Insert the exchange
        let exchange_sql = format!(
            "INSERT INTO exchanges (conversation_id, model_id, system_prompt, 
             completion_options, prompt_options, completion_tokens, 
             prompt_tokens, created_at, previous_exchange_id, is_deleted)
            VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {});",
            exchange.conversation_id.0,
            exchange.model_id.0,
            exchange.system_prompt.as_ref().map_or(
                "NULL".to_string(),
                |s| format!("'{}'", s.replace("'", "''"))
            ),
            exchange.completion_options.as_ref().map_or(
                "NULL".to_string(),
                |v| format!("'{}'", v.to_string().replace("'", "''"))
            ),
            exchange.prompt_options.as_ref().map_or(
                "NULL".to_string(),
                |v| format!("'{}'", v.to_string().replace("'", "''"))
            ),
            exchange
                .completion_tokens
                .map_or("NULL".to_string(), |t| t.to_string()),
            exchange
                .prompt_tokens
                .map_or("NULL".to_string(), |t| t.to_string()),
            exchange.created_at,
            exchange
                .previous_exchange_id
                .map_or("NULL".to_string(), |id| id.0.to_string()),
            exchange.is_deleted
        );
        self.db.queue_operation(exchange_sql);

        // Insert messages
        for message in messages {
            let message_sql = format!(
                "INSERT INTO messages (conversation_id, exchange_id, role, 
                 message_type, content, has_attachments, token_length, 
                 created_at, is_deleted)
                VALUES ({}, {}, '{}', '{}', '{}', {}, {}, {}, {});",
                message.conversation_id.0,
                message.exchange_id.0,
                message.role.to_string(),
                message.message_type,
                message.content.replace("'", "''"),
                message.has_attachments,
                message
                    .token_length
                    .map_or("NULL".to_string(), |t| t.to_string()),
                message.created_at,
                message.is_deleted
            );
            self.db.queue_operation(message_sql);
        }

        // Insert attachments
        for attachment in attachments {
            let attachment_sql = format!(
                "INSERT INTO attachments (message_id, conversation_id, 
                 exchange_id, file_uri, file_data, file_type, metadata, 
                 created_at, is_deleted)
                VALUES ({}, {}, {}, {}, {}, '{}', {}, {}, {});",
                attachment.message_id.0,
                attachment.conversation_id.0,
                attachment.exchange_id.0,
                match &attachment.data {
                    AttachmentData::Uri(uri) =>
                        format!("'{}'", uri.replace("'", "''")),
                    AttachmentData::Data(_) => "NULL".to_string(),
                },
                match &attachment.data {
                    AttachmentData::Uri(_) => "NULL".to_string(),
                    AttachmentData::Data(data) =>
                        format!("X'{}'", hex::encode(data)),
                },
                attachment.file_type,
                attachment.metadata.as_ref().map_or(
                    "NULL".to_string(),
                    |v| format!("'{}'", v.to_string().replace("'", "''"))
                ),
                attachment.created_at,
                attachment.is_deleted
            );
            self.db.queue_operation(attachment_sql);
        }

        // Commit the transaction
        self.commit_queued_operations()?;

        Ok(())
    }

    fn commit_queued_operations(&mut self) -> Result<(), SqliteError> {
        let result = self.db.process_queue()?;
        eprintln!("Commit Result: {:?}", result);
        Ok(result)
    }
}
