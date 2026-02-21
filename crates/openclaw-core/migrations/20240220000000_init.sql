-- Create sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    channel TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    metadata TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- Create messages table
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    images TEXT NOT NULL,
    tool_calls TEXT NOT NULL,
    tool_result TEXT,
    timestamp DATETIME NOT NULL,
    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Create index for faster lookups
CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_peer ON sessions(channel, peer_id);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);

-- Create embeddings table
CREATE TABLE IF NOT EXISTS embeddings (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    vector TEXT NOT NULL, -- Storing as JSON string
    created_at DATETIME NOT NULL
);
