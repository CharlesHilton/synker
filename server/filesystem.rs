use std::path::{Path, PathBuf};
use std::fs::{self, Metadata};
use std::io::{self, Read, Write};
use tokio::fs as async_fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use anyhow::{Result, anyhow};
use walkdir::WalkDir;
use mime_guess::from_path;
use notify::{Watcher, RecursiveMode, watcher, DebouncedEvent};
use std::sync::mpsc;
use std::time::Duration;
use crate::types::{FileMetadata, FilePermissions, FileChange, ChangeType};

pub struct FileSystemService {
    base_path: PathBuf,
    max_file_size: u64,
}

impl FileSystemService {
    pub fn new(base_path: impl AsRef<Path>, max_file_size: u64) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        // Create base directory if it doesn't exist
        if !base_path.exists() {
            fs::create_dir_all(&base_path)?;
        }

        Ok(Self {
            base_path,
            max_file_size,
        })
    }

    pub fn get_absolute_path(&self, relative_path: &str) -> PathBuf {
        let cleaned_path = relative_path.trim_start_matches('/');
        self.base_path.join(cleaned_path)
    }

    pub fn get_relative_path(&self, absolute_path: &Path) -> Result<String> {
        let relative = absolute_path.strip_prefix(&self.base_path)?;
        Ok(format!("/{}", relative.to_string_lossy()))
    }

    pub async fn save_file(&self, relative_path: &str, data: &[u8]) -> Result<FileMetadata> {
        if data.len() as u64 > self.max_file_size {
            return Err(anyhow!("File size exceeds maximum allowed size"));
        }

        let absolute_path = self.get_absolute_path(relative_path);
        
        // Create parent directories if they don't exist
        if let Some(parent) = absolute_path.parent() {
            async_fs::create_dir_all(parent).await?;
        }

        // Write file
        async_fs::write(&absolute_path, data).await?;

        // Generate metadata
        let metadata = self.generate_file_metadata(&absolute_path, Uuid::new_v4()).await?;
        Ok(metadata)
    }

    pub async fn read_file(&self, relative_path: &str) -> Result<Vec<u8>> {
        let absolute_path = self.get_absolute_path(relative_path);
        
        if !absolute_path.exists() {
            return Err(anyhow!("File not found"));
        }

        let data = async_fs::read(absolute_path).await?;
        Ok(data)
    }

    pub async fn delete_file(&self, relative_path: &str) -> Result<()> {
        let absolute_path = self.get_absolute_path(relative_path);
        
        if !absolute_path.exists() {
            return Err(anyhow!("File not found"));
        }

        if absolute_path.is_dir() {
            async_fs::remove_dir_all(absolute_path).await?;
        } else {
            async_fs::remove_file(absolute_path).await?;
        }

        Ok(())
    }

    pub async fn create_directory(&self, relative_path: &str) -> Result<FileMetadata> {
        let absolute_path = self.get_absolute_path(relative_path);
        
        async_fs::create_dir_all(&absolute_path).await?;
        
        let metadata = self.generate_file_metadata(&absolute_path, Uuid::new_v4()).await?;
        Ok(metadata)
    }

    pub async fn move_file(&self, old_path: &str, new_path: &str) -> Result<()> {
        let old_absolute = self.get_absolute_path(old_path);
        let new_absolute = self.get_absolute_path(new_path);
        
        if !old_absolute.exists() {
            return Err(anyhow!("Source file not found"));
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = new_absolute.parent() {
            async_fs::create_dir_all(parent).await?;
        }

        async_fs::rename(old_absolute, new_absolute).await?;
        Ok(())
    }

    pub async fn list_directory(&self, relative_path: &str) -> Result<Vec<FileMetadata>> {
        let absolute_path = self.get_absolute_path(relative_path);
        
        if !absolute_path.exists() || !absolute_path.is_dir() {
            return Err(anyhow!("Directory not found"));
        }

        let mut entries = Vec::new();
        let mut dir_entries = async_fs::read_dir(absolute_path).await?;
        
        while let Some(entry) = dir_entries.next_entry().await? {
            let metadata = self.generate_file_metadata(&entry.path(), Uuid::new_v4()).await?;
            entries.push(metadata);
        }

        entries.sort_by(|a, b| {
            // Sort directories first, then by name
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(entries)
    }

    pub async fn get_file_metadata(&self, relative_path: &str) -> Result<FileMetadata> {
        let absolute_path = self.get_absolute_path(relative_path);
        
        if !absolute_path.exists() {
            return Err(anyhow!("File not found"));
        }

        let metadata = self.generate_file_metadata(&absolute_path, Uuid::new_v4()).await?;
        Ok(metadata)
    }

    async fn generate_file_metadata(&self, path: &Path, owner_id: Uuid) -> Result<FileMetadata> {
        let std_metadata = async_fs::metadata(path).await?;
        let relative_path = self.get_relative_path(path)?;
        
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let is_directory = std_metadata.is_dir();
        let size = if is_directory { 0 } else { std_metadata.len() };
        
        let mime_type = if is_directory {
            "inode/directory".to_string()
        } else {
            from_path(path).first_or_octet_stream().to_string()
        };

        let checksum = if is_directory {
            String::new()
        } else {
            self.calculate_checksum(path).await?
        };

        let created_at = std_metadata
            .created()
            .map(|t| DateTime::<Utc>::from(t))
            .unwrap_or_else(|_| Utc::now());

        let modified_at = std_metadata
            .modified()
            .map(|t| DateTime::<Utc>::from(t))
            .unwrap_or_else(|_| Utc::now());

        Ok(FileMetadata {
            id: Uuid::new_v4(),
            name,
            path: relative_path,
            size,
            mime_type,
            checksum,
            created_at,
            modified_at,
            owner_id,
            is_directory,
            parent_id: None, // This would need to be set by the caller
            permissions: FilePermissions {
                read: true,
                write: true,
                delete: true,
                share: true,
            },
        })
    }

    async fn calculate_checksum(&self, path: &Path) -> Result<String> {
        let mut file = async_fs::File::open(path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn watch_directory(&self, relative_path: &str) -> Result<mpsc::Receiver<DebouncedEvent>> {
        let absolute_path = self.get_absolute_path(relative_path);
        let (tx, rx) = mpsc::channel();
        
        let mut watcher = watcher(tx, Duration::from_secs(1))?;
        watcher.watch(&absolute_path, RecursiveMode::Recursive)?;
        
        // Keep watcher alive by moving it into a thread
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        });

        Ok(rx)
    }

    pub async fn get_directory_size(&self, relative_path: &str) -> Result<u64> {
        let absolute_path = self.get_absolute_path(relative_path);
        
        if !absolute_path.exists() {
            return Err(anyhow!("Directory not found"));
        }

        let mut total_size = 0u64;
        
        for entry in WalkDir::new(&absolute_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total_size += entry.metadata()?.len();
            }
        }

        Ok(total_size)
    }

    pub async fn copy_file(&self, source_path: &str, dest_path: &str) -> Result<FileMetadata> {
        let source_absolute = self.get_absolute_path(source_path);
        let dest_absolute = self.get_absolute_path(dest_path);
        
        if !source_absolute.exists() {
            return Err(anyhow!("Source file not found"));
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = dest_absolute.parent() {
            async_fs::create_dir_all(parent).await?;
        }

        async_fs::copy(&source_absolute, &dest_absolute).await?;
        
        let metadata = self.generate_file_metadata(&dest_absolute, Uuid::new_v4()).await?;
        Ok(metadata)
    }

    pub fn get_available_space(&self) -> Result<u64> {
        // This is a simplified implementation
        // In a real implementation, you'd use platform-specific APIs
        // to get actual disk space information
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = fs::metadata(&self.base_path)?;
            // This is not accurate - you'd need to use statvfs or similar
            Ok(u64::MAX) // Placeholder
        }
        
        #[cfg(windows)]
        {
            // Use GetDiskFreeSpaceEx on Windows
            Ok(u64::MAX) // Placeholder
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = tempdir().unwrap();
        let fs_service = FileSystemService::new(temp_dir.path(), 1024 * 1024).unwrap();
        
        // Test saving a file
        let test_data = b"Hello, World!";
        let metadata = fs_service.save_file("/test.txt", test_data).await.unwrap();
        assert_eq!(metadata.name, "test.txt");
        assert_eq!(metadata.size, test_data.len() as u64);
        
        // Test reading the file
        let read_data = fs_service.read_file("/test.txt").await.unwrap();
        assert_eq!(read_data, test_data);
        
        // Test creating directory
        let dir_metadata = fs_service.create_directory("/testdir").await.unwrap();
        assert!(dir_metadata.is_directory);
        
        // Test listing directory
        let entries = fs_service.list_directory("/").await.unwrap();
        assert!(entries.len() >= 2); // test.txt and testdir
    }
}
