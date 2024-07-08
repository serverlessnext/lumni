CREATE TABLE models (
    model_id INTEGER PRIMARY KEY AUTOINCREMENT,
    model_name TEXT NOT NULL,
    model_service TEXT NOT NULL UNIQUE
);

CREATE TABLE conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    metadata TEXT, -- JSON string including description and other metadata
    parent_conversation_id INTEGER,
    fork_exchange_id INTEGER,
    schema_version INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_conversation_id) REFERENCES conversations(id),
    FOREIGN KEY (fork_exchange_id) REFERENCES exchanges(id)
);

CREATE TABLE exchanges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    model_id INTEGER NOT NULL,
    system_prompt TEXT,
    completion_options TEXT, -- JSON string
    prompt_options TEXT, -- JSON string
    completion_tokens INTEGER,
    prompt_tokens INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id),
    FOREIGN KEY (model_id) REFERENCES models(model_id)
);

CREATE TABLE messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    exchange_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
    message_type TEXT NOT NULL,
    content TEXT NOT NULL,
    has_attachments BOOLEAN NOT NULL DEFAULT FALSE,
    token_length INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id),
    FOREIGN KEY (exchange_id) REFERENCES exchanges(id)
);

CREATE TABLE attachments (
    attachment_id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id INTEGER NOT NULL,
    conversation_id INTEGER NOT NULL,
    exchange_id INTEGER NOT NULL,
    file_uri TEXT,
    file_data BLOB,
    file_type TEXT NOT NULL,
    metadata TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (message_id) REFERENCES messages(id),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id),
    FOREIGN KEY (exchange_id) REFERENCES exchanges(id),
    CHECK ((file_uri IS NULL) != (file_data IS NULL))
);

CREATE INDEX idx_model_service ON models(model_service);
CREATE INDEX idx_conversation_id ON exchanges(conversation_id);
CREATE INDEX idx_exchange_id ON messages(exchange_id);
CREATE INDEX idx_parent_conversation ON conversations(parent_conversation_id);
CREATE INDEX idx_fork_exchange ON conversations(fork_exchange_id);
CREATE INDEX idx_model_id ON exchanges(model_id);
CREATE INDEX idx_conversation_created_at ON exchanges(conversation_id, created_at);
CREATE INDEX idx_attachment_message ON attachments(message_id);

PRAGMA foreign_keys = ON;
