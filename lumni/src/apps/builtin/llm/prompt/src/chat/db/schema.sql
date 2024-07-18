
PRAGMA foreign_keys = ON;

CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE models (
    model_id INTEGER PRIMARY KEY AUTOINCREMENT,
    model_name TEXT NOT NULL,
    model_service TEXT NOT NULL UNIQUE
);

CREATE TABLE conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    metadata TEXT, -- JSON string including description and other metadata
    model_id INTEGER NOT NULL,
    parent_conversation_id INTEGER,
    fork_message_id INTEGER,
    completion_options TEXT, -- JSON string
    schema_version INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    message_count INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    is_deleted BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (parent_conversation_id) REFERENCES conversations(id),
    FOREIGN KEY (model_id) REFERENCES models(model_id),
    FOREIGN KEY (fork_message_id) REFERENCES messages(id)
);

CREATE TABLE messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
    message_type TEXT NOT NULL,
    content TEXT NOT NULL,
    has_attachments BOOLEAN NOT NULL DEFAULT FALSE,
    token_length INTEGER,
    previous_message_id INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_deleted BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id),
    FOREIGN KEY (previous_message_id) REFERENCES messages(id)
);

CREATE TABLE attachments (
    attachment_id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id INTEGER NOT NULL,
    conversation_id INTEGER NOT NULL,
    file_uri TEXT,
    file_data BLOB,
    file_type TEXT NOT NULL,
    metadata TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_deleted BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (message_id) REFERENCES messages(id),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id),
    CHECK ((file_uri IS NULL) != (file_data IS NULL))
);

CREATE INDEX idx_model_service ON models(model_service);
CREATE INDEX idx_parent_conversation ON conversations(parent_conversation_id);
CREATE INDEX idx_conversation_model_id ON conversations(model_id);  
CREATE INDEX idx_attachment_message ON attachments(message_id);
CREATE INDEX idx_conversation_is_deleted_updated ON conversations(is_deleted, updated_at);
CREATE INDEX idx_message_conversation_created ON messages(conversation_id, created_at);
CREATE INDEX idx_message_previous ON messages(previous_message_id);
CREATE INDEX idx_attachment_conversation ON attachments(conversation_id);