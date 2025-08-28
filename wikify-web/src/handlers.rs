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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Get all repositories (SQLite feature only)
#[cfg(feature = "sqlite")]
pub async fn get_repositories(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(database) = &state.database {
        match database.get_repositories().await {
            Ok(repositories) => {
                let repos_json: Vec<serde_json::Value> = repositories
                    .into_iter()
                    .map(|repo| {
                        serde_json::json!({
                            "id": repo.id,
                            "name": repo.name,
                            "repo_path": repo.repo_path,
                            "repo_type": repo.repo_type,
                            "status": repo.status,
                            "created_at": repo.created_at,
                            "last_indexed_at": repo.last_indexed_at,
                        })
                    })
                    .collect();

                Ok(Json(serde_json::json!({
                    "repositories": repos_json,
                    "count": repos_json.len()
                })))
            }
            Err(e) => {
                tracing::error!("Failed to get repositories: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // 数据库未启用，返回空列表
        Ok(Json(serde_json::json!({
            "repositories": [],
            "count": 0,
            "message": "Database not enabled"
        })))
    }
}

/// Get all sessions (SQLite feature only)
#[cfg(feature = "sqlite")]
pub async fn get_sessions(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(database) = &state.database {
        match database.get_sessions().await {
            Ok(sessions) => {
                let sessions_json: Vec<serde_json::Value> = sessions
                    .into_iter()
                    .map(|session| {
                        serde_json::json!({
                            "id": session.id,
                            "repository_id": session.repository_id,
                            "created_at": session.created_at,
                            "last_activity": session.last_activity,
                            "is_active": session.is_active,
                        })
                    })
                    .collect();

                Ok(Json(serde_json::json!({
                    "sessions": sessions_json,
                    "count": sessions_json.len()
                })))
            }
            Err(e) => {
                tracing::error!("Failed to get sessions: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // 数据库未启用，返回空列表
        Ok(Json(serde_json::json!({
            "sessions": [],
            "count": 0,
            "message": "Database not enabled"
        })))
    }
}

/// Handle chat queries
pub async fn chat_query(
    State(state): State<AppState>,
    JsonExtractor(request): JsonExtractor<ChatQueryRequest>,
) -> Result<Json<ChatQueryResponse>, StatusCode> {
    use tracing::{error, info};

    info!("Processing chat query for session: {}", request.session_id);

    // Update session activity
    if let Err(e) = state.update_session_activity(&request.session_id).await {
        tracing::warn!("Failed to update session activity: {}", e);
    }

    // Execute RAG query
    match state
        .query_rag(&request.session_id, &request.question)
        .await
    {
        Ok(rag_response) => {
            // Convert RAG response to chat response
            let sources: Vec<SourceDocument> = rag_response
                .sources
                .into_iter()
                .map(|source| SourceDocument {
                    file_path: source
                        .chunk
                        .metadata
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    content: source.chunk.content,
                    similarity_score: source.score as f64,
                })
                .collect();

            let response = ChatQueryResponse {
                answer: rag_response.answer.clone(),
                sources: sources.clone(),
                session_id: request.session_id.clone(),
                timestamp: chrono::Utc::now(),
            };

            // Save query to database if available
            #[cfg(feature = "sqlite")]
            if let Some(database) = &state.database {
                if let Err(e) = save_query_to_database(
                    database,
                    &request.session_id,
                    &request.question,
                    &rag_response.answer,
                )
                .await
                {
                    tracing::warn!("Failed to save query to database: {}", e);
                }
            }

            info!("Chat query completed successfully");
            Ok(Json(response))
        }
        Err(e) => {
            error!("Chat query failed: {}", e);

            let error_answer = format!(
                "Sorry, I encountered an error while processing your question: {}",
                e
            );

            // Save failed query to database if available
            #[cfg(feature = "sqlite")]
            if let Some(database) = &state.database {
                if let Err(db_e) = save_query_to_database(
                    database,
                    &request.session_id,
                    &request.question,
                    &error_answer,
                )
                .await
                {
                    tracing::warn!("Failed to save failed query to database: {}", db_e);
                }
            }

            // Return error response
            let response = ChatQueryResponse {
                answer: error_answer,
                sources: vec![],
                session_id: request.session_id,
                timestamp: chrono::Utc::now(),
            };

            Ok(Json(response))
        }
    }
}

/// Get query history (SQLite feature only)
#[cfg(feature = "sqlite")]
pub async fn get_query_history(
    State(state): State<AppState>,
    Path(_repository_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(database) = &state.database {
        // For now, get all queries. TODO: Filter by repository_id when session-repository mapping is implemented
        match database.get_query_history(None, 50).await {
            Ok(queries) => {
                let queries_json: Vec<serde_json::Value> = queries
                    .into_iter()
                    .map(|query| {
                        serde_json::json!({
                            "id": query.id,
                            "session_id": query.session_id,
                            "question": query.question,
                            "answer": query.answer,
                            "created_at": query.created_at,
                        })
                    })
                    .collect();

                Ok(Json(serde_json::json!({
                    "queries": queries_json,
                    "count": queries_json.len()
                })))
            }
            Err(e) => {
                tracing::error!("Failed to get query history: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // 数据库未启用，返回空列表
        Ok(Json(serde_json::json!({
            "queries": [],
            "count": 0,
            "message": "Database not enabled"
        })))
    }
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

/// Save query to database (helper function)
#[cfg(feature = "sqlite")]
async fn save_query_to_database(
    database: &std::sync::Arc<crate::simple_database::SimpleDatabaseService>,
    session_id: &str,
    question: &str,
    answer: &str,
) -> crate::WebResult<()> {
    use crate::simple_database::SimpleQuery;

    let query = SimpleQuery {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: Some(session_id.to_string()),
        question: question.to_string(),
        answer: answer.to_string(),
        created_at: chrono::Utc::now(),
    };

    database.save_query(&query).await?;
    tracing::debug!("Query saved to database: {}", query.id);
    Ok(())
}
