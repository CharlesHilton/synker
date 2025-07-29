use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub mime_type: String,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub owner_id: Uuid,
    pub is_directory: bool,
    pub parent_id: Option<Uuid>,
    pub permissions: FilePermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePermissions {
    pub read: bool,
    pub write: bool,
    pub delete: bool,
    pub share: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: String,
    pub device_name: String,
    pub last_sync: DateTime<Utc>,
    pub sync_folders: Vec<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLink {
    pub id: Uuid,
    pub file_id: Uuid,
    pub created_by: Uuid,
    pub share_token: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub password_protected: bool,
    pub download_count: u32,
    pub max_downloads: Option<u32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub path: String,
    pub overwrite: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub file_id: Uuid,
    pub path: String,
    pub size: u64,
    pub checksum: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub folders: Vec<String>,
    pub last_sync: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub changes: Vec<FileChange>,
    pub sync_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub file_id: Uuid,
    pub change_type: ChangeType,
    pub path: String,
    pub metadata: Option<FileMetadata>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Moved,
}
