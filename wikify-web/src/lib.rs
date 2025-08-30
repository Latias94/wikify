//! Wikify Web Server
//!
//! This module provides a web interface for Wikify, similar to DeepWiki's architecture.

pub mod auth;
pub mod handlers;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod server;
pub mod state;
pub mod templates;
pub mod websocket;

// Database support (optional)
#[cfg(feature = "sqlite")]
pub mod simple_database;

// Re-export main types
pub use server::WikifyServer;
pub use state::AppState;

use axum::{
    extract::DefaultBodyLimit,
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Create the main application router
pub fn create_app(state: AppState) -> Router {
    // Configure CORS with environment variable support
    let cors = {
        // Get allowed origins from environment variable or use defaults
        let allowed_origins = std::env::var("WIKIFY_CORS_ORIGINS").unwrap_or_else(|_| {
            // Default origins for development (Vite default port)
            "http://localhost:5173,http://127.0.0.1:5173".to_string()
        });

        let cors_layer = CorsLayer::new()
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_credentials(true)
            .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

        // Parse and add each origin
        let origins: Vec<HeaderValue> = allowed_origins
            .split(',')
            .filter_map(|origin| {
                let origin = origin.trim();
                if origin.is_empty() {
                    return None;
                }
                match origin.parse::<HeaderValue>() {
                    Ok(header_value) => {
                        tracing::info!("Added CORS origin: {}", origin);
                        Some(header_value)
                    }
                    Err(e) => {
                        tracing::warn!("Invalid CORS origin format '{}': {}", origin, e);
                        None
                    }
                }
            })
            .collect();

        // Add all origins at once
        cors_layer.allow_origin(origins)
    };

    // Create the main router
    Router::new()
        // API routes
        .nest("/api", routes::api_routes(state.clone()))
        // WebSocket routes
        .nest("/ws", routes::websocket_routes())
        // OpenAPI documentation routes
        .nest("/api-docs", routes::openapi_routes())
        // Static file serving
        .nest("/static", routes::static_routes())
        // Frontend routes (SPA fallback)
        .fallback(handlers::spa_fallback)
        // Add middleware
        .layer(axum::middleware::from_fn(
            middleware::user_context_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB max body size
        .with_state(state)
}

/// Configuration for the web server
#[derive(Debug, Clone)]
pub struct WebConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Enable development mode
    pub dev_mode: bool,
    /// Static files directory
    pub static_dir: Option<String>,
    /// Database URL (optional)
    pub database_url: Option<String>,
    /// Permission mode (open, restricted, local)
    pub permission_mode: Option<String>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            dev_mode: false,
            static_dir: None,
            database_url: Some("sqlite:./data/wikify.db".to_string()), // 启用文件 SQLite 数据库
            permission_mode: Some("open".to_string()),                 // 默认开放模式
        }
    }
}

impl WebConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("WIKIFY_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("WIKIFY_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            dev_mode: std::env::var("WIKIFY_DEV_MODE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            static_dir: std::env::var("WIKIFY_STATIC_DIR").ok(),
            database_url: std::env::var("DATABASE_URL").ok(),
            permission_mode: std::env::var("WIKIFY_PERMISSION_MODE").ok(),
        }
    }

    /// Get the server address
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Get the database URL with path expansion
    pub fn database_url(&self) -> String {
        let url = self
            .database_url
            .as_ref()
            .unwrap_or(&"sqlite:~/.wikify/wikify.db".to_string())
            .clone();

        // 展开 ~ 路径
        if url.starts_with("sqlite:~/") {
            if let Some(home) = dirs::home_dir() {
                let path = url.strip_prefix("sqlite:~/").unwrap();
                let full_path = home.join(".wikify").join(path);

                // 确保目录存在
                if let Some(parent) = full_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                return format!("sqlite:{}", full_path.display());
            }
        }

        url
    }
}

/// Error types for the web server
#[derive(thiserror::Error, Debug)]
pub enum WebError {
    #[error("Server error: {0}")]
    Server(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Wiki generation error: {0}")]
    WikiGeneration(String),

    #[error("RAG query error: {0}")]
    RagQuery(String),

    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

/// Result type for web operations
pub type WebResult<T> = Result<T, WebError>;

/// Initialize logging for the web server
pub fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wikify_web=debug,tower_http=debug,axum=debug".into()),
        )
        .init();
}
