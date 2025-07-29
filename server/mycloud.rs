use serde::{Deserialize, Serialize};
use reqwest::{Client, header::HeaderMap};
use anyhow::{Result, anyhow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::types::User;
use crate::config::MyCloudSettings;

#[derive(Debug, Serialize, Deserialize)]
pub struct MyCloudUser {
    pub username: String,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub groups: Vec<String>,
    pub is_admin: bool,
    pub is_active: bool,
    pub last_login: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MyCloudAuthResponse {
    pub success: bool,
    pub session_token: Option<String>,
    pub user: Option<MyCloudUser>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MyCloudShare {
    pub name: String,
    pub path: String,
    pub permissions: Vec<String>,
    pub accessible_by: Vec<String>,
}

pub struct MyCloudIntegration {
    client: Client,
    config: MyCloudSettings,
    session_token: Option<String>,
}

impl MyCloudIntegration {
    pub fn new(config: MyCloudSettings) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        
        let client = Client::builder()
            .default_headers(headers)
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()
            .unwrap();

        Self {
            client,
            config,
            session_token: None,
        }
    }

    pub async fn authenticate_admin(&mut self) -> Result<()> {
        let auth_url = format!("{}/api/2.1/rest/login", self.config.api_endpoint);
        
        let auth_request = serde_json::json!({
            "username": self.config.admin_username,
            "password": self.config.admin_password
        });

        let response = self.client
            .post(&auth_url)
            .json(&auth_request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to authenticate with MyCloud: {}", response.status()));
        }

        let auth_response: MyCloudAuthResponse = response.json().await?;
        
        if !auth_response.success {
            return Err(anyhow!("MyCloud authentication failed: {:?}", auth_response.error));
        }

        self.session_token = auth_response.session_token;
        Ok(())
    }

    pub async fn verify_user_credentials(&self, username: &str, password: &str) -> Result<Option<MyCloudUser>> {
        let auth_url = format!("{}/api/2.1/rest/login", self.config.api_endpoint);
        
        let auth_request = serde_json::json!({
            "username": username,
            "password": password
        });

        let response = self.client
            .post(&auth_url)
            .json(&auth_request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let auth_response: MyCloudAuthResponse = response.json().await?;
        
        if auth_response.success {
            Ok(auth_response.user)
        } else {
            Ok(None)
        }
    }

    pub async fn get_user_info(&self, username: &str) -> Result<Option<MyCloudUser>> {
        self.ensure_authenticated().await?;
        
        let user_url = format!("{}/api/2.1/rest/users/{}", self.config.api_endpoint, username);
        
        let response = self.client
            .get(&user_url)
            .header("Authorization", format!("Bearer {}", self.session_token.as_ref().unwrap()))
            .send()
            .await?;

        if response.status().is_success() {
            let user: MyCloudUser = response.json().await?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    pub async fn get_user_shares(&self, username: &str) -> Result<Vec<MyCloudShare>> {
        self.ensure_authenticated().await?;
        
        let shares_url = format!("{}/api/2.1/rest/users/{}/shares", self.config.api_endpoint, username);
        
        let response = self.client
            .get(&shares_url)
            .header("Authorization", format!("Bearer {}", self.session_token.as_ref().unwrap()))
            .send()
            .await?;

        if response.status().is_success() {
            let shares: Vec<MyCloudShare> = response.json().await?;
            Ok(shares)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn sync_user_to_local(&self, mycloud_user: &MyCloudUser, password_hash: &str) -> Result<User> {
        let user = User {
            id: Uuid::new_v4(),
            username: mycloud_user.username.clone(),
            email: mycloud_user.email.clone(),
            password_hash: password_hash.to_string(),
            created_at: Utc::now(),
            last_login: mycloud_user.last_login,
            is_active: mycloud_user.is_active,
            permissions: self.map_mycloud_permissions(&mycloud_user.groups),
        };

        Ok(user)
    }

    pub async fn check_user_permissions(&self, username: &str, resource: &str, action: &str) -> Result<bool> {
        self.ensure_authenticated().await?;
        
        let permissions_url = format!(
            "{}/api/2.1/rest/users/{}/permissions?resource={}&action={}",
            self.config.api_endpoint, username, resource, action
        );
        
        let response = self.client
            .get(&permissions_url)
            .header("Authorization", format!("Bearer {}", self.session_token.as_ref().unwrap()))
            .send()
            .await?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result.get("allowed").and_then(|v| v.as_bool()).unwrap_or(false))
        } else {
            Ok(false)
        }
    }

    async fn ensure_authenticated(&self) -> Result<()> {
        if self.session_token.is_none() {
            return Err(anyhow!("Not authenticated with MyCloud"));
        }
        Ok(())
    }

    fn map_mycloud_permissions(&self, groups: &[String]) -> Vec<String> {
        let mut permissions = Vec::new();
        
        for group in groups {
            match group.as_str() {
                "administrators" => {
                    permissions.extend_from_slice(&[
                        "read".to_string(),
                        "write".to_string(),
                        "delete".to_string(),
                        "share".to_string(),
                        "admin".to_string(),
                    ]);
                }
                "users" => {
                    permissions.extend_from_slice(&[
                        "read".to_string(),
                        "write".to_string(),
                        "share".to_string(),
                    ]);
                }
                "guests" => {
                    permissions.push("read".to_string());
                }
                _ => {
                    // Custom group permissions can be added here
                    permissions.push("read".to_string());
                }
            }
        }

        permissions.sort();
        permissions.dedup();
        permissions
    }

    pub async fn get_system_info(&self) -> Result<serde_json::Value> {
        self.ensure_authenticated().await?;
        
        let info_url = format!("{}/api/2.1/rest/system/info", self.config.api_endpoint);
        
        let response = self.client
            .get(&info_url)
            .header("Authorization", format!("Bearer {}", self.session_token.as_ref().unwrap()))
            .send()
            .await?;

        if response.status().is_success() {
            let info: serde_json::Value = response.json().await?;
            Ok(info)
        } else {
            Err(anyhow!("Failed to get system info: {}", response.status()))
        }
    }

    pub async fn monitor_shares(&self) -> Result<Vec<MyCloudShare>> {
        self.ensure_authenticated().await?;
        
        let shares_url = format!("{}/api/2.1/rest/shares", self.config.api_endpoint);
        
        let response = self.client
            .get(&shares_url)
            .header("Authorization", format!("Bearer {}", self.session_token.as_ref().unwrap()))
            .send()
            .await?;

        if response.status().is_success() {
            let shares: Vec<MyCloudShare> = response.json().await?;
            Ok(shares)
        } else {
            Err(anyhow!("Failed to get shares: {}", response.status()))
        }
    }
}

// Background service to periodically sync with MyCloud
pub struct MyCloudSyncService {
    integration: MyCloudIntegration,
    sync_interval: std::time::Duration,
}

impl MyCloudSyncService {
    pub fn new(config: MyCloudSettings) -> Self {
        let sync_interval = std::time::Duration::from_secs(config.sync_interval_seconds);
        let integration = MyCloudIntegration::new(config);

        Self {
            integration,
            sync_interval,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        // Authenticate with MyCloud
        self.integration.authenticate_admin().await?;
        
        // Start background sync loop
        loop {
            if let Err(e) = self.sync_cycle().await {
                eprintln!("MyCloud sync error: {}", e);
            }
            
            tokio::time::sleep(self.sync_interval).await;
        }
    }

    async fn sync_cycle(&mut self) -> Result<()> {
        // Re-authenticate if needed
        if self.integration.session_token.is_none() {
            self.integration.authenticate_admin().await?;
        }

        // Sync shares
        let shares = self.integration.monitor_shares().await?;
        println!("Synced {} shares from MyCloud", shares.len());

        // Additional sync operations can be added here
        // - User synchronization
        // - Permission updates
        // - System status checks

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mycloud_user_mapping() {
        let config = MyCloudSettings {
            api_endpoint: "http://localhost".to_string(),
            admin_username: "admin".to_string(),
            admin_password: "password".to_string(),
            verify_ssl: false,
            sync_interval_seconds: 300,
        };

        let integration = MyCloudIntegration::new(config);
        
        let mycloud_user = MyCloudUser {
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            full_name: Some("Test User".to_string()),
            groups: vec!["users".to_string()],
            is_admin: false,
            is_active: true,
            last_login: None,
        };

        let user = integration.sync_user_to_local(&mycloud_user, "password_hash").await.unwrap();
        
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, Some("test@example.com".to_string()));
        assert!(user.permissions.contains(&"read".to_string()));
        assert!(user.permissions.contains(&"write".to_string()));
    }
}
