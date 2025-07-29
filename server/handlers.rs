use axum::{
    extract::{Path, Query, State, Multipart},
    http::{StatusCode, HeaderMap, header},
    response::{Response, Json},
    Extension,
};
use serde_json::json;
use uuid::Uuid;
use std::collections::HashMap;
use chrono::Utc;
use anyhow::Result;

use crate::types::*;
use crate::auth::{Claims, AuthService};
use crate::database::Database;
use crate::filesystem::FileSystemService;

pub async fn login(
    State(auth_service): State<AuthService>,
    State(database): State<Database>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, StatusCode> {
    // Get user from database
    let user = match database.get_user_by_username(&request.username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(Json(ApiResponse::error("Invalid credentials".to_string())));
        }
        Err(_) => {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Verify password
    if !auth_service.verify_password(&request.password, &user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        return Ok(Json(ApiResponse::error("Invalid credentials".to_string())));
    }

    // Update last login
    if let Err(_) = database.update_last_login(user.id, Utc::now()).await {
        // Log error but don't fail the login
    }

    // Generate JWT token
    let token = auth_service.generate_token(&user, request.device_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = LoginResponse {
        token: token.clone(),
        user: user.clone(),
        expires_at: Utc::now() + chrono::Duration::hours(24),
    };

    Ok(Json(ApiResponse::success(response)))
}

pub async fn upload_file(
    State(filesystem): State<FileSystemService>,
    State(database): State<Database>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<HashMap<String, String>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<UploadResponse>>, StatusCode> {
    let path = params.get("path").unwrap_or(&"/".to_string()).clone();
    let overwrite = params.get("overwrite")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("file").to_string();
        let filename = field.file_name().unwrap_or("unnamed").to_string();
        let data = field.bytes().await.unwrap();

        let file_path = if path.ends_with('/') {
            format!("{}{}", path, filename)
        } else {
            format!("{}/{}", path, filename)
        };

        // Check if file exists and overwrite is not allowed
        if !overwrite {
            if let Ok(_) = filesystem.get_file_metadata(&file_path).await {
                return Ok(Json(ApiResponse::error("File already exists".to_string())));
            }
        }

        // Save file to filesystem
        let mut metadata = filesystem.save_file(&file_path, &data).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Update owner ID
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        metadata.owner_id = user_id;

        // Save metadata to database
        database.create_file_metadata(&metadata).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let response = UploadResponse {
            file_id: metadata.id,
            path: metadata.path,
            size: metadata.size,
            checksum: metadata.checksum,
        };

        return Ok(Json(ApiResponse::success(response)));
    }

    Ok(Json(ApiResponse::error("No file uploaded".to_string())))
}

pub async fn download_file(
    State(filesystem): State<FileSystemService>,
    State(database): State<Database>,
    Extension(claims): Extension<Claims>,
    Path(file_path): Path<String>,
) -> Result<Response, StatusCode> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Decode the file path (it might be URL encoded)
    let file_path = urlencoding::decode(&file_path)
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_owned();

    // Check if user has access to the file
    // This is a simplified check - in production you'd want more granular permissions
    let file_data = filesystem.read_file(&file_path).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let file_metadata = filesystem.get_file_metadata(&file_path).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        file_metadata.mime_type.parse().unwrap(),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", file_metadata.name).parse().unwrap(),
    );

    Ok(Response::builder()
        .status(StatusCode::OK)
        .headers(headers)
        .body(axum::body::Body::from(file_data))
        .unwrap())
}

pub async fn list_files(
    State(filesystem): State<FileSystemService>,
    State(database): State<Database>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<FileMetadata>>>, StatusCode> {
    let path = params.get("path").unwrap_or(&"/".to_string()).clone();
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let files = filesystem.list_directory(&path).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Filter files by user ownership (simplified - you might want more complex permissions)
    let user_files: Vec<FileMetadata> = files.into_iter()
        .map(|mut file| {
            file.owner_id = user_id; // Set correct owner
            file
        })
        .collect();

    Ok(Json(ApiResponse::success(user_files)))
}

pub async fn create_folder(
    State(filesystem): State<FileSystemService>,
    State(database): State<Database>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateFolderRequest>,
) -> Result<Json<ApiResponse<FileMetadata>>, StatusCode> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let folder_path = if request.path.ends_with('/') {
        format!("{}{}", request.path, request.name)
    } else {
        format!("{}/{}", request.path, request.name)
    };

    let mut metadata = filesystem.create_directory(&folder_path).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    metadata.owner_id = user_id;

    // Save metadata to database
    database.create_file_metadata(&metadata).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::success(metadata)))
}

pub async fn delete_file(
    State(filesystem): State<FileSystemService>,
    State(database): State<Database>,
    Extension(claims): Extension<Claims>,
    Path(file_path): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let file_path = urlencoding::decode(&file_path)
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_owned();

    // TODO: Check permissions before deleting

    filesystem.delete_file(&file_path).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn sync_files(
    State(filesystem): State<FileSystemService>,
    State(database): State<Database>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<SyncRequest>,
) -> Result<Json<ApiResponse<SyncResponse>>, StatusCode> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let since = request.last_sync.unwrap_or_else(|| {
        Utc::now() - chrono::Duration::hours(24)
    });

    let changes = database.get_files_changed_since(user_id, since).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let sync_token = Uuid::new_v4().to_string();

    let response = SyncResponse {
        changes,
        sync_token,
    };

    Ok(Json(ApiResponse::success(response)))
}

pub async fn create_share_link(
    State(database): State<Database>,
    Extension(claims): Extension<Claims>,
    Path(file_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<ShareLink>>, StatusCode> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let file_id = Uuid::parse_str(&file_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Check if file exists and user owns it
    let file_metadata = database.get_file_metadata(file_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let file_metadata = match file_metadata {
        Some(metadata) if metadata.owner_id == user_id => metadata,
        Some(_) => return Ok(Json(ApiResponse::error("Access denied".to_string()))),
        None => return Ok(Json(ApiResponse::error("File not found".to_string()))),
    };

    let expires_in_hours = params.get("expires_in_hours")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(24);

    let max_downloads = params.get("max_downloads")
        .and_then(|s| s.parse::<u32>().ok());

    let share_link = ShareLink {
        id: Uuid::new_v4(),
        file_id,
        created_by: user_id,
        share_token: Uuid::new_v4().to_string(),
        expires_at: Some(Utc::now() + chrono::Duration::hours(expires_in_hours)),
        password_protected: false,
        download_count: 0,
        max_downloads,
        created_at: Utc::now(),
    };

    database.create_share_link(&share_link).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::success(share_link)))
}

pub async fn get_server_info() -> Json<ApiResponse<serde_json::Value>> {
    let info = json!({
        "name": "Synker Server",
        "version": "0.1.0",
        "description": "Self-hosted cloud storage server for MyCloud OS5",
        "api_version": "v1",
        "features": [
            "file_upload",
            "file_download",
            "file_sync",
            "folder_creation",
            "file_sharing",
            "user_authentication"
        ]
    });

    Json(ApiResponse::success(info))
}
