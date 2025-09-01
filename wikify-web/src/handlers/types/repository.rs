//! Repository-related types

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Repository initialization request
#[derive(Deserialize, ToSchema)]
pub struct InitializeRepositoryRequest {
    #[schema(example = "https://github.com/user/repo")]
    pub repository: String,
    #[schema(example = "github")]
    pub repo_type: Option<String>, // "github", "local", etc.
    pub access_token: Option<String>,
    #[schema(example = true)]
    pub auto_index: Option<bool>, // Whether to automatically index the repository
    #[schema(example = true)]
    pub auto_generate_wiki: Option<bool>, // Whether to automatically generate wiki after indexing
    pub metadata: Option<std::collections::HashMap<String, String>>, // Additional metadata
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

/// Reindex response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReindexResponse {
    pub session_id: String,
    pub status: String,
    pub message: String,
}
