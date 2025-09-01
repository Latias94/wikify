//! File operations handlers

use super::types::{
    FileContentResponse, FileTreeResponse, GetFileContentRequest, GetFileTreeRequest,
    GetReadmeRequest, ReadmeResponse, RepositoryFileInfo,
};
use crate::{auth::RequireQuery, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, Json},
    Json as JsonExtractor,
};
use tracing::{error, info};

/// Helper function to convert User to PermissionContext for application layer
fn user_to_permission_context(user: &crate::auth::User) -> wikify_applications::PermissionContext {
    user.to_permission_context()
}

/// Get file tree for repository
#[utoipa::path(
    post,
    path = "/api/files/tree",
    tag = "Files",
    request_body = GetFileTreeRequest,
    responses(
        (status = 200, description = "File tree retrieved successfully", body = FileTreeResponse),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_file_tree(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    JsonExtractor(request): JsonExtractor<GetFileTreeRequest>,
) -> Result<Json<FileTreeResponse>, StatusCode> {
    info!(
        "Getting file tree for repository: {}",
        request.repository_id
    );

    let context = user_to_permission_context(&user);
    match state
        .application
        .get_repository_file_tree(&context, &request.repository_id, request.branch.clone())
        .await
    {
        Ok(files) => {
            info!("File tree retrieved successfully: {} files", files.len());
            let file_infos: Vec<RepositoryFileInfo> =
                files.into_iter().map(RepositoryFileInfo::from).collect();

            Ok(Json(FileTreeResponse {
                repository_id: request.repository_id,
                branch: request.branch,
                total_files: file_infos.len(),
                files: file_infos,
            }))
        }
        Err(e) => {
            error!("Failed to get file tree: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Get file content
#[utoipa::path(
    post,
    path = "/api/files/content",
    tag = "Files",
    request_body = GetFileContentRequest,
    responses(
        (status = 200, description = "File content retrieved successfully", body = FileContentResponse),
        (status = 404, description = "File not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_file_content(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    JsonExtractor(request): JsonExtractor<GetFileContentRequest>,
) -> Result<Json<FileContentResponse>, StatusCode> {
    info!(
        "Getting file content for: {}/{}",
        request.repository_id, request.file_path
    );

    let context = user_to_permission_context(&user);
    match state
        .application
        .get_repository_file_content(
            &context,
            &request.repository_id,
            &request.file_path,
            request.branch.clone(),
        )
        .await
    {
        Ok(content) => {
            info!(
                "File content retrieved successfully: {} bytes",
                content.len()
            );
            Ok(Json(FileContentResponse {
                repository_id: request.repository_id,
                file_path: request.file_path,
                branch: request.branch,
                size: content.len(),
                encoding: "utf-8".to_string(),
                content,
            }))
        }
        Err(e) => {
            error!("Failed to get file content: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Get README content for repository
#[utoipa::path(
    post,
    path = "/api/files/readme",
    tag = "Files",
    request_body = GetReadmeRequest,
    responses(
        (status = 200, description = "README content retrieved successfully", body = ReadmeResponse),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_readme(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    JsonExtractor(request): JsonExtractor<GetReadmeRequest>,
) -> Result<Json<ReadmeResponse>, StatusCode> {
    info!("Getting README for repository: {}", request.repository_id);

    let context = user_to_permission_context(&user);
    match state
        .application
        .get_repository_readme(&context, &request.repository_id, request.branch.clone())
        .await
    {
        Ok(content) => {
            let found = content.is_some();
            info!("README retrieval completed: found={}", found);
            Ok(Json(ReadmeResponse {
                repository_id: request.repository_id,
                branch: request.branch,
                content,
                found,
            }))
        }
        Err(e) => {
            error!("Failed to get README: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// SPA fallback handler
pub async fn spa_fallback() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}
