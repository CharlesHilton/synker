use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
    pub filesystem: FilesystemSettings,
    pub auth: AuthSettings,
    pub mycloud: MyCloudSettings,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub request_timeout_seconds: u64,
    pub max_request_size: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseSettings {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FilesystemSettings {
    pub base_path: PathBuf,
    pub max_file_size_mb: u64,
    pub allowed_extensions: Vec<String>,
    pub temp_directory: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthSettings {
    pub jwt_secret: String,
    pub token_expiry_hours: i64,
    pub bcrypt_cost: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MyCloudSettings {
    pub api_endpoint: String,
    pub admin_username: String,
    pub admin_password: String,
    pub verify_ssl: bool,
    pub sync_interval_seconds: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: ServerSettings {
                host: "0.0.0.0".to_string(),
                port: 8080,
                max_connections: 1000,
                request_timeout_seconds: 30,
                max_request_size: 100 * 1024 * 1024, // 100MB
            },
            database: DatabaseSettings {
                url: "sqlite:./synker.db".to_string(),
                max_connections: 10,
                connection_timeout_seconds: 30,
            },
            filesystem: FilesystemSettings {
                base_path: PathBuf::from("./storage"),
                max_file_size_mb: 1024, // 1GB
                allowed_extensions: vec![
                    // Documents
                    "txt".to_string(), "pdf".to_string(), "doc".to_string(), "docx".to_string(),
                    "xls".to_string(), "xlsx".to_string(), "ppt".to_string(), "pptx".to_string(),
                    // Images
                    "jpg".to_string(), "jpeg".to_string(), "png".to_string(), "gif".to_string(),
                    "bmp".to_string(), "svg".to_string(), "webp".to_string(),
                    // Videos
                    "mp4".to_string(), "avi".to_string(), "mkv".to_string(), "mov".to_string(),
                    "wmv".to_string(), "flv".to_string(), "webm".to_string(),
                    // Audio
                    "mp3".to_string(), "wav".to_string(), "flac".to_string(), "aac".to_string(),
                    "ogg".to_string(), "wma".to_string(),
                    // Archives
                    "zip".to_string(), "rar".to_string(), "7z".to_string(), "tar".to_string(),
                    "gz".to_string(), "bz2".to_string(),
                ],
                temp_directory: PathBuf::from("./temp"),
            },
            auth: AuthSettings {
                jwt_secret: "your-super-secret-jwt-key-change-this-in-production".to_string(),
                token_expiry_hours: 24,
                bcrypt_cost: 12,
            },
            mycloud: MyCloudSettings {
                api_endpoint: "http://192.168.1.100".to_string(),
                admin_username: "admin".to_string(),
                admin_password: "".to_string(),
                verify_ssl: false,
                sync_interval_seconds: 300, // 5 minutes
            },
        }
    }
}

impl ServerConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = std::env::var("SYNKER_CONFIG")
            .unwrap_or_else(|_| "config.toml".to_string());

        if std::path::Path::new(&config_path).exists() {
            let config_str = std::fs::read_to_string(&config_path)?;
            let config: ServerConfig = toml::from_str(&config_str)?;
            Ok(config)
        } else {
            // Create default config file
            let default_config = Self::default();
            let config_str = toml::to_string_pretty(&default_config)?;
            std::fs::write(&config_path, config_str)?;
            
            println!("Created default configuration file at: {}", config_path);
            println!("Please edit the configuration file and restart the server.");
            
            Ok(default_config)
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate server settings
        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }

        // Validate auth settings
        if self.auth.jwt_secret.len() < 32 {
            return Err(anyhow::anyhow!("JWT secret must be at least 32 characters long"));
        }

        // Validate filesystem settings
        if !self.filesystem.base_path.is_absolute() {
            return Err(anyhow::anyhow!("Filesystem base path must be absolute"));
        }

        // Validate MyCloud settings
        if self.mycloud.admin_username.is_empty() {
            return Err(anyhow::anyhow!("MyCloud admin username cannot be empty"));
        }

        if self.mycloud.admin_password.is_empty() {
            return Err(anyhow::anyhow!("MyCloud admin password cannot be empty"));
        }

        Ok(())
    }
}
