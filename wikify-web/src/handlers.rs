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
use utoipa::ToSchema;

/// Health check response
#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    #[schema(example = "healthy")]
    status: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    #[schema(example = "0.1.0")]
    version: String,
}

/// Repository initialization request
#[derive(Deserialize, ToSchema)]
pub struct InitializeRepositoryRequest {
    #[schema(example = "https://github.com/user/repo")]
    pub repository: String,
    #[schema(example = "github")]
    pub repo_type: Option<String>, // "github", "local", etc.
    pub access_token: Option<String>,
    #[schema(example = true)]
    pub auto_generate_wiki: Option<bool>, // Whether to automatically generate wiki after indexing
}

/// Repository initialization response
#[derive(Serialize, ToSchema)]
pub struct InitializeRepositoryResponse {
    #[schema(example = "uuid-string")]
    pub session_id: String,
    #[schema(example = "success")]
    pub status: String,
    #[schema(example = "Repository initialized successfully")]
    pub message: String,
}

/// Repository deletion response
#[derive(Serialize, ToSchema)]
pub struct DeleteRepositoryResponse {
    #[schema(example = "success")]
    pub status: String,
    #[schema(example = "Repository deleted successfully")]
    pub message: String,
    #[schema(example = "uuid-string")]
    pub deleted_session_id: String,
}

/// Chat query request
#[derive(Deserialize, ToSchema)]
pub struct ChatQueryRequest {
    #[schema(example = "uuid-string")]
    pub session_id: String,
    #[schema(example = "How does the authentication work?")]
    pub question: String,
    pub context: Option<String>,
}

/// Chat query response
#[derive(Serialize, ToSchema)]
pub struct ChatQueryResponse {
    pub answer: String,
    pub sources: Vec<SourceDocument>,
    #[schema(example = "uuid-string")]
    pub session_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Source document information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceDocument {
    #[schema(example = "src/main.rs")]
    pub file_path: String,
    pub content: String,
    #[schema(example = 0.85)]
    pub similarity_score: f64,
}

/// Wiki generation request
#[derive(Deserialize, ToSchema)]
pub struct GenerateWikiRequest {
    #[schema(example = "uuid-string")]
    pub session_id: String,
    pub config: WikiGenerationConfig,
}

/// Wiki generation configuration
#[derive(Deserialize, ToSchema)]
pub struct WikiGenerationConfig {
    #[schema(example = "en")]
    pub language: Option<String>,
    #[schema(example = 50)]
    pub max_pages: Option<usize>,
    #[schema(example = true)]
    pub include_diagrams: Option<bool>,
    #[schema(example = false)]
    pub comprehensive_view: Option<bool>,
}

/// Wiki generation response
#[derive(Serialize, ToSchema)]
pub struct GenerateWikiResponse {
    #[schema(example = "uuid-string")]
    pub wiki_id: String,
    #[schema(example = "success")]
    pub status: String,
    #[schema(example = 25)]
    pub pages_count: usize,
    #[schema(example = 8)]
    pub sections_count: usize,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "Health",
    summary = "Health check",
    description = "Check the server health status",
    responses(
        (status = 200, description = "Server is healthy", body = HealthResponse)
    )
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Initialize repository for processing
#[utoipa::path(
    post,
    path = "/api/repositories",
    tag = "Repository",
    summary = "Initialize repository",
    description = "Initialize a repository for processing and create a new session. If the repository is already being indexed by another session, an error will be returned.",
    request_body = InitializeRepositoryRequest,
    responses(
        (status = 200, description = "Repository initialized successfully", body = InitializeRepositoryResponse),
        (status = 409, description = "Repository is already being indexed by another session"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn initialize_repository(
    State(state): State<AppState>,
    JsonExtractor(request): JsonExtractor<InitializeRepositoryRequest>,
) -> Result<Json<InitializeRepositoryResponse>, StatusCode> {
    let auto_generate_wiki = request.auto_generate_wiki.unwrap_or(true); // Default to true
    match state
        .initialize_rag(&request.repository, auto_generate_wiki)
        .await
    {
        Ok(session_id) => Ok(Json(InitializeRepositoryResponse {
            session_id,
            status: "success".to_string(),
            message: "Repository initialized successfully".to_string(),
        })),
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Failed to initialize repository: {}", error_msg);

            // Check if it's a concurrency/conflict error
            if error_msg.contains("already being indexed")
                || error_msg.contains("already in progress")
            {
                Err(StatusCode::CONFLICT)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Get repository information
#[utoipa::path(
    get,
    path = "/api/repositories/{session_id}",
    tag = "Repository",
    summary = "Get repository information",
    description = "Get information about a repository session",
    params(
        ("session_id" = String, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Repository information retrieved successfully"),
        (status = 404, description = "Session not found")
    )
)]
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

/// Reindex repository
#[utoipa::path(
    post,
    path = "/api/repositories/{session_id}/reindex",
    tag = "Repository",
    summary = "Reindex repository",
    description = "Reindex an existing repository. If the repository is currently being indexed, returns a conflict error. If already indexed, resets the state and starts reindexing.",
    params(
        ("session_id" = String, Path, description = "Session ID of the repository to reindex")
    ),
    responses(
        (status = 200, description = "Repository reindexing started successfully", body = InitializeRepositoryResponse),
        (status = 404, description = "Repository not found"),
        (status = 409, description = "Repository is currently being indexed"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn reindex_repository(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<InitializeRepositoryResponse>, StatusCode> {
    use tracing::{error, info};

    info!("Reindexing repository for session: {}", session_id);

    // Check if session exists
    {
        let sessions = state.sessions.read().await;
        if !sessions.contains_key(&session_id) {
            error!("Session not found for reindexing: {}", session_id);
            return Err(StatusCode::NOT_FOUND);
        }
    }

    // Check if already indexing using IndexingManager
    if state.indexing_manager.is_indexing(&session_id).await {
        error!("Repository is already being indexed: {}", session_id);
        return Err(StatusCode::CONFLICT);
    }

    // Reset session state for reindexing
    {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.is_indexed = false;
            session.indexing_progress = 0.0;
            session.rag_pipeline = None;
            session.last_activity = chrono::Utc::now();
            info!("Reset session state for reindexing: {}", session_id);
        }
    }

    // Get repository path for reindexing
    let repo_path = {
        let sessions = state.sessions.read().await;
        sessions
            .get(&session_id)
            .map(|session| session.repository.clone())
            .ok_or_else(|| {
                error!("Session not found when getting repo path: {}", session_id);
                StatusCode::NOT_FOUND
            })?
    };

    // Start reindexing using IndexingManager for concurrency control
    let sessions = state.sessions.clone();
    let progress_broadcaster = state.progress_broadcaster.clone();
    let wiki_service = state.wiki_service.clone();
    let wiki_cache = state.wiki_cache.clone();
    let session_id_for_task = session_id.clone();
    let repo_path_for_task = repo_path.clone();

    match state
        .indexing_manager
        .start_indexing_with_repo_check(session_id.clone(), repo_path_for_task.clone(), move || {
            let session_id_clone = session_id_for_task.clone();
            let repo_path = repo_path_for_task.clone();
            let sessions = sessions.clone();
            let progress_broadcaster = progress_broadcaster.clone();
            let wiki_service = wiki_service.clone();
            let wiki_cache = wiki_cache.clone();

            async move {
                crate::state::AppState::perform_indexing_task(
                    session_id_clone,
                    repo_path,
                    sessions,
                    progress_broadcaster,
                    wiki_service,
                    wiki_cache,
                )
                .await;
            }
        })
        .await
    {
        Ok(()) => {
            info!("Repository reindexing started for session: {}", session_id);
        }
        Err(e) => {
            error!(
                "Failed to start reindexing for session {}: {}",
                session_id, e
            );

            // Check if it's a concurrency/conflict error
            if e.contains("already being indexed") || e.contains("already in progress") {
                return Err(StatusCode::CONFLICT);
            } else {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    let response = InitializeRepositoryResponse {
        session_id: session_id.clone(),
        status: "success".to_string(),
        message: "Repository reindexing started".to_string(),
    };

    info!("Repository reindexing started for session: {}", session_id);
    Ok(Json(response))
}

/// Delete repository
#[utoipa::path(
    delete,
    path = "/api/repositories/{session_id}",
    tag = "Repository",
    summary = "Delete repository",
    description = "Delete a repository and all associated data including sessions, vector data, and database records",
    params(
        ("session_id" = String, Path, description = "Session ID of the repository to delete")
    ),
    responses(
        (status = 200, description = "Repository deleted successfully", body = DeleteRepositoryResponse),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_repository(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<DeleteRepositoryResponse>, StatusCode> {
    match state.delete_repository(&session_id).await {
        Ok(()) => Ok(Json(DeleteRepositoryResponse {
            status: "success".to_string(),
            message: "Repository deleted successfully".to_string(),
            deleted_session_id: session_id,
        })),
        Err(e) => {
            tracing::error!("Failed to delete repository {}: {}", session_id, e);
            match e {
                crate::WebError::NotFound(_) => Err(StatusCode::NOT_FOUND),
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

/// Get all repositories (SQLite feature only)
#[cfg(feature = "sqlite")]
#[utoipa::path(
    get,
    path = "/api/repositories",
    tag = "Repository",
    summary = "Get all repositories",
    description = "Get a list of all repositories (requires SQLite feature)",
    responses(
        (status = 200, description = "Repositories retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_repositories(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(database) = &state.database {
        match database.get_repositories().await {
            Ok(repositories) => {
                // Get current session states from memory
                let sessions = state.sessions.read().await;

                let repos_json: Vec<serde_json::Value> = repositories
                    .into_iter()
                    .map(|repo| {
                        // Check if there's a corresponding session with updated status
                        let current_status = if let Some(session) = sessions.get(&repo.id) {
                            if session.is_indexed {
                                "indexed"
                            } else if session.indexing_progress > 0.0 {
                                "indexing"
                            } else {
                                "created"
                            }
                        } else {
                            // Use database status as fallback
                            &repo.status
                        };

                        let indexing_progress = sessions
                            .get(&repo.id)
                            .map(|s| s.indexing_progress)
                            .unwrap_or(0.0);

                        let last_activity = sessions
                            .get(&repo.id)
                            .map(|s| s.last_activity.to_rfc3339())
                            .unwrap_or_else(|| repo.created_at.to_rfc3339());

                        serde_json::json!({
                            "id": repo.id,
                            "name": repo.name,
                            "repo_path": repo.repo_path,
                            "repo_type": repo.repo_type,
                            "status": current_status,
                            "indexing_progress": indexing_progress,
                            "created_at": repo.created_at,
                            "last_indexed_at": repo.last_indexed_at,
                            "last_activity": last_activity,
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
#[utoipa::path(
    get,
    path = "/api/sessions",
    tag = "Session",
    summary = "Get all sessions",
    description = "Get a list of all active sessions (requires SQLite feature)",
    responses(
        (status = 200, description = "Sessions retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
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
#[utoipa::path(
    post,
    path = "/api/chat",
    tag = "Chat",
    summary = "Ask a question",
    description = "Ask a question about the repository using RAG (Retrieval-Augmented Generation)",
    request_body = ChatQueryRequest,
    responses(
        (status = 200, description = "Question answered successfully", body = ChatQueryResponse)
    )
)]
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
#[utoipa::path(
    get,
    path = "/api/history/{repository_id}",
    tag = "Chat",
    summary = "Get query history",
    description = "Get chat history for a specific repository (requires SQLite feature)",
    params(
        ("repository_id" = String, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Query history retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
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
#[utoipa::path(
    post,
    path = "/api/chat/stream",
    tag = "Chat",
    summary = "Stream chat response",
    description = "Ask a question and receive streaming response (not yet implemented)",
    request_body = ChatQueryRequest,
    responses(
        (status = 200, description = "Streaming response started"),
        (status = 501, description = "Not implemented")
    )
)]
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
#[utoipa::path(
    post,
    path = "/api/wiki/generate",
    tag = "Wiki",
    summary = "Generate wiki documentation",
    description = "Generate comprehensive wiki documentation for a repository",
    request_body = GenerateWikiRequest,
    responses(
        (status = 200, description = "Wiki generated successfully", body = GenerateWikiResponse),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Wiki generation failed")
    )
)]
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
    match state
        .generate_wiki_for_session(&request.session_id, &session.repository, wiki_config)
        .await
    {
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
#[utoipa::path(
    get,
    path = "/api/wiki/{session_id}",
    tag = "Wiki",
    summary = "Get generated wiki",
    description = "Retrieve the generated wiki documentation for a session",
    params(
        ("session_id" = String, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Wiki retrieved successfully"),
        (status = 404, description = "Wiki not found")
    )
)]
pub async fn get_wiki(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get session info (verify session exists)
    let _session = match state.get_session(&session_id).await {
        Some(session) => session,
        None => return Err(StatusCode::NOT_FOUND),
    };

    // Get cached wiki using session_id as key
    match state.get_cached_wiki(&session_id).await {
        Some(cached_wiki) => Ok(Json(serde_json::to_value(&cached_wiki.wiki).unwrap())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Export wiki in various formats
#[utoipa::path(
    post,
    path = "/api/wiki/{session_id}/export",
    tag = "Wiki",
    summary = "Export wiki",
    description = "Export generated wiki in various formats (not yet implemented)",
    params(
        ("session_id" = String, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Wiki exported successfully"),
        (status = 501, description = "Not implemented")
    )
)]
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
#[utoipa::path(
    get,
    path = "/api/config",
    tag = "Configuration",
    summary = "Get server configuration",
    description = "Get current server configuration",
    responses(
        (status = 200, description = "Configuration retrieved successfully")
    )
)]
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

#[cfg(test)]
mod tests {

    use crate::{AppState, WebConfig};
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_delete_repository_not_found() {
        let state = AppState::new(WebConfig::default()).await.unwrap();
        let app = crate::routes::api_routes().with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri("/repositories/non-existent-session")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_repository_success() {
        let state = AppState::new(WebConfig::default()).await.unwrap();

        // Create a mock session directly (without indexing)
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = crate::state::RepositorySession {
            id: session_id.clone(),
            repository: "./mock-repo".to_string(),
            repo_type: "local".to_string(),
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            is_indexed: false,
            indexing_progress: 0.0,
            auto_generate_wiki: false,
            rag_pipeline: None,
        };

        // Insert the session manually
        {
            let mut sessions = state.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        // Verify it exists
        assert!(state.get_session(&session_id).await.is_some());

        // Now delete it
        let result = state.delete_repository(&session_id).await;
        assert!(result.is_ok());

        // Verify it's gone
        assert!(state.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_indexing_progress_broadcast() {
        let state = AppState::new(WebConfig::default()).await.unwrap();

        // Subscribe to progress updates
        let mut progress_receiver = state.subscribe_to_progress();

        // Send a test progress update
        let test_progress = crate::state::IndexingUpdate::Progress {
            session_id: "test-session".to_string(),
            stage: "Testing".to_string(),
            percentage: 50.0,
            current_item: Some("test-file.rs".to_string()),
            files_processed: Some(5),
            total_files: Some(10),
        };

        // Send the progress update
        let _ = state.progress_broadcaster.send(test_progress.clone());

        // Receive and verify the progress update
        let received = progress_receiver.recv().await.unwrap();
        match received {
            crate::state::IndexingUpdate::Progress {
                session_id,
                stage,
                percentage,
                ..
            } => {
                assert_eq!(session_id, "test-session");
                assert_eq!(stage, "Testing");
                assert_eq!(percentage, 50.0);
            }
            _ => panic!("Expected Progress update"),
        }
    }
}
