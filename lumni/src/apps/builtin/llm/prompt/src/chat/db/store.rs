use std::path::PathBuf;

use rusqlite::Error as SqliteError;

use super::connector::DatabaseConnector;
use super::schema::{
    Attachment, AttachmentData, Conversation, Exchange, Message,
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

    pub fn store_new_conversation(&self, conversation: &Conversation) {
        let conversation_sql = format!(
            "INSERT INTO conversations (id, name, metadata, \
             parent_conversation_id, fork_exchange_id, schema_version, \
             created_at, updated_at, is_deleted)
            VALUES ({}, '{}', {}, {}, {}, {}, {}, {}, {});",
            conversation.id.0,
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

        // Commit the transaction
        eprintln!("Committing conversation");
    }

    pub fn store_finalized_exchange(
        &self,
        exchange: &Exchange,
        messages: &[Message],
        attachments: &[Attachment],
    ) {
        // Upsert the exchange
        let exchange_sql = format!(
            "INSERT INTO exchanges (id, conversation_id, model_id, \
             system_prompt, completion_options, prompt_options, \
             completion_tokens, prompt_tokens, created_at, \
             previous_exchange_id, is_deleted)
            VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})
            ON CONFLICT(id) DO UPDATE SET
            conversation_id = excluded.conversation_id,
            model_id = excluded.model_id,
            system_prompt = excluded.system_prompt,
            completion_options = excluded.completion_options,
            prompt_options = excluded.prompt_options,
            completion_tokens = excluded.completion_tokens,
            prompt_tokens = excluded.prompt_tokens,
            created_at = excluded.created_at,
            previous_exchange_id = excluded.previous_exchange_id,
            is_deleted = excluded.is_deleted;",
            exchange.id.0,
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

        // Upsert messages
        for message in messages {
            let message_sql = format!(
                "INSERT INTO messages (id, conversation_id, exchange_id, \
                 role, message_type, content, has_attachments, token_length, \
                 created_at, is_deleted)
                VALUES ({}, {}, {}, '{}', '{}', '{}', {}, {}, {}, {})
                ON CONFLICT(id) DO UPDATE SET
                conversation_id = excluded.conversation_id,
                exchange_id = excluded.exchange_id,
                role = excluded.role,
                message_type = excluded.message_type,
                content = excluded.content,
                has_attachments = excluded.has_attachments,
                token_length = excluded.token_length,
                created_at = excluded.created_at,
                is_deleted = excluded.is_deleted;",
                message.id.0,
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

        // Upsert attachments
        for attachment in attachments {
            let attachment_sql = format!(
                "INSERT INTO attachments (attachment_id, message_id, \
                 conversation_id, exchange_id, file_uri, file_data, \
                 file_type, metadata, created_at, is_deleted)
                VALUES ({}, {}, {}, {}, {}, {}, '{}', {}, {}, {})
                ON CONFLICT(attachment_id) DO UPDATE SET
                message_id = excluded.message_id,
                conversation_id = excluded.conversation_id,
                exchange_id = excluded.exchange_id,
                file_uri = excluded.file_uri,
                file_data = excluded.file_data,
                file_type = excluded.file_type,
                metadata = excluded.metadata,
                created_at = excluded.created_at,
                is_deleted = excluded.is_deleted;",
                attachment.attachment_id.0,
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
        //self.db.queue_operation("COMMIT;".to_string());
    }

    pub fn commit_queued_operations(&mut self) -> Result<(), SqliteError> {
        self.db.process_queue()
    }
}
