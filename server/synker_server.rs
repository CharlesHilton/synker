// Synker Server - Self-hosted cloud storage server for MyCloud OS5
// This provides OneDrive-like functionality with MyCloud authentication

mod types;
mod database;
mod auth;
mod filesystem;
mod handlers;
mod config;
mod mycloud;

use axum::{
    extract::DefaultBodyLimit,
    http::{StatusCode, Method},
    middleware,
    routing::{get, post, delete, put},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    limit::RequestBodyLimitLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use clap::Parser;
use anyhow::Result;
use std::sync::Arc;

use crate::{
    auth::{AuthService, auth_middleware},
    database::Database,
    filesystem::FileSystemService,
    config::ServerConfig,
    mycloud::{MyCloudIntegration, MyCloudSyncService},
    handlers::*,
};

#[derive(Parser, Debug)]
#[command(name = "synker-server")]
#[command(about = "Self-hosted cloud storage server for MyCloud OS5")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
    
    /// Initialize database and exit
    #[arg(long)]
    init_db: bool,
    
    /// Create initial admin user
    #[arg(long)]
    create_admin: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub filesystem: FileSystemService,
    pub auth_service: AuthService,
    pub mycloud: Arc<MyCloudIntegration>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("synker_server={},tower_http=debug", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = ServerConfig::load()?;
    config.validate()?;

    tracing::info!("Starting Synker Server v0.1.0");
    tracing::info!("Configuration loaded from: {}", args.config);

    // Initialize database
    let database = Database::new(&config.database.url).await?;
    tracing::info!("Database connected: {}", config.database.url);

    if args.init_db {
        tracing::info!("Database initialized successfully");
        return Ok(());
    }

    // Initialize filesystem service
    let filesystem = FileSystemService::new(
        &config.filesystem.base_path,
        config.filesystem.max_file_size_mb * 1024 * 1024, // Convert MB to bytes
    )?;
    tracing::info!("Filesystem service initialized: {:?}", config.filesystem.base_path);

    // Initialize auth service
    let auth_service = AuthService::new(&config.auth.jwt_secret);
    tracing::info!("Authentication service initialized");

    // Initialize MyCloud integration
    let mycloud = Arc::new(MyCloudIntegration::new(config.mycloud.clone()));
    tracing::info!("MyCloud integration initialized");

    // Create admin user if requested
    if args.create_admin {
        create_initial_admin(&database, &auth_service, &config).await?;
        return Ok(());
    }

    // Create app state
    let app_state = AppState {
        database,
        filesystem,
        auth_service: auth_service.clone(),
        mycloud,
    };

    // Start MyCloud sync service in background
    let mycloud_sync_config = config.mycloud.clone();
    tokio::spawn(async move {
        let mut sync_service = MyCloudSyncService::new(mycloud_sync_config);
        if let Err(e) = sync_service.start().await {
            tracing::error!("MyCloud sync service error: {}", e);
        }
    });

    // Build application router
    let app = create_router(app_state, &config);

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn create_router(state: AppState, config: &ServerConfig) -> Router {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/", get(get_server_info))
        .route("/health", get(health_check))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/share/:token", get(download_shared_file));

    // Protected routes (authentication required)
    let protected_routes = Router::new()
        .route("/api/v1/files/upload", post(upload_file))
        .route("/api/v1/files/download/*path", get(download_file))
        .route("/api/v1/files/list", get(list_files))
        .route("/api/v1/files/delete/*path", delete(delete_file))
        .route("/api/v1/folders/create", post(create_folder))
        .route("/api/v1/sync", post(sync_files))
        .route("/api/v1/share/:file_id", post(create_share_link))
        .route("/api/v1/user/profile", get(get_user_profile))
        .route("/api/v1/user/storage", get(get_storage_info))
        .layer(middleware::from_fn_with_state(
            state.auth_service.clone(),
            auth_middleware,
        ));

    // Combine routes
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                        .allow_headers(Any),
                )
                .layer(DefaultBodyLimit::max(config.server.max_request_size))
                .layer(RequestBodyLimitLayer::new(config.server.max_request_size)),
        )
        .with_state(state);

    app
}

async fn health_check() -> &'static str {
    "OK"
}

async fn download_shared_file() -> Result<String, StatusCode> {
    // TODO: Implement shared file download
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn get_user_profile() -> Result<String, StatusCode> {
    // TODO: Implement user profile endpoint
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn get_storage_info() -> Result<String, StatusCode> {
    // TODO: Implement storage info endpoint
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn create_initial_admin(
    database: &Database,
    auth_service: &AuthService,
    config: &ServerConfig,
) -> Result<()> {
    use crate::types::User;
    use uuid::Uuid;
    use chrono::Utc;

    let username = &config.mycloud.admin_username;
    let password = &config.mycloud.admin_password;

    // Check if admin user already exists
    if let Ok(Some(_)) = database.get_user_by_username(username).await {
        tracing::warn!("Admin user '{}' already exists", username);
        return Ok(());
    }

    let password_hash = auth_service.hash_password(password)?;

    let admin_user = User {
        id: Uuid::new_v4(),
        username: username.clone(),
        email: Some(format!("{}@localhost", username)),
        password_hash,
        created_at: Utc::now(),
        last_login: None,
        is_active: true,
        permissions: vec![
            "read".to_string(),
            "write".to_string(),
            "delete".to_string(),
            "share".to_string(),
            "admin".to_string(),
        ],
    };

    database.create_user(&admin_user).await?;
    tracing::info!("Created initial admin user: {}", username);

    Ok(())
}

