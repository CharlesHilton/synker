use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;
use crate::types::*;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Self { pool })
    }

    pub async fn create_user(&self, user: &User) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, password_hash, created_at, last_login, is_active, permissions)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            user.id,
            user.username,
            user.email,
            user.password_hash,
            user.created_at,
            user.last_login,
            user.is_active,
            serde_json::to_string(&user.permissions)?
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query!(
            "SELECT * FROM users WHERE username = ?1",
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let permissions: Vec<String> = serde_json::from_str(&row.permissions)?;
            
            Ok(Some(User {
                id: row.id,
                username: row.username,
                email: row.email,
                password_hash: row.password_hash,
                created_at: row.created_at,
                last_login: row.last_login,
                is_active: row.is_active,
                permissions,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_last_login(&self, user_id: Uuid, last_login: DateTime<Utc>) -> Result<()> {
        sqlx::query!(
            "UPDATE users SET last_login = ?1 WHERE id = ?2",
            last_login,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn create_file_metadata(&self, metadata: &FileMetadata) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO file_metadata 
            (id, name, path, size, mime_type, checksum, created_at, modified_at, owner_id, is_directory, parent_id, permissions)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            metadata.id,
            metadata.name,
            metadata.path,
            metadata.size as i64,
            metadata.mime_type,
            metadata.checksum,
            metadata.created_at,
            metadata.modified_at,
            metadata.owner_id,
            metadata.is_directory,
            metadata.parent_id,
            serde_json::to_string(&metadata.permissions)?
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_file_metadata(&self, file_id: Uuid) -> Result<Option<FileMetadata>> {
        let row = sqlx::query!(
            "SELECT * FROM file_metadata WHERE id = ?1",
            file_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let permissions: FilePermissions = serde_json::from_str(&row.permissions)?;
            
            Ok(Some(FileMetadata {
                id: row.id,
                name: row.name,
                path: row.path,
                size: row.size as u64,
                mime_type: row.mime_type,
                checksum: row.checksum,
                created_at: row.created_at,
                modified_at: row.modified_at,
                owner_id: row.owner_id,
                is_directory: row.is_directory,
                parent_id: row.parent_id,
                permissions,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_files_in_directory(&self, parent_id: Option<Uuid>, owner_id: Uuid) -> Result<Vec<FileMetadata>> {
        let rows = sqlx::query!(
            "SELECT * FROM file_metadata WHERE parent_id = ?1 AND owner_id = ?2 ORDER BY name",
            parent_id,
            owner_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut files = Vec::new();
        for row in rows {
            let permissions: FilePermissions = serde_json::from_str(&row.permissions)?;
            
            files.push(FileMetadata {
                id: row.id,
                name: row.name,
                path: row.path,
                size: row.size as u64,
                mime_type: row.mime_type,
                checksum: row.checksum,
                created_at: row.created_at,
                modified_at: row.modified_at,
                owner_id: row.owner_id,
                is_directory: row.is_directory,
                parent_id: row.parent_id,
                permissions,
            });
        }

        Ok(files)
    }

    pub async fn create_sync_session(&self, session: &SyncSession) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO sync_sessions (id, user_id, device_id, device_name, last_sync, sync_folders, is_active)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            session.id,
            session.user_id,
            session.device_id,
            session.device_name,
            session.last_sync,
            serde_json::to_string(&session.sync_folders)?,
            session.is_active
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_sync_session(&self, user_id: Uuid, device_id: &str) -> Result<Option<SyncSession>> {
        let row = sqlx::query!(
            "SELECT * FROM sync_sessions WHERE user_id = ?1 AND device_id = ?2",
            user_id,
            device_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let sync_folders: Vec<String> = serde_json::from_str(&row.sync_folders)?;
            
            Ok(Some(SyncSession {
                id: row.id,
                user_id: row.user_id,
                device_id: row.device_id,
                device_name: row.device_name,
                last_sync: row.last_sync,
                sync_folders,
                is_active: row.is_active,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_sync_session(&self, session: &SyncSession) -> Result<()> {
        sqlx::query!(
            "UPDATE sync_sessions SET last_sync = ?1, sync_folders = ?2, is_active = ?3 WHERE id = ?4",
            session.last_sync,
            serde_json::to_string(&session.sync_folders)?,
            session.is_active,
            session.id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn create_share_link(&self, share_link: &ShareLink) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO share_links 
            (id, file_id, created_by, share_token, expires_at, password_protected, download_count, max_downloads, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            share_link.id,
            share_link.file_id,
            share_link.created_by,
            share_link.share_token,
            share_link.expires_at,
            share_link.password_protected,
            share_link.download_count as i32,
            share_link.max_downloads.map(|x| x as i32),
            share_link.created_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_share_link_by_token(&self, token: &str) -> Result<Option<ShareLink>> {
        let row = sqlx::query!(
            "SELECT * FROM share_links WHERE share_token = ?1",
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(ShareLink {
                id: row.id,
                file_id: row.file_id,
                created_by: row.created_by,
                share_token: row.share_token,
                expires_at: row.expires_at,
                password_protected: row.password_protected,
                download_count: row.download_count as u32,
                max_downloads: row.max_downloads.map(|x| x as u32),
                created_at: row.created_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_files_changed_since(&self, user_id: Uuid, since: DateTime<Utc>) -> Result<Vec<FileChange>> {
        let rows = sqlx::query!(
            r#"
            SELECT fm.*, 'Modified' as change_type 
            FROM file_metadata fm 
            WHERE fm.owner_id = ?1 AND fm.modified_at > ?2
            ORDER BY fm.modified_at
            "#,
            user_id,
            since
        )
        .fetch_all(&self.pool)
        .await?;

        let mut changes = Vec::new();
        for row in rows {
            let permissions: FilePermissions = serde_json::from_str(&row.permissions)?;
            
            let metadata = FileMetadata {
                id: row.id,
                name: row.name,
                path: row.path.clone(),
                size: row.size as u64,
                mime_type: row.mime_type,
                checksum: row.checksum,
                created_at: row.created_at,
                modified_at: row.modified_at,
                owner_id: row.owner_id,
                is_directory: row.is_directory,
                parent_id: row.parent_id,
                permissions,
            };

            changes.push(FileChange {
                file_id: row.id,
                change_type: ChangeType::Modified,
                path: row.path,
                metadata: Some(metadata),
                timestamp: row.modified_at,
            });
        }

        Ok(changes)
    }
}
