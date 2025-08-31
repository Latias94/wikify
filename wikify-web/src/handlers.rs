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
    response::{Html, IntoResponse, Json},
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
    pub repository_id: String,
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
    pub deleted_repository_id: String,
}

/// Chat query request
#[derive(Deserialize, ToSchema)]
pub struct ChatQueryRequest {
    #[schema(example = "repo-uuid-string")]
    pub repository_id: String,
    #[schema(example = "How does the authentication work?")]
    pub question: String,
    pub context: Option<String>,
    /// Maximum number of results to return
    pub max_results: Option<usize>,
}

/// Chat query response
#[derive(Serialize, ToSchema)]
pub struct ChatQueryResponse {
    pub answer: String,
    pub sources: Vec<SourceDocument>,
    #[schema(example = "repo-uuid-string")]
    pub repository_id: String,
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
    pub repository_id: String,
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
    crate::auth::ModeAwareUser(user): crate::auth::ModeAwareUser,
    JsonExtractor(request): JsonExtractor<InitializeRepositoryRequest>,
) -> Result<Json<InitializeRepositoryResponse>, StatusCode> {
    info!(
        "Initializing repository: {} (user: {})",
        request.repository, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    let auto_index = request.auto_generate_wiki.unwrap_or(true);

    // Use new Repository API
    let repository_options = wikify_applications::RepositoryOptions {
        auto_index,
        metadata: None,
    };

    let repo_type = request.repo_type.clone().unwrap_or_else(|| {
        // Auto-detect repo type from URL
        if request.repository.contains("github.com") {
            "github".to_string()
        } else if request.repository.contains("gitlab.com") {
            "gitlab".to_string()
        } else {
            "local".to_string()
        }
    });

    match state
        .application
        .add_repository(
            &context,
            request.repository.clone(),
            repo_type,
            repository_options,
        )
        .await
    {
        Ok(repository_id) => {
            info!("Repository initialized successfully: {}", repository_id);
            Ok(Json(InitializeRepositoryResponse {
                repository_id,
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

/// List user repositories
#[utoipa::path(
    get,
    path = "/api/repositories",
    tag = "Repository",
    summary = "List repositories",
    description = "List all repositories accessible to the current user",
    responses(
        (status = 200, description = "Repositories listed successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_repositories(
    State(state): State<AppState>,
    crate::auth::ModeAwareUser(user): crate::auth::ModeAwareUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Listing repositories for user: {}", user.id);

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Use new Repository API
    match state.application.list_repositories(&context).await {
        Ok(repositories) => {
            let repo_list: Vec<serde_json::Value> = repositories
                .into_iter()
                .map(|repo| {
                    // Convert IndexingStatus to string
                    let status = match repo.status {
                        wikify_applications::IndexingStatus::Pending => "pending",
                        wikify_applications::IndexingStatus::Indexing => "indexing",
                        wikify_applications::IndexingStatus::Completed => "indexed",
                        wikify_applications::IndexingStatus::Failed => "failed",
                        wikify_applications::IndexingStatus::Cancelled => "cancelled",
                    };

                    serde_json::json!({
                        "id": repo.id,
                        "repository": repo.url,
                        "repo_type": repo.repo_type,
                        "status": status,
                        "indexing_progress": repo.progress,
                        "created_at": repo.created_at,
                        "last_indexed_at": repo.indexed_at,
                        "owner": repo.owner_id,
                        "metadata": repo.metadata
                    })
                })
                .collect();

            let response = serde_json::json!({
                "repositories": repo_list,
                "user": user.id,
                "permissions": user.permissions
            });

            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to list repositories: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get repository information
#[utoipa::path(
    get,
    path = "/api/repositories/{repository_id}",
    tag = "Repository",
    summary = "Get repository information",
    description = "Get information about a repository",
    params(
        ("repository_id" = String, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Repository information retrieved successfully"),
        (status = 404, description = "Repository not found")
    )
)]
pub async fn get_repository_info(
    State(state): State<AppState>,
    crate::auth::ModeAwareUser(user): crate::auth::ModeAwareUser,
    Path(repository_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Getting repository info for repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);

    match state
        .application
        .get_repository(&_context, &repository_id)
        .await
    {
        Ok(repository) => {
            let info = serde_json::json!({
                "repository_id": repository.id,
                "url": repository.url,
                "repo_type": repository.repo_type,
                "status": repository.status,
                "created_at": repository.created_at,
                "last_indexed_at": repository.indexed_at,
                "progress": repository.progress,
            });
            Ok(Json(info))
        }
        Err(_) => {
            warn!("Repository not found: {}", repository_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Reindex repository
#[utoipa::path(
    post,
    path = "/api/repositories/{repository_id}/reindex",
    tag = "Repository",
    summary = "Reindex repository",
    description = "Reindex an existing repository. If the repository is currently being indexed, returns a conflict error. If already indexed, resets the state and starts reindexing.",
    params(
        ("repository_id" = String, Path, description = "Repository ID to reindex")
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
    crate::auth::ModeAwareUser(user): crate::auth::ModeAwareUser,
    Path(repository_id): Path<String>,
) -> Result<Json<InitializeRepositoryResponse>, StatusCode> {
    info!(
        "Reindexing repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);

    // Check if repository exists using repository manager
    match state
        .application
        .get_repository(&_context, &repository_id)
        .await
    {
        Ok(_) => {
            info!(
                "Repository found, proceeding with reindexing: {}",
                repository_id
            );
        }
        Err(_) => {
            error!("Repository not found for reindexing: {}", repository_id);
            return Err(StatusCode::NOT_FOUND);
        }
    }

    // Start reindexing using the repository manager
    match state
        .application
        .reindex_repository(&_context, &repository_id)
        .await
    {
        Ok(()) => {
            let response = InitializeRepositoryResponse {
                repository_id: repository_id.clone(),
                status: "success".to_string(),
                message: "Repository reindexing started successfully".to_string(),
            };

            info!(
                "Repository reindexing started for repository: {}",
                repository_id
            );
            Ok(Json(response))
        }
        Err(e) => {
            error!(
                "Failed to start reindexing for repository {}: {}",
                repository_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Delete repository
#[utoipa::path(
    delete,
    path = "/api/repositories/{repository_id}",
    tag = "Repository",
    summary = "Delete repository",
    description = "Delete a repository and all associated data including sessions, vector data, and database records",
    params(
        ("repository_id" = String, Path, description = "Repository ID to delete")
    ),
    responses(
        (status = 200, description = "Repository deleted successfully", body = DeleteRepositoryResponse),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_repository(
    State(state): State<AppState>,
    crate::auth::ModeAwareUser(user): crate::auth::ModeAwareUser,
    Path(repository_id): Path<String>,
) -> Result<Json<DeleteRepositoryResponse>, StatusCode> {
    info!("Deleting repository: {} (user: {})", repository_id, user.id);

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
    match state.delete_repository(&repository_id).await {
        Ok(()) => Ok(Json(DeleteRepositoryResponse {
            status: "success".to_string(),
            message: "Repository deleted successfully".to_string(),
            deleted_repository_id: repository_id.clone(),
        })),
        Err(e) => {
            tracing::error!("Failed to delete repository {}: {}", repository_id, e);
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
                // Get current repository states from application layer
                let context = state.create_anonymous_context();
                // Note: Session-based status checking is no longer available
                // Repository status is now managed directly by the Repository Manager

                let repos_json: Vec<serde_json::Value> = repositories
                    .into_iter()
                    .map(|repo| {
                        // Repository status is now managed directly, no session lookup needed
                        let current_status = &repo.status;
                        let indexing_progress = 0.0; // Progress tracking moved to Repository Manager
                        let last_activity = repo.created_at.to_rfc3339(); // Use creation time as fallback

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
    crate::auth::ModeAwareUser(user): crate::auth::ModeAwareUser,
    JsonExtractor(request): JsonExtractor<ChatQueryRequest>,
) -> Result<Json<ChatQueryResponse>, StatusCode> {
    info!(
        "Processing chat query for repository: {} (user: {})",
        request.repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // No session activity update needed for repository-based queries

    // Repository-based query only
    let repository_id = &request.repository_id;

    let repo_query = wikify_applications::RepositoryQuery {
        question: request.question.clone(),
        max_results: request.max_results,
        parameters: None,
    };

    match state
        .application
        .query_repository(&context, &repository_id, repo_query)
        .await
    {
        Ok(repo_response) => {
            info!("✅ Repository query completed for: {}", repository_id);

            // Convert repository response to chat response format
            let sources: Vec<SourceDocument> = repo_response
                .sources
                .into_iter()
                .map(|source_path| SourceDocument {
                    file_path: source_path.clone(),
                    content: format!("Source: {}", source_path), // TODO: Get actual content
                    similarity_score: 0.8, // TODO: Get actual similarity score
                })
                .collect();

            let response = ChatQueryResponse {
                answer: repo_response.answer,
                sources,
                repository_id: repository_id.clone(),
                timestamp: chrono::Utc::now(),
            };

            // Save query to database if available
            #[cfg(feature = "sqlite")]
            if let Some(database) = &state.database {
                if let Err(e) = save_query_to_database(
                    database,
                    &response.repository_id,
                    &request.question,
                    &response.answer,
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
            error!("❌ Repository query failed for {}: {}", repository_id, e);

            let error_answer = format!(
                "Sorry, I encountered an error while processing your question: {}",
                e
            );

            // Save failed query to database if available
            #[cfg(feature = "sqlite")]
            if let Some(database) = &state.database {
                if let Err(db_e) = save_query_to_database(
                    database,
                    repository_id,
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
                repository_id: repository_id.clone(),
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
                            "repository_id": query.repository_id,
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
    State(state): State<AppState>,
    crate::auth::ModeAwareUser(user): crate::auth::ModeAwareUser,
    JsonExtractor(request): JsonExtractor<ChatQueryRequest>,
) -> Result<impl axum::response::IntoResponse, StatusCode> {
    info!(
        "Starting chat stream for user: {} with repository: {}",
        user.id, request.repository_id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    let repository_id = &request.repository_id;

    // Create repository query
    let repo_query = wikify_applications::RepositoryQuery {
        question: request.question.clone(),
        max_results: request.max_results,
        parameters: None, // TODO: Add support for additional parameters
    };

    // TODO: Implement streaming query when available
    // For now, return a placeholder response
    Ok(Json(serde_json::json!({
        "message": "Streaming queries not yet implemented for repository-based queries",
        "repository_id": repository_id
    }))
    .into_response())
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
        "Generating wiki for repository: {} (user: {})",
        request.repository_id, user.id
    );

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
    // Get repository info
    let repository = match state
        .application
        .get_repository(&_context, &request.repository_id)
        .await
    {
        Ok(repository) => repository,
        Err(_) => return Err(StatusCode::NOT_FOUND),
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

    // TODO: Implement wiki generation through application layer
    // For now, return a placeholder response
    let response = GenerateWikiResponse {
        wiki_id: request.repository_id.clone(), // Use repository_id as wiki_id
        status: "not_implemented".to_string(),
        pages_count: 0,
        sections_count: 0,
    };
    Ok(Json(response))
}

/// Get generated wiki
#[utoipa::path(
    get,
    path = "/api/wiki/{repository_id}",
    tag = "Wiki",
    summary = "Get generated wiki",
    description = "Retrieve the generated wiki documentation for a repository",
    params(
        ("repository_id" = String, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Wiki retrieved successfully"),
        (status = 404, description = "Wiki not found")
    )
)]
pub async fn get_wiki(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(repository_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Getting wiki for repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
    // Get repository info (verify repository exists)
    let _repository = match state
        .application
        .get_repository(&_context, &repository_id)
        .await
    {
        Ok(repository) => repository,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    // TODO: Implement wiki retrieval through application layer
    // For now, return not found
    Err(StatusCode::NOT_FOUND)
}

/// Export wiki in various formats
#[utoipa::path(
    post,
    path = "/api/wiki/{repository_id}/export",
    tag = "Wiki",
    summary = "Export wiki",
    description = "Export generated wiki in various formats (not yet implemented)",
    params(
        ("repository_id" = String, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Wiki exported successfully"),
        (status = 501, description = "Not implemented")
    )
)]
pub async fn export_wiki(
    State(_state): State<AppState>,
    RequireExport(user): RequireExport,
    Path(_repository_id): Path<String>,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Exporting wiki for repository: {} (user: {})",
        _repository_id, user.id
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
    /// Repository ID for the research
    pub repository_id: String,
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
    /// Research session ID
    pub research_id: String,
    /// Repository ID being researched
    pub repository_id: String,
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
            research_id: progress.session_id,
            repository_id: progress.repository_id,
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
        .start_research(
            &context,
            &request.repository_id,
            request.topic,
            request.config,
        )
        .await
    {
        Ok(research_session_id) => {
            info!(
                "Research started successfully with session ID: {}",
                research_session_id
            );
            // Return progress for the newly created session
            match state
                .application
                .get_research_progress(&context, &research_session_id)
                .await
            {
                Ok(progress) => Ok(Json(ResearchProgressResponse::from(progress))),
                Err(e) => {
                    error!("Failed to get initial research progress: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            error!("Failed to start research: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Execute one research iteration
#[utoipa::path(
    post,
    path = "/api/research/iterate/{repository_id}",
    params(
        ("repository_id" = String, Path, description = "Repository ID for research")
    ),
    responses(
        (status = 200, description = "Research iteration completed", body = ResearchProgressResponse),
        (status = 403, description = "Permission denied"),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn research_iteration(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(repository_id): Path<String>,
) -> Result<Json<ResearchProgressResponse>, StatusCode> {
    info!(
        "Executing research iteration for repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Execute research iteration using application layer
    match state
        .application
        .research_iteration(&context, &repository_id, &repository_id) // Using repository_id as research_session_id
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
    path = "/api/research/progress/{repository_id}",
    params(
        ("repository_id" = String, Path, description = "Repository ID")
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
    Path(repository_id): Path<String>,
) -> Result<Json<ResearchProgressResponse>, StatusCode> {
    info!(
        "Getting research progress for repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Get research progress using application layer
    match state
        .application
        .get_research_progress(&context, &repository_id)
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

/// List active research sessions
#[utoipa::path(
    get,
    path = "/api/research/sessions",
    responses(
        (status = 200, description = "List of active research sessions", body = Vec<String>),
        (status = 403, description = "Permission denied"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_research_sessions(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
) -> Result<Json<Vec<String>>, StatusCode> {
    info!("Listing active research sessions (user: {})", user.id);

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // List research sessions using application layer
    match state.application.list_active_research(&context).await {
        Ok(sessions) => {
            info!("Found {} active research sessions", sessions.len());
            Ok(Json(sessions))
        }
        Err(e) => {
            error!("Failed to list research sessions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get research progress by research ID (alias for frontend compatibility)
#[utoipa::path(
    get,
    path = "/api/research/{research_id}",
    tag = "Research",
    summary = "Get research progress by ID",
    description = "Get research progress using research session ID (frontend compatibility alias)",
    params(
        ("research_id" = String, Path, description = "Research session ID")
    ),
    responses(
        (status = 200, description = "Research progress retrieved", body = ResearchProgressResponse),
        (status = 403, description = "Permission denied"),
        (status = 404, description = "Research session not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_research_progress_by_id(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(research_id): Path<String>,
) -> Result<Json<ResearchProgressResponse>, StatusCode> {
    // This is just an alias for get_research_progress
    get_research_progress(State(state), RequireQuery(user), Path(research_id)).await
}

/// Stop research session
#[utoipa::path(
    post,
    path = "/api/research/{research_id}/stop",
    tag = "Research",
    summary = "Stop research session",
    description = "Stop an active research session",
    params(
        ("research_id" = String, Path, description = "Research session ID")
    ),
    responses(
        (status = 200, description = "Research stopped successfully"),
        (status = 403, description = "Permission denied"),
        (status = 404, description = "Research session not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn stop_research(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(_research_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Stopping research session (user: {})", user.id);

    // For now, return a placeholder response
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Research session stopped (not yet implemented)"
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
    repository_id: &str,
    question: &str,
    answer: &str,
) -> crate::WebResult<()> {
    use crate::simple_database::SimpleQuery;

    let query = SimpleQuery {
        id: uuid::Uuid::new_v4().to_string(),
        repository_id: Some(repository_id.to_string()),
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

    // TODO: Research Engine is being re-implemented
    error!("Research Engine is currently being re-implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
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

    // TODO: Research Engine is being re-implemented
    error!("Research Engine is currently being re-implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
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

    // TODO: Research Engine is being re-implemented
    error!("Research Engine is currently being re-implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
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

    // TODO: Research Engine is being re-implemented
    error!("Research Engine is currently being re-implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
}

// Research History API endpoints

/// Research history query parameters
#[derive(Debug, Deserialize)]
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
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("template_id" = Option<String>, Query, description = "Filter by template ID"),
        ("date_from" = Option<String>, Query, description = "Filter by date from (ISO 8601)"),
        ("date_to" = Option<String>, Query, description = "Filter by date to (ISO 8601)"),
        ("limit" = Option<usize>, Query, description = "Limit number of results"),
        ("offset" = Option<usize>, Query, description = "Offset for pagination")
    ),
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
            "in_progress" => {
                Some(wikify_applications::research::history::ResearchStatus::InProgress)
            }
            "completed" => Some(wikify_applications::research::history::ResearchStatus::Completed),
            "cancelled" => Some(wikify_applications::research::history::ResearchStatus::Cancelled),
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

    // TODO: Research Engine is being re-implemented
    error!("Research Engine is currently being re-implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
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

    // TODO: Research Engine is being re-implemented
    error!("Research Engine is currently being re-implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
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

    // TODO: Research Engine is being re-implemented
    error!("Research Engine is currently being re-implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
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

    // TODO: Research Engine is being re-implemented
    error!("Research Engine is currently being re-implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
}
