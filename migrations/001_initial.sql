-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_login TEXT,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    permissions TEXT NOT NULL -- JSON array of permissions
);

-- Create file_metadata table
CREATE TABLE IF NOT EXISTS file_metadata (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    size INTEGER NOT NULL,
    mime_type TEXT NOT NULL,
    checksum TEXT NOT NULL,
    created_at TEXT NOT NULL,
    modified_at TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    is_directory BOOLEAN NOT NULL DEFAULT 0,
    parent_id TEXT,
    permissions TEXT NOT NULL, -- JSON object with file permissions
    FOREIGN KEY (owner_id) REFERENCES users (id),
    FOREIGN KEY (parent_id) REFERENCES file_metadata (id)
);

-- Create sync_sessions table
CREATE TABLE IF NOT EXISTS sync_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    device_name TEXT NOT NULL,
    last_sync TEXT NOT NULL,
    sync_folders TEXT NOT NULL, -- JSON array of folder paths
    is_active BOOLEAN NOT NULL DEFAULT 1,
    FOREIGN KEY (user_id) REFERENCES users (id),
    UNIQUE(user_id, device_id)
);

-- Create share_links table
CREATE TABLE IF NOT EXISTS share_links (
    id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL,
    created_by TEXT NOT NULL,
    share_token TEXT UNIQUE NOT NULL,
    expires_at TEXT,
    password_protected BOOLEAN NOT NULL DEFAULT 0,
    download_count INTEGER NOT NULL DEFAULT 0,
    max_downloads INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (file_id) REFERENCES file_metadata (id),
    FOREIGN KEY (created_by) REFERENCES users (id)
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_users_username ON users (username);
CREATE INDEX IF NOT EXISTS idx_file_metadata_owner ON file_metadata (owner_id);
CREATE INDEX IF NOT EXISTS idx_file_metadata_parent ON file_metadata (parent_id);
CREATE INDEX IF NOT EXISTS idx_file_metadata_path ON file_metadata (path);
CREATE INDEX IF NOT EXISTS idx_sync_sessions_user ON sync_sessions (user_id);
CREATE INDEX IF NOT EXISTS idx_share_links_token ON share_links (share_token);
CREATE INDEX IF NOT EXISTS idx_share_links_file ON share_links (file_id);
