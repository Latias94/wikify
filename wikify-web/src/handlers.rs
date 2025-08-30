//! HTTP request handlers for the Wikify web server
//!
//! This module contains all the HTTP request handlers.

use crate::{
    auth::{
        AdminUser, RequireExport, RequireGenerateWiki, RequireManageSession, RequireQuery, User,
    },
    AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, Json},
    Json as JsonExtractor,
};
use serde::{Deserialize, Serialize};

use tracing::{error, info, warn};
use utoipa::ToSchema;
use wikify_applications::{
    research::{ResearchHistoryFilters, ResearchHistoryRecord, ResearchStatistics},
    PermissionContext, ResearchCategory, ResearchConfig, ResearchProgress, ResearchTemplate,
};

/// Helper function to convert User to PermissionContext for application layer
fn user_to_permission_context(user: &User) -> PermissionContext {
    user.to_permission_context()
}

// All handlers now use the new permission extractors (RequireQuery, RequireGenerateWiki, etc.)
// Legacy extract_permission_context function has been removed

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

/// Reindex response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReindexResponse {
    pub session_id: String,
    pub status: String,
    pub message: String,
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
    RequireGenerateWiki(user): RequireGenerateWiki,
    JsonExtractor(request): JsonExtractor<InitializeRepositoryRequest>,
) -> Result<Json<InitializeRepositoryResponse>, StatusCode> {
    info!(
        "Initializing repository: {} (user: {})",
        request.repository, user.id
    );

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);

    let auto_generate_wiki = request.auto_generate_wiki.unwrap_or(true);
    match state
        .initialize_rag(&request.repository, auto_generate_wiki)
        .await
    {
        Ok(session_id) => {
            info!("Repository initialized successfully: {}", session_id);
            Ok(Json(InitializeRepositoryResponse {
                session_id,
                status: "success".to_string(),
                message: "Repository initialized successfully".to_string(),
            }))
        }
        Err(e) => {
            let error_msg = e.to_string();
            error!("Failed to initialize repository: {}", error_msg);

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
    RequireQuery(user): RequireQuery,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Getting repository info for session: {} (user: {})",
        session_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    match state.application.get_session(&context, &session_id).await {
        Ok(session) => {
            let info = serde_json::json!({
                "session_id": session.id,
                "repository": session.repository,
                "repo_type": session.repository.repo_type,
                "created_at": session.created_at,
                "last_activity": session.last_activity,
                "is_indexed": session.is_indexed,
                "indexing_progress": session.indexing_progress,
            });
            Ok(Json(info))
        }
        Err(_) => {
            warn!("Session not found: {}", session_id);
            Err(StatusCode::NOT_FOUND)
        }
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
    RequireManageSession(user): RequireManageSession,
    Path(session_id): Path<String>,
) -> Result<Json<InitializeRepositoryResponse>, StatusCode> {
    info!(
        "Reindexing repository for session: {} (user: {})",
        session_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Check if session exists using application layer
    match state.application.get_session(&context, &session_id).await {
        Ok(_) => {
            info!("Session found, proceeding with reindexing: {}", session_id);
        }
        Err(_) => {
            error!("Session not found for reindexing: {}", session_id);
            return Err(StatusCode::NOT_FOUND);
        }
    }

    // For now, we'll skip the indexing check since the application layer handles this
    // TODO: Add indexing status check to application layer if needed

    // TODO: Implement reindexing through the application layer
    warn!("Reindexing not yet implemented with new application layer");

    let response = InitializeRepositoryResponse {
        session_id: session_id.clone(),
        status: "not_implemented".to_string(),
        message: "Reindexing will be implemented in the application layer".to_string(),
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
    RequireManageSession(user): RequireManageSession,
    Path(session_id): Path<String>,
) -> Result<Json<DeleteRepositoryResponse>, StatusCode> {
    info!(
        "Deleting repository for session: {} (user: {})",
        session_id, user.id
    );

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
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
    RequireQuery(user): RequireQuery,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Getting repositories list (user: {})", user.id);

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
    if let Some(database) = &state.database {
        match database.get_repositories().await {
            Ok(repositories) => {
                // Get current session states from application layer
                let context = state.create_anonymous_context();
                let sessions = state.application.list_sessions(&context).await;

                let repos_json: Vec<serde_json::Value> = repositories
                    .into_iter()
                    .map(|repo| {
                        // Check if there's a corresponding session with updated status
                        let session = sessions.iter().find(|s| s.repository.url == repo.id);

                        let current_status = if let Some(session) = session {
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

                        let indexing_progress = session.map(|s| s.indexing_progress).unwrap_or(0.0);

                        let last_activity = sessions
                            .iter()
                            .find(|s| s.repository.url == repo.id)
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
    RequireQuery(user): RequireQuery,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Getting sessions list (user: {})", user.id);

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
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
    RequireQuery(user): RequireQuery,
    JsonExtractor(request): JsonExtractor<ChatQueryRequest>,
) -> Result<Json<ChatQueryResponse>, StatusCode> {
    info!(
        "Processing chat query for session: {} (user: {})",
        request.session_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Update session activity
    if let Err(e) = state.update_session_activity(&request.session_id).await {
        warn!("Failed to update session activity: {}", e);
    }

    // Execute RAG query using application layer
    match state
        .query_rag(&request.session_id, &request.question)
        .await
    {
        Ok(rag_response) => {
            // Convert RAG response to chat response
            let sources: Vec<SourceDocument> = rag_response
                .sources
                .into_iter()
                .enumerate()
                .map(|(i, source_content)| SourceDocument {
                    file_path: format!("source_{}", i), // TODO: Extract actual file path from content
                    content: source_content,
                    similarity_score: 1.0, // TODO: Get actual similarity score from application layer
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
    RequireQuery(user): RequireQuery,
    Path(_repository_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Getting query history for repository (user: {})", user.id);

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
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
    RequireQuery(user): RequireQuery,
    JsonExtractor(_request): JsonExtractor<ChatQueryRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Starting chat stream (user: {})", user.id);

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
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
    RequireGenerateWiki(user): RequireGenerateWiki,
    JsonExtractor(request): JsonExtractor<GenerateWikiRequest>,
) -> Result<Json<GenerateWikiResponse>, StatusCode> {
    info!(
        "Generating wiki for session: {} (user: {})",
        request.session_id, user.id
    );

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
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
        .generate_wiki_for_session(&request.session_id, &session.repository.url, wiki_config)
        .await
    {
        Ok(wiki) => {
            let response = GenerateWikiResponse {
                wiki_id: request.session_id.clone(), // Use session_id as wiki_id
                status: "success".to_string(),
                pages_count: 1, // Placeholder - wiki is a single string now
                sections_count: wiki.matches('#').count(), // Count markdown headers
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
    RequireQuery(user): RequireQuery,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Getting wiki for session: {} (user: {})",
        session_id, user.id
    );

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
    // Get session info (verify session exists)
    let _session = match state.get_session(&session_id).await {
        Some(session) => session,
        None => return Err(StatusCode::NOT_FOUND),
    };

    // Get cached wiki using session_id as key
    match state.get_cached_wiki(&session_id).await {
        Some(cached_wiki) => Ok(Json(serde_json::to_value(&cached_wiki.content).unwrap())),
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
    RequireExport(user): RequireExport,
    Path(_session_id): Path<String>,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Exporting wiki for session: {} (user: {})",
        _session_id, user.id
    );

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
    // Placeholder for wiki export
    Ok(Json(serde_json::json!({
        "message": "Wiki export is not yet implemented"
    })))
}

/// Get file tree for repository
pub async fn get_file_tree(
    State(_state): State<AppState>,
    AdminUser(user): AdminUser,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Getting file tree (admin user: {})", user.id);
    // Placeholder for file tree
    Ok(Json(serde_json::json!({
        "message": "File tree endpoint is not yet implemented"
    })))
}

/// Get file content
pub async fn get_file_content(
    State(_state): State<AppState>,
    AdminUser(user): AdminUser,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Getting file content (admin user: {})", user.id);
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
    AdminUser(user): AdminUser,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Updating configuration (admin user: {})", user.id);
    // Placeholder for config update
    Ok(Json(serde_json::json!({
        "message": "Config update is not yet implemented"
    })))
}

// Research-related request/response types

/// Start research request
#[derive(Debug, Deserialize, ToSchema)]
pub struct StartResearchRequest {
    /// Session ID for the research
    pub session_id: String,
    /// Research topic
    pub topic: String,
    /// Optional research configuration
    pub config: Option<ResearchConfig>,
}

/// Research iteration request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResearchIterationRequest {
    /// Session ID for the research
    pub session_id: String,
}

/// Research progress response
#[derive(Debug, Serialize, ToSchema)]
pub struct ResearchProgressResponse {
    /// Session ID
    pub session_id: String,
    /// Current iteration
    pub current_iteration: usize,
    /// Total planned iterations
    pub total_iterations: usize,
    /// Current stage description
    pub stage: String,
    /// Progress percentage (0.0-1.0)
    pub progress: f64,
    /// Current question being researched
    pub current_question: Option<String>,
    /// Number of findings so far
    pub findings_count: usize,
    /// Estimated time remaining in seconds
    pub estimated_remaining_seconds: Option<u64>,
}

impl From<ResearchProgress> for ResearchProgressResponse {
    fn from(progress: ResearchProgress) -> Self {
        Self {
            session_id: progress.session_id,
            current_iteration: progress.current_iteration,
            total_iterations: progress.total_iterations,
            stage: progress.stage,
            progress: progress.progress,
            current_question: progress.current_question,
            findings_count: progress.findings_count,
            estimated_remaining_seconds: progress.estimated_remaining.map(|d| d.as_secs()),
        }
    }
}

// Research API endpoints

/// Start a deep research session
#[utoipa::path(
    post,
    path = "/api/research/start",
    request_body = StartResearchRequest,
    responses(
        (status = 200, description = "Research started successfully", body = ResearchProgressResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Permission denied"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn start_research(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    JsonExtractor(request): JsonExtractor<StartResearchRequest>,
) -> Result<Json<ResearchProgressResponse>, StatusCode> {
    info!(
        "Starting research for topic: {} (user: {})",
        request.topic, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Start research using application layer
    match state
        .application
        .start_research(&context, request.session_id, request.topic, request.config)
        .await
    {
        Ok(progress) => {
            info!("Research started successfully");
            Ok(Json(ResearchProgressResponse::from(progress)))
        }
        Err(e) => {
            error!("Failed to start research: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Execute one research iteration
#[utoipa::path(
    post,
    path = "/api/research/iterate/{session_id}",
    params(
        ("session_id" = String, Path, description = "Research session ID")
    ),
    responses(
        (status = 200, description = "Research iteration completed", body = ResearchProgressResponse),
        (status = 403, description = "Permission denied"),
        (status = 404, description = "Research session not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn research_iteration(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(session_id): Path<String>,
) -> Result<Json<ResearchProgressResponse>, StatusCode> {
    info!(
        "Executing research iteration for session: {} (user: {})",
        session_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Execute research iteration using application layer
    match state
        .application
        .research_iteration(&context, &session_id)
        .await
    {
        Ok(progress) => {
            info!("Research iteration completed successfully");
            Ok(Json(ResearchProgressResponse::from(progress)))
        }
        Err(e) => {
            error!("Failed to execute research iteration: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Get research progress
#[utoipa::path(
    get,
    path = "/api/research/progress/{session_id}",
    params(
        ("session_id" = String, Path, description = "Research session ID")
    ),
    responses(
        (status = 200, description = "Research progress retrieved", body = ResearchProgressResponse),
        (status = 403, description = "Permission denied"),
        (status = 404, description = "Research session not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_research_progress(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(session_id): Path<String>,
) -> Result<Json<ResearchProgressResponse>, StatusCode> {
    info!(
        "Getting research progress for session: {} (user: {})",
        session_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Get research progress using application layer
    match state
        .application
        .get_research_progress(&context, &session_id)
        .await
    {
        Ok(progress) => Ok(Json(ResearchProgressResponse::from(progress))),
        Err(e) => {
            error!("Failed to get research progress: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
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
        .api-list h3 { margin-top: 1.5rem; margin-bottom: 0.5rem; color: #495057; }
        .api-list h3:first-child { margin-top: 0; }
        .api-list ul { margin: 0; padding-left: 1.5rem; }
        .api-list li { margin: 0.3rem 0; }
        .api-list code { background: #e9ecef; padding: 0.2rem 0.4rem; border-radius: 3px; font-family: 'Courier New', monospace; }
        .api-list a { color: #007bff; text-decoration: none; }
        .api-list a:hover { text-decoration: underline; }
        .api-list strong a { font-weight: bold; }
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
            <h3>📚 API Documentation:</h3>
            <ul>
                <li><strong><a href="/api-docs/docs/" target="_blank">🔗 Interactive API Documentation (Swagger UI)</a></strong></li>
                <li><a href="/api-docs/openapi.json" target="_blank">📄 OpenAPI Specification (JSON)</a></li>
                <li><a href="/api-docs/openapi.yaml" target="_blank">📄 OpenAPI Specification (YAML)</a></li>
            </ul>

            <h3>🔑 Key API Endpoints:</h3>
            <ul>
                <li><strong>Authentication:</strong>
                    <ul>
                        <li><code>POST /api/auth/register</code> - User registration</li>
                        <li><code>POST /api/auth/login</code> - User login</li>
                        <li><code>POST /api/auth/refresh</code> - Refresh access token</li>
                    </ul>
                </li>
                <li><strong>Repository Management:</strong>
                    <ul>
                        <li><code>POST /api/repositories</code> - Initialize repository</li>
                        <li><code>GET /api/repositories</code> - List repositories</li>
                        <li><code>POST /api/repositories/{id}/reindex</code> - Reindex repository</li>
                        <li><code>DELETE /api/repositories/{id}</code> - Delete repository</li>
                    </ul>
                </li>
                <li><strong>AI Chat & Research:</strong>
                    <ul>
                        <li><code>POST /api/chat</code> - Chat with repository</li>
                        <li><code>POST /api/research/deep</code> - Start deep research</li>
                        <li><code>GET /api/research/{id}</code> - Get research status</li>
                    </ul>
                </li>
                <li><strong>Wiki Generation:</strong>
                    <ul>
                        <li><code>POST /api/wiki/generate</code> - Generate wiki</li>
                        <li><code>GET /api/wiki/{id}</code> - Get wiki content</li>
                    </ul>
                </li>
                <li><strong>WebSocket Streams:</strong>
                    <ul>
                        <li><code>GET /ws/chat</code> - Real-time chat streaming</li>
                        <li><code>GET /ws/wiki</code> - Real-time wiki generation</li>
                        <li><code>GET /ws/research</code> - Real-time research progress</li>
                    </ul>
                </li>
            </ul>
        </div>

        <div style="text-align: center; margin-top: 2rem; padding: 1rem; background: #e3f2fd; border-radius: 4px;">
            <p><strong>🎉 Wikify is now fully operational!</strong></p>
            <p>Access the web interface at <a href="/" target="_blank">http://localhost:8080</a></p>
            <p>For complete API documentation, visit <a href="/api-docs/docs/" target="_blank">Swagger UI</a></p>
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
    use tracing::warn;

    #[tokio::test]
    async fn test_delete_repository_not_found() {
        let state = AppState::new(WebConfig::default()).await.unwrap();
        let app = crate::routes::api_routes(state.clone()).with_state(state);

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

    // TODO: Fix this test for new application layer architecture
    // #[tokio::test]
    // async fn test_delete_repository_success() {
    //     let state = AppState::new(WebConfig::default()).await.unwrap();
    //     // Test implementation needs to be updated for new application layer
    // }

    // TODO: Fix this test - subscribe_to_progress method doesn't exist
    // #[tokio::test]
    // async fn test_indexing_progress_broadcast() {
    //     let state = AppState::new(WebConfig::default()).await.unwrap();
    //     // Test implementation needs to be updated
    // }

    //     // Test implementation would go here
    // }
}

// Research Template API endpoints

/// List all research templates
#[utoipa::path(
    get,
    path = "/api/research/templates",
    responses(
        (status = 200, description = "List of research templates", body = Vec<ResearchTemplate>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_research_templates(
    State(state): State<AppState>,
) -> Result<Json<Vec<ResearchTemplate>>, StatusCode> {
    info!("Listing research templates");

    // Create anonymous context for public endpoint
    let context = state.create_anonymous_context();

    match state.application.list_research_templates(&context).await {
        Ok(templates) => {
            info!("Found {} research templates", templates.len());
            Ok(Json(templates))
        }
        Err(e) => {
            error!("Failed to list research templates: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get specific research template
#[utoipa::path(
    get,
    path = "/api/research/templates/{template_id}",
    params(
        ("template_id" = String, Path, description = "Template ID")
    ),
    responses(
        (status = 200, description = "Research template", body = ResearchTemplate),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_research_template(
    State(state): State<AppState>,
    Path(template_id): Path<String>,
) -> Result<Json<ResearchTemplate>, StatusCode> {
    info!("Getting research template: {}", template_id);

    // Create anonymous context for public endpoint
    let context = state.create_anonymous_context();

    match state
        .application
        .get_research_template(&context, &template_id)
        .await
    {
        Ok(Some(template)) => {
            info!("Found research template: {}", template_id);
            Ok(Json(template))
        }
        Ok(None) => {
            warn!("Research template not found: {}", template_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get research template: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List templates by category
#[utoipa::path(
    get,
    path = "/api/research/templates/category/{category}",
    params(
        ("category" = String, Path, description = "Template category")
    ),
    responses(
        (status = 200, description = "List of templates in category", body = Vec<ResearchTemplate>),
        (status = 400, description = "Invalid category"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_templates_by_category(
    State(state): State<AppState>,
    Path(category_str): Path<String>,
) -> Result<Json<Vec<ResearchTemplate>>, StatusCode> {
    info!("Listing templates by category: {}", category_str);

    // Parse category
    let category = match category_str.to_lowercase().as_str() {
        "technical" => ResearchCategory::Technical,
        "architecture" => ResearchCategory::Architecture,
        "security" => ResearchCategory::Security,
        "performance" => ResearchCategory::Performance,
        "documentation" => ResearchCategory::Documentation,
        "business" => ResearchCategory::Business,
        "custom" => ResearchCategory::Custom,
        _ => {
            warn!("Invalid research category: {}", category_str);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Create anonymous context for public endpoint
    let context = state.create_anonymous_context();

    match state
        .application
        .list_templates_by_category(&context, &category)
        .await
    {
        Ok(templates) => {
            info!(
                "Found {} templates in category {}",
                templates.len(),
                category_str
            );
            Ok(Json(templates))
        }
        Err(e) => {
            error!("Failed to list templates by category: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Start research from template request
#[derive(Debug, Deserialize, ToSchema)]
pub struct StartResearchFromTemplateRequest {
    /// Session ID for the research
    pub session_id: String,
    /// Template ID to use
    pub template_id: String,
    /// Research topic
    pub topic: String,
    /// Template parameters
    #[serde(default)]
    pub parameters: std::collections::HashMap<String, String>,
}

/// Start research from template
#[utoipa::path(
    post,
    path = "/api/research/start-from-template",
    request_body = StartResearchFromTemplateRequest,
    responses(
        (status = 200, description = "Research started from template", body = ResearchProgressResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Permission denied"),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn start_research_from_template(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    JsonExtractor(request): JsonExtractor<StartResearchFromTemplateRequest>,
) -> Result<Json<ResearchProgressResponse>, StatusCode> {
    info!(
        "Starting research from template: {} for topic: {} (user: {})",
        request.template_id, request.topic, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Start research from template using application layer
    match state
        .application
        .start_research_from_template(
            &context,
            request.session_id,
            request.template_id,
            request.topic,
            request.parameters,
        )
        .await
    {
        Ok(progress) => {
            info!("Research started from template successfully");
            Ok(Json(ResearchProgressResponse::from(progress)))
        }
        Err(e) => {
            error!("Failed to start research from template: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

// Research History API endpoints

/// Research history query parameters
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::IntoParams))]
pub struct ResearchHistoryQuery {
    /// Filter by status
    pub status: Option<String>,
    /// Filter by template ID
    pub template_id: Option<String>,
    /// Filter by date from (ISO 8601)
    pub date_from: Option<String>,
    /// Filter by date to (ISO 8601)
    pub date_to: Option<String>,
    /// Limit number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Get research history
#[utoipa::path(
    get,
    path = "/api/research/history",
    params(ResearchHistoryQuery),
    responses(
        (status = 200, description = "Research history", body = Vec<ResearchHistoryRecord>),
        (status = 403, description = "Permission denied"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_research_history(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    axum::extract::Query(query): axum::extract::Query<ResearchHistoryQuery>,
) -> Result<Json<Vec<ResearchHistoryRecord>>, StatusCode> {
    info!("Getting research history (user: {})", user.id);

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Parse query parameters
    let mut filters = ResearchHistoryFilters::default();

    if let Some(status_str) = query.status {
        // Parse status - simplified for now
        filters.status = match status_str.to_lowercase().as_str() {
            "in_progress" => Some(wikify_applications::research::ResearchStatus::InProgress),
            "completed" => Some(wikify_applications::research::ResearchStatus::Completed),
            "cancelled" => Some(wikify_applications::research::ResearchStatus::Cancelled),
            _ => None,
        };
    }

    filters.template_id = query.template_id;
    filters.limit = query.limit;
    filters.offset = query.offset;

    // Parse dates if provided
    if let Some(date_str) = query.date_from {
        if let Ok(date) = chrono::DateTime::parse_from_rfc3339(&date_str) {
            filters.date_from = Some(date.with_timezone(&chrono::Utc));
        }
    }

    if let Some(date_str) = query.date_to {
        if let Ok(date) = chrono::DateTime::parse_from_rfc3339(&date_str) {
            filters.date_to = Some(date.with_timezone(&chrono::Utc));
        }
    }

    match state
        .application
        .get_research_history(&context, filters)
        .await
    {
        Ok(history) => {
            info!("Found {} research records", history.len());
            Ok(Json(history))
        }
        Err(e) => {
            error!("Failed to get research history: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get specific research record
#[utoipa::path(
    get,
    path = "/api/research/history/{session_id}",
    params(
        ("session_id" = String, Path, description = "Research session ID")
    ),
    responses(
        (status = 200, description = "Research record", body = ResearchHistoryRecord),
        (status = 403, description = "Permission denied"),
        (status = 404, description = "Record not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_research_record(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(session_id): Path<String>,
) -> Result<Json<ResearchHistoryRecord>, StatusCode> {
    info!(
        "Getting research record: {} (user: {})",
        session_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    match state
        .application
        .get_research_record(&context, &session_id)
        .await
    {
        Ok(Some(record)) => {
            info!("Found research record: {}", session_id);
            Ok(Json(record))
        }
        Ok(None) => {
            warn!("Research record not found or access denied: {}", session_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get research record: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Delete research record
#[utoipa::path(
    delete,
    path = "/api/research/history/{session_id}",
    params(
        ("session_id" = String, Path, description = "Research session ID")
    ),
    responses(
        (status = 200, description = "Record deleted successfully"),
        (status = 403, description = "Permission denied"),
        (status = 404, description = "Record not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_research_record(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Deleting research record: {} (user: {})",
        session_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    match state
        .application
        .delete_research_record(&context, &session_id)
        .await
    {
        Ok(()) => {
            info!("Research record deleted successfully: {}", session_id);
            Ok(Json(serde_json::json!({
                "message": "Research record deleted successfully"
            })))
        }
        Err(e) => {
            error!("Failed to delete research record: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else if e.to_string().contains("Access denied") {
                Err(StatusCode::FORBIDDEN)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Get research statistics (admin only)
#[utoipa::path(
    get,
    path = "/api/research/statistics",
    responses(
        (status = 200, description = "Research statistics", body = ResearchStatistics),
        (status = 403, description = "Permission denied (admin only)"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_research_statistics(
    State(state): State<AppState>,
    RequireManageSession(user): RequireManageSession, // Admin permission required
) -> Result<Json<ResearchStatistics>, StatusCode> {
    info!("Getting research statistics (admin: {})", user.id);

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    match state.application.get_research_statistics(&context).await {
        Ok(stats) => {
            info!("Retrieved research statistics");
            Ok(Json(stats))
        }
        Err(e) => {
            error!("Failed to get research statistics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
