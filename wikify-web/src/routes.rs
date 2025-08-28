//! Route definitions for the Wikify web server
//!
//! This module defines all the routes for the web application.

use crate::{handlers, openapi, websocket, AppState};
use axum::{
    response::Json,
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Create API routes
pub fn api_routes() -> Router<AppState> {
    let router = Router::new()
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
        .route("/config", post(handlers::update_config));

    // Add database-specific routes if SQLite feature is enabled
    #[cfg(feature = "sqlite")]
    let router = router
        .route("/repositories", get(handlers::get_repositories))
        .route("/sessions", get(handlers::get_sessions))
        .route("/history/{repository_id}", get(handlers::get_query_history));

    router
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

/// Create OpenAPI documentation routes
pub fn openapi_routes() -> Router<AppState> {
    Router::new()
        // OpenAPI specification endpoints
        .route("/openapi.json", get(get_openapi_json))
        .route("/openapi.yaml", get(get_openapi_yaml))
        // Swagger UI
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
}

/// Get OpenAPI specification as JSON
async fn get_openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi::ApiDoc::openapi())
}

/// Get OpenAPI specification as YAML
async fn get_openapi_yaml() -> String {
    openapi::get_openapi_yaml()
}

/// Create all routes combined
pub fn all_routes() -> Router<AppState> {
    Router::new()
        .nest("/api", api_routes())
        .nest("/ws", websocket_routes())
        .nest("/api-docs", openapi_routes())
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

    #[tokio::test]
    async fn test_openapi_json_route() {
        let state = AppState::new(WebConfig::default()).await.unwrap();
        let app = openapi_routes().with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/openapi.json")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
