//! Route definitions for the Wikify web server
//!
//! This module defines all the routes for the web application.

use crate::{handlers, websocket, AppState};
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

/// Create API routes
pub fn api_routes() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        // Repository management
        .route("/repositories", post(handlers::initialize_repository))
        .route(
            "/repositories/{session_id}",
            get(handlers::get_repository_info),
        )
        // RAG endpoints
        .route("/chat", post(handlers::chat_query))
        .route("/chat/stream", post(handlers::chat_stream))
        // Wiki generation
        .route("/wiki/generate", post(handlers::generate_wiki))
        .route("/wiki/{session_id}", get(handlers::get_wiki))
        .route("/wiki/{session_id}/export", post(handlers::export_wiki))
        // File operations
        .route("/files/tree", post(handlers::get_file_tree))
        .route("/files/content", post(handlers::get_file_content))
        // Configuration
        .route("/config", get(handlers::get_config))
        .route("/config", post(handlers::update_config))
}

/// Create WebSocket routes
pub fn websocket_routes() -> Router<AppState> {
    Router::new()
        // Main chat WebSocket
        .route("/chat", get(websocket::chat_handler))
        // Wiki generation WebSocket (for progress updates)
        .route("/wiki", get(websocket::wiki_handler))
        // Repository indexing WebSocket (for progress updates)
        .route("/index", get(websocket::index_handler))
}

/// Create static file routes
pub fn static_routes() -> Router<AppState> {
    // In production, you might want to serve static files differently
    Router::new().nest_service("/assets", ServeDir::new("wikify-web/static"))
}

/// Create all routes combined
pub fn all_routes() -> Router<AppState> {
    Router::new()
        .nest("/api", api_routes())
        .nest("/ws", websocket_routes())
        .merge(static_routes())
        .fallback(handlers::spa_fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppState, WebConfig};
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check_route() {
        let state = AppState::new(WebConfig::default()).await.unwrap();
        let app = api_routes().with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
