
CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE user_profiles (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    options TEXT NOT NULL, -- JSON string
    is_default INTEGER DEFAULT 0,
    encryption_key_id INTEGER NOT NULL,
    provider_config_id INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (encryption_key_id) REFERENCES encryption_keys(id)
    FOREIGN KEY (provider_config_id) REFERENCES provider_configs(id)
);

CREATE TABLE provider_configs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    model_identifier TEXT,
    additional_settings TEXT, -- JSON string
    encryption_key_id INTEGER NOT NULL,
    FOREIGN KEY (encryption_key_id) REFERENCES encryption_keys(id)
);

CREATE TABLE encryption_keys (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    sha256_hash TEXT NOT NULL UNIQUE
);

CREATE TABLE models (
    identifier TEXT PRIMARY KEY,
    info TEXT, -- JSON string
    config TEXT, -- JSON string
    context_window_size INTEGER,
    input_token_limit INTEGER
);

CREATE TABLE conversations (
    id INTEGER PRIMARY KEY,
    name TEXT,
    info TEXT, -- JSON string including description and other metadata
    completion_options TEXT, -- JSON string
    model_identifier TEXT NOT NULL,
    parent_conversation_id INTEGER,
    fork_message_id INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    message_count INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    is_deleted BOOLEAN DEFAULT FALSE,  -- NOTE, will be removed given that we have status
    is_pinned BOOLEAN DEFAULT FALSE,
    status TEXT CHECK(status IN ('active', 'archived', 'deleted')) DEFAULT 'active',
    FOREIGN KEY (parent_conversation_id) REFERENCES conversations(id),
    FOREIGN KEY (model_identifier) REFERENCES models(identifier),
    FOREIGN KEY (fork_message_id) REFERENCES messages(id),
    CONSTRAINT check_message_count CHECK (message_count >= 0),
    CONSTRAINT check_total_tokens CHECK (total_tokens >= 0)
);

CREATE TABLE messages (
    id INTEGER PRIMARY KEY,
    conversation_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
    message_type TEXT NOT NULL,
    content TEXT NOT NULL,
    has_attachments BOOLEAN NOT NULL DEFAULT FALSE,
    token_length INTEGER,
    previous_message_id INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    vote INTEGER DEFAULT 0,
    include_in_prompt BOOLEAN DEFAULT TRUE,
    is_hidden BOOLEAN DEFAULT FALSE,
    is_deleted BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id),
    FOREIGN KEY (previous_message_id) REFERENCES messages(id),
    CONSTRAINT check_token_length CHECK (token_length >= 0)
);

CREATE TABLE attachments (
    attachment_id INTEGER PRIMARY KEY,
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

CREATE TABLE tags (
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE conversation_tags (
    conversation_id INTEGER REFERENCES conversations(id),
    tag_id INTEGER REFERENCES tags(id),
    PRIMARY KEY (conversation_id, tag_id)
);

CREATE INDEX idx_parent_conversation ON conversations(parent_conversation_id);
CREATE INDEX idx_conversation_model_identifier ON conversations(model_identifier);  
CREATE INDEX idx_attachment_message ON attachments(message_id);
CREATE INDEX idx_conversation_is_deleted_updated ON conversations(is_deleted, updated_at);
CREATE INDEX idx_message_conversation_created ON messages(conversation_id, created_at);
CREATE INDEX idx_message_previous ON messages(previous_message_id);
CREATE INDEX idx_attachment_conversation ON attachments(conversation_id);
CREATE INDEX idx_conversation_pinned_updated ON conversations(is_pinned DESC, updated_at DESC);
CREATE UNIQUE INDEX idx_user_profiles_default ON user_profiles(is_default) WHERE is_default = 1;