//! File operation types

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request to get file tree
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetFileTreeRequest {
    pub repository_id: String,
    pub branch: Option<String>,
}

/// Response containing file tree
#[derive(Debug, Serialize, ToSchema)]
pub struct FileTreeResponse {
    pub repository_id: String,
    pub branch: Option<String>,
    pub files: Vec<RepositoryFileInfo>,
    pub total_files: usize,
}

/// File information in the repository
#[derive(Debug, Serialize, ToSchema)]
pub struct RepositoryFileInfo {
    pub path: String,
    pub file_type: String,
    pub size: Option<u64>,
    pub sha: Option<String>,
}

/// Request to get file content
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetFileContentRequest {
    pub repository_id: String,
    pub file_path: String,
    pub branch: Option<String>,
}

/// Response containing file content
#[derive(Debug, Serialize, ToSchema)]
pub struct FileContentResponse {
    pub repository_id: String,
    pub file_path: String,
    pub branch: Option<String>,
    pub content: String,
    pub size: usize,
    pub encoding: String,
}

/// Request to get README content
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetReadmeRequest {
    pub repository_id: String,
    pub branch: Option<String>,
}

/// Response containing README content
#[derive(Debug, Serialize, ToSchema)]
pub struct ReadmeResponse {
    pub repository_id: String,
    pub branch: Option<String>,
    pub content: Option<String>,
    pub found: bool,
}

// Conversion implementations
impl From<wikify_repo::api::RepositoryFile> for RepositoryFileInfo {
    fn from(file: wikify_repo::api::RepositoryFile) -> Self {
        Self {
            path: file.path,
            file_type: file.file_type,
            size: file.size,
            sha: file.sha,
        }
    }
}

impl From<wikify_core::RepositoryFile> for RepositoryFileInfo {
    fn from(file: wikify_core::RepositoryFile) -> Self {
        Self {
            path: file.path,
            file_type: file.file_type,
            size: file.size,
            sha: file.sha,
        }
    }
}
