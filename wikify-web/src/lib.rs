//! Wikify Web Server
//!
//! This module provides a web interface for Wikify, similar to DeepWiki's architecture.

pub mod handlers;
pub mod routes;
pub mod server;
pub mod state;
pub mod templates;
pub mod websocket;

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
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_origin("http://127.0.0.1:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_credentials(true)
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    // Create the main router
    Router::new()
        // API routes
        .nest("/api", routes::api_routes())
        // WebSocket routes
        .nest("/ws", routes::websocket_routes())
        // Static file serving
        .nest("/static", routes::static_routes())
        // Frontend routes (SPA fallback)
        .fallback(handlers::spa_fallback)
        // Add middleware
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
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            dev_mode: false,
            static_dir: None,
            database_url: None,
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
        }
    }

    /// Get the server address
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
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

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),
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
