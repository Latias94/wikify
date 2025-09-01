//! Route definitions for the Wikify web server
//!
//! This module defines all the routes for the web application.

use crate::{auth, handlers, openapi, websocket, AppState};
use axum::{
    response::Json,
    routing::{delete, get, post},
    Router,
};
use tower_http::services::ServeDir;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Create API routes
pub fn api_routes(_state: AppState) -> Router<AppState> {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        // Authentication endpoints
        .route("/auth/status", get(auth::handlers::get_auth_status))
        .route("/auth/register", post(auth::handlers::register_user))
        .route("/auth/login", post(auth::handlers::login_user))
        .route("/auth/refresh", post(auth::handlers::refresh_token))
        // Research template endpoints (public read)
        .route(
            "/research/templates",
            get(handlers::list_research_templates),
        )
        .route(
            "/research/templates/{template_id}",
            get(handlers::get_research_template),
        )
        .route(
            "/research/templates/category/{category}",
            get(handlers::list_templates_by_category),
        )
        // Configuration (public read)
        .route("/config", get(handlers::get_config))
        // Repository listing (public in open mode, protected by middleware)
        .route("/repositories", get(handlers::list_repositories))
        // Wiki viewing (public access)
        .route("/wiki/{repository_id}", get(handlers::get_wiki));

    // Protected routes (authentication required)
    let protected_routes = Router::new()
        // Authentication endpoints (require valid token)
        .route("/auth/me", get(auth::handlers::get_current_user))
        .route("/auth/logout", post(auth::handlers::logout_user))
        // API Key management endpoints
        .route("/auth/api-keys", post(auth::handlers::create_api_key))
        .route("/auth/api-keys", get(auth::handlers::list_api_keys))
        .route(
            "/auth/api-keys/{key_id}",
            delete(auth::handlers::delete_api_key),
        )
        // Repository management (requires GenerateWiki permission)
        .route("/repositories", post(handlers::initialize_repository))
        .route(
            "/repositories/{repository_id}",
            get(handlers::get_repository_info),
        )
        .route(
            "/repositories/{repository_id}",
            delete(handlers::delete_repository),
        )
        .route(
            "/repositories/{repository_id}/reindex",
            post(handlers::reindex_repository),
        )
        // RAG endpoints (requires Query permission)
        .route("/chat", post(handlers::chat_query))
        .route("/chat/stream", post(handlers::chat_stream))
        // Wiki generation (requires GenerateWiki permission)
        .route("/wiki/generate", post(handlers::generate_wiki))
        .route("/wiki/{repository_id}/export", post(handlers::export_wiki))
        // Research endpoints (requires Query permission)
        .route("/research/start", post(handlers::start_research))
        .route("/research/deep", post(handlers::start_research)) // Alias for frontend compatibility
        .route(
            "/research/iterate/{repository_id}",
            post(handlers::research_iteration),
        )
        .route(
            "/research/progress/{repository_id}",
            get(handlers::get_research_progress),
        )
        .route(
            "/research/{research_id}",
            get(handlers::get_research_progress_by_id),
        )
        .route(
            "/research/{research_id}/stop",
            post(handlers::stop_research),
        )
        .route("/research/sessions", get(handlers::list_research_sessions))
        .route(
            "/research/start-from-template",
            post(handlers::start_research_from_template),
        )
        // Research history endpoints (requires Query permission)
        .route("/research/history", get(handlers::get_research_history))
        .route(
            "/research/history/{repository_id}",
            get(handlers::get_research_record),
        )
        .route(
            "/research/history/{repository_id}",
            delete(handlers::delete_research_record),
        )
        .route(
            "/research/statistics",
            get(handlers::get_research_statistics),
        )
        // Configuration (admin write)
        .route("/config", post(handlers::update_config))
        // File operations (query permission required)
        .route("/files/tree", post(handlers::get_file_tree))
        .route("/files/content", post(handlers::get_file_content))
        .route("/files/readme", post(handlers::get_readme))
        // Apply authentication middleware to all protected routes
        .layer(axum::middleware::from_fn_with_state(
            _state.clone(),
            crate::middleware::auth_middleware,
        ));

    let mut router = Router::new().merge(public_routes).merge(protected_routes);

    // Add database-specific routes if SQLite feature is enabled
    #[cfg(feature = "sqlite")]
    {
        router = router.route("/history/{repository_id}", get(handlers::get_query_history));
    }

    router
}

/// Create WebSocket routes
pub fn websocket_routes() -> Router<AppState> {
    Router::new()
        // WebSocket endpoint for all real-time communication
        .route("/", get(websocket::unified_handler))
        .route("/global", get(websocket::unified_handler))
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
pub fn all_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/api", api_routes(state.clone()))
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
        let app = api_routes(state.clone()).with_state(state);

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
