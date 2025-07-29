use reqwest::{Client, multipart};
use serde_json::{json, Value};
use std::path::Path;
use anyhow::Result;

#[derive(Debug)]
pub struct SynkerClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl SynkerClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            token: None,
        }
    }

    pub async fn login(&mut self, username: &str, password: &str, device_id: Option<&str>) -> Result<()> {
        let login_data = json!({
            "username": username,
            "password": password,
            "device_id": device_id,
            "device_name": "Rust Client"
        });

        let response = self.client
            .post(&format!("{}/api/v1/auth/login", self.base_url))
            .json(&login_data)
            .send()
            .await?;

        let result: Value = response.json().await?;
        
        if result["success"].as_bool().unwrap_or(false) {
            self.token = Some(result["data"]["token"].as_str().unwrap().to_string());
            println!("Login successful!");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Login failed: {}", result["error"]))
        }
    }

    pub async fn upload_file(&self, file_path: &Path, remote_path: &str) -> Result<()> {
        let token = self.token.as_ref().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
        
        let file_content = tokio::fs::read(file_path).await?;
        let file_name = file_path.file_name().unwrap().to_str().unwrap();
        
        let form = multipart::Form::new()
            .part("file", multipart::Part::bytes(file_content).file_name(file_name.to_string()));

        let response = self.client
            .post(&format!("{}/api/v1/files/upload?path={}", self.base_url, remote_path))
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await?;

        let result: Value = response.json().await?;
        
        if result["success"].as_bool().unwrap_or(false) {
            println!("File uploaded successfully: {}", file_name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Upload failed: {}", result["error"]))
        }
    }

    pub async fn download_file(&self, remote_path: &str, local_path: &Path) -> Result<()> {
        let token = self.token.as_ref().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
        
        let response = self.client
            .get(&format!("{}/api/v1/files/download{}", self.base_url, remote_path))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if response.status().is_success() {
            let content = response.bytes().await?;
            tokio::fs::write(local_path, content).await?;
            println!("File downloaded successfully to: {:?}", local_path);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Download failed: {}", response.status()))
        }
    }

    pub async fn list_files(&self, path: &str) -> Result<Vec<Value>> {
        let token = self.token.as_ref().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
        
        let response = self.client
            .get(&format!("{}/api/v1/files/list?path={}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let result: Value = response.json().await?;
        
        if result["success"].as_bool().unwrap_or(false) {
            Ok(result["data"].as_array().unwrap().clone())
        } else {
            Err(anyhow::anyhow!("List files failed: {}", result["error"]))
        }
    }

    pub async fn create_folder(&self, path: &str, name: &str) -> Result<()> {
        let token = self.token.as_ref().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
        
        let folder_data = json!({
            "path": path,
            "name": name
        });

        let response = self.client
            .post(&format!("{}/api/v1/folders/create", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&folder_data)
            .send()
            .await?;

        let result: Value = response.json().await?;
        
        if result["success"].as_bool().unwrap_or(false) {
            println!("Folder created successfully: {}", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Create folder failed: {}", result["error"]))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Example usage
    let mut client = SynkerClient::new("http://localhost:8080");
    
    // Login
    client.login("your-username", "your-password", Some("rust-client-1")).await?;
    
    // List root directory
    let files = client.list_files("/").await?;
    println!("Files in root directory: {:#?}", files);
    
    // Create a folder
    client.create_folder("/", "test-folder").await?;
    
    // Upload a file (example)
    // client.upload_file(Path::new("./test.txt"), "/test-folder/").await?;
    
    // Download a file (example)
    // client.download_file("/test-folder/test.txt", Path::new("./downloaded-test.txt")).await?;
    
    Ok(())
}
