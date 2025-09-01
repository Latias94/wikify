//! Repository management handlers

use super::types::{
    DeleteRepositoryResponse, InitializeRepositoryRequest, InitializeRepositoryResponse,
    ReindexResponse,
};
use crate::{auth::ModeAwareUser, AppState, WebError};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Json as JsonExtractor,
};
use tracing::{error, info, warn};

/// Helper function to convert User to PermissionContext for application layer
fn user_to_permission_context(user: &crate::auth::User) -> wikify_applications::PermissionContext {
    user.to_permission_context()
}

/// Helper function for auto-generating wiki after repository indexing
async fn generate_wiki_for_repository(
    state: &AppState,
    repository_id: &str,
    progress_sender: &tokio::sync::broadcast::Sender<crate::state::IndexingUpdate>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // This is a placeholder for wiki generation logic
    // In the actual implementation, this would call the wiki generation service
    info!("Auto-generating wiki for repository: {}", repository_id);

    // Send completion update
    let _ = progress_sender.send(crate::state::IndexingUpdate::WikiGenerationComplete {
        repository_id: repository_id.to_string(),
        wiki_content: "Wiki generated successfully".to_string(),
    });

    Ok(())
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
    ModeAwareUser(user): ModeAwareUser,
    JsonExtractor(request): JsonExtractor<InitializeRepositoryRequest>,
) -> Result<Json<InitializeRepositoryResponse>, StatusCode> {
    info!(
        "Initializing repository: {} (user: {})",
        request.repository, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    let auto_index = request.auto_index.unwrap_or(true);

    // Use new Repository API
    let repository_options = wikify_applications::RepositoryOptions {
        auto_index,
        metadata: request.metadata,
        access_mode: Some(wikify_applications::RepositoryAccessMode::Auto),
        api_token: None,
        extract_metadata: true,
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

            // Start listening to application progress updates and forward to web progress broadcaster
            let app_progress_receiver = state.application.subscribe_to_repository_progress();
            let web_progress_sender = state.progress_broadcaster.clone();
            let repo_id_clone = repository_id.clone();
            let auto_generate_wiki = request.auto_generate_wiki.unwrap_or(true);
            let state_clone = state.clone();

            info!(
                "Setting up progress forwarding for repository: {}",
                repository_id
            );

            tokio::spawn(async move {
                let mut receiver = app_progress_receiver;
                while let Ok(update) = receiver.recv().await {
                    info!("Received application progress update: {:?}", update);

                    // Convert application progress to web progress format
                    let web_update = match &update {
                        update
                            if update.status == wikify_applications::IndexingStatus::Indexing =>
                        {
                            crate::state::IndexingUpdate::Progress {
                                repository_id: update.repository_id.clone(),
                                stage: update.message.clone(),
                                percentage: update.progress * 100.0,
                                current_item: Some(format!(
                                    "Processing... {:.1}%",
                                    update.progress * 100.0
                                )),
                                files_processed: None,
                                total_files: None,
                            }
                        }
                        update
                            if update.status == wikify_applications::IndexingStatus::Completed =>
                        {
                            let complete_update = crate::state::IndexingUpdate::Complete {
                                repository_id: update.repository_id.clone(),
                                total_files: 0, // TODO: Get actual stats
                                total_chunks: 0,
                                duration_ms: 0,
                            };

                            // Auto-generate wiki if requested
                            if auto_generate_wiki && &update.repository_id == &repo_id_clone {
                                info!(
                                    "Auto-generating wiki for repository: {}",
                                    update.repository_id
                                );

                                // Send wiki generation started update
                                let _ = web_progress_sender.send(
                                    crate::state::IndexingUpdate::WikiGenerationStarted {
                                        repository_id: update.repository_id.clone(),
                                    },
                                );

                                // Generate wiki in background
                                let state_for_wiki = state_clone.clone();
                                let repo_id_for_wiki = update.repository_id.clone();
                                let progress_sender_for_wiki = web_progress_sender.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = generate_wiki_for_repository(
                                        &state_for_wiki,
                                        &repo_id_for_wiki,
                                        &progress_sender_for_wiki,
                                    )
                                    .await
                                    {
                                        error!("Failed to auto-generate wiki: {}", e);
                                        let _ = progress_sender_for_wiki.send(
                                            crate::state::IndexingUpdate::WikiGenerationError {
                                                repository_id: repo_id_for_wiki,
                                                error: e.to_string(),
                                            },
                                        );
                                    }
                                });
                            }

                            complete_update
                        }
                        update if update.status == wikify_applications::IndexingStatus::Failed => {
                            crate::state::IndexingUpdate::Error {
                                repository_id: update.repository_id.clone(),
                                error: update.message.clone(),
                            }
                        }
                        _ => {
                            // Handle other statuses as progress updates
                            crate::state::IndexingUpdate::Progress {
                                repository_id: update.repository_id.clone(),
                                stage: update.message.clone(),
                                percentage: update.progress * 100.0,
                                current_item: Some(format!("Status: {:?}", update.status)),
                                files_processed: None,
                                total_files: None,
                            }
                        }
                    };

                    info!("Sending web progress update: {:?}", web_update);

                    // Forward to web progress broadcaster
                    if let Err(e) = web_progress_sender.send(web_update) {
                        error!("Failed to send progress update: {}", e);
                    }
                }

                info!(
                    "Progress forwarding task ended for repository: {}",
                    repo_id_clone
                );
            });

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
    ModeAwareUser(user): ModeAwareUser,
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
    ModeAwareUser(user): ModeAwareUser,
    Path(repository_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Getting repository info for repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    match state
        .application
        .get_repository(&context, &repository_id)
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
    ModeAwareUser(user): ModeAwareUser,
    Path(repository_id): Path<String>,
) -> Result<Json<InitializeRepositoryResponse>, StatusCode> {
    info!(
        "Reindexing repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Check if repository exists using repository manager
    match state
        .application
        .get_repository(&context, &repository_id)
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
        .reindex_repository(&context, &repository_id)
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
    ModeAwareUser(user): ModeAwareUser,
    Path(repository_id): Path<String>,
) -> Result<Json<DeleteRepositoryResponse>, StatusCode> {
    info!("Deleting repository: {} (user: {})", repository_id, user.id);

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);
    match state.delete_repository(&repository_id).await {
        Ok(()) => Ok(Json(DeleteRepositoryResponse {
            status: "success".to_string(),
            message: "Repository deleted successfully".to_string(),
            deleted_repository_id: repository_id.clone(),
        })),
        Err(e) => {
            tracing::error!("Failed to delete repository {}: {}", repository_id, e);
            match e {
                WebError::NotFound(_) => Err(StatusCode::NOT_FOUND),
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
    crate::auth::RequireQuery(user): crate::auth::RequireQuery,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Getting repositories list (user: {})", user.id);

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);
    if let Some(database) = &state.database {
        match database.get_repositories().await {
            Ok(repositories) => {
                // Get current repository states from application layer
                let _context = state.create_anonymous_context();
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
