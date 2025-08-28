//! HTTP request handlers for the Wikify web server
//!
//! This module contains all the HTTP request handlers.

use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, Json},
    Json as JsonExtractor,
};
use serde::{Deserialize, Serialize};

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    version: String,
}

/// Repository initialization request
#[derive(Deserialize)]
pub struct InitializeRepositoryRequest {
    pub repository: String,
    pub repo_type: Option<String>, // "github", "local", etc.
    pub access_token: Option<String>,
}

/// Repository initialization response
#[derive(Serialize)]
pub struct InitializeRepositoryResponse {
    pub session_id: String,
    pub status: String,
    pub message: String,
}

/// Chat query request
#[derive(Deserialize)]
pub struct ChatQueryRequest {
    pub session_id: String,
    pub question: String,
    pub context: Option<String>,
}

/// Chat query response
#[derive(Serialize)]
pub struct ChatQueryResponse {
    pub answer: String,
    pub sources: Vec<SourceDocument>,
    pub session_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Source document information
#[derive(Serialize)]
pub struct SourceDocument {
    pub file_path: String,
    pub content: String,
    pub similarity_score: f64,
}

/// Wiki generation request
#[derive(Deserialize)]
pub struct GenerateWikiRequest {
    pub session_id: String,
    pub config: WikiGenerationConfig,
}

/// Wiki generation configuration
#[derive(Deserialize)]
pub struct WikiGenerationConfig {
    pub language: Option<String>,
    pub max_pages: Option<usize>,
    pub include_diagrams: Option<bool>,
    pub comprehensive_view: Option<bool>,
}

/// Wiki generation response
#[derive(Serialize)]
pub struct GenerateWikiResponse {
    pub wiki_id: String,
    pub status: String,
    pub pages_count: usize,
    pub sections_count: usize,
}

/// Health check endpoint
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Initialize repository for processing
pub async fn initialize_repository(
    State(state): State<AppState>,
    JsonExtractor(request): JsonExtractor<InitializeRepositoryRequest>,
) -> Result<Json<InitializeRepositoryResponse>, StatusCode> {
    match state.initialize_rag(&request.repository).await {
        Ok(session_id) => Ok(Json(InitializeRepositoryResponse {
            session_id,
            status: "success".to_string(),
            message: "Repository initialized successfully".to_string(),
        })),
        Err(e) => {
            tracing::error!("Failed to initialize repository: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get repository information
pub async fn get_repository_info(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.get_session(&session_id).await {
        Some(session) => {
            let info = serde_json::json!({
                "session_id": session.id,
                "repository": session.repository,
                "repo_type": session.repo_type,
                "created_at": session.created_at,
                "last_activity": session.last_activity,
                "is_indexed": session.is_indexed,
                "indexing_progress": session.indexing_progress,
            });
            Ok(Json(info))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Handle chat queries
pub async fn chat_query(
    State(state): State<AppState>,
    JsonExtractor(request): JsonExtractor<ChatQueryRequest>,
) -> Result<Json<ChatQueryResponse>, StatusCode> {
    // Update session activity
    state.update_session_activity(&request.session_id).await;

    // For now, return a placeholder response since we need to redesign RAG for web usage
    let response = ChatQueryResponse {
        answer: "RAG integration is being implemented. This is a placeholder response.".to_string(),
        sources: vec![],
        session_id: request.session_id,
        timestamp: chrono::Utc::now(),
    };

    Ok(Json(response))
}

/// Handle streaming chat queries (placeholder)
pub async fn chat_stream(
    State(_state): State<AppState>,
    JsonExtractor(_request): JsonExtractor<ChatQueryRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Placeholder for streaming implementation
    Ok(Json(serde_json::json!({
        "message": "Streaming chat is not yet implemented"
    })))
}

/// Generate wiki for repository
pub async fn generate_wiki(
    State(state): State<AppState>,
    JsonExtractor(request): JsonExtractor<GenerateWikiRequest>,
) -> Result<Json<GenerateWikiResponse>, StatusCode> {
    // Get session info
    let session = match state.get_session(&request.session_id).await {
        Some(session) => session,
        None => return Err(StatusCode::NOT_FOUND),
    };

    // Create wiki configuration
    let mut wiki_config = wikify_wiki::WikiConfig::default();
    if let Some(language) = request.config.language {
        wiki_config.language = language;
    }
    if let Some(max_pages) = request.config.max_pages {
        wiki_config.max_pages = Some(max_pages);
    }
    if let Some(include_diagrams) = request.config.include_diagrams {
        wiki_config.include_diagrams = include_diagrams;
    }
    if let Some(comprehensive_view) = request.config.comprehensive_view {
        wiki_config.comprehensive_view = comprehensive_view;
    }

    // Generate wiki
    match state.generate_wiki(&session.repository, wiki_config).await {
        Ok(wiki) => {
            let response = GenerateWikiResponse {
                wiki_id: wiki.id.clone(),
                status: "success".to_string(),
                pages_count: wiki.pages.len(),
                sections_count: wiki.sections.len(),
            };
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to generate wiki: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get generated wiki
pub async fn get_wiki(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get session info
    let session = match state.get_session(&session_id).await {
        Some(session) => session,
        None => return Err(StatusCode::NOT_FOUND),
    };

    // Get cached wiki
    match state.get_cached_wiki(&session.repository).await {
        Some(cached_wiki) => Ok(Json(serde_json::to_value(&cached_wiki.wiki).unwrap())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Export wiki in various formats
pub async fn export_wiki(
    State(_state): State<AppState>,
    Path(_session_id): Path<String>,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Placeholder for wiki export
    Ok(Json(serde_json::json!({
        "message": "Wiki export is not yet implemented"
    })))
}

/// Get file tree for repository
pub async fn get_file_tree(
    State(_state): State<AppState>,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Placeholder for file tree
    Ok(Json(serde_json::json!({
        "message": "File tree endpoint is not yet implemented"
    })))
}

/// Get file content
pub async fn get_file_content(
    State(_state): State<AppState>,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Placeholder for file content
    Ok(Json(serde_json::json!({
        "message": "File content endpoint is not yet implemented"
    })))
}

/// Get server configuration
pub async fn get_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "host": state.config.host,
        "port": state.config.port,
        "dev_mode": state.config.dev_mode,
    }))
}

/// Update server configuration
pub async fn update_config(
    State(_state): State<AppState>,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Placeholder for config update
    Ok(Json(serde_json::json!({
        "message": "Config update is not yet implemented"
    })))
}

/// SPA fallback handler (serves index.html for client-side routing)
pub async fn spa_fallback() -> Html<&'static str> {
    // For now, return a simple HTML page
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Wikify Web Server</title>
    <style>
        body { font-family: system-ui, sans-serif; margin: 0; padding: 2rem; background: #f5f5f5; }
        .container { max-width: 800px; margin: 0 auto; background: white; padding: 2rem; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .header { text-align: center; margin-bottom: 2rem; }
        .status { background: #d4edda; color: #155724; padding: 1rem; border-radius: 4px; margin-bottom: 2rem; }
        .api-list { background: #f8f9fa; padding: 1rem; border-radius: 4px; }
        .api-list h3 { margin-top: 0; }
        .api-list ul { margin: 0; }
        .api-list li { margin: 0.5rem 0; }
        .api-list code { background: #e9ecef; padding: 0.2rem 0.4rem; border-radius: 3px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 Wikify Web Server</h1>
            <p>AI-Powered Repository Documentation & Chat</p>
        </div>

        <div class="status">
            ✅ Web server is running successfully!
        </div>

        <div class="api-list">
            <h3>Available API Endpoints:</h3>
            <ul>
                <li><code>GET /api/health</code> - Health check</li>
                <li><code>POST /api/repositories</code> - Initialize repository</li>
                <li><code>POST /api/chat</code> - Chat with repository</li>
                <li><code>POST /api/wiki/generate</code> - Generate wiki</li>
                <li><code>GET /ws/chat</code> - WebSocket chat</li>
                <li><code>GET /ws/wiki</code> - WebSocket wiki generation</li>
            </ul>
        </div>

        <div style="text-align: center; margin-top: 2rem; color: #666;">
            <p>Frontend interface coming soon...</p>
        </div>
    </div>
</body>
</html>
    "#,
    )
}
