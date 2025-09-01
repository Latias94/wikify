//! Chat and RAG query related types

use super::common::SourceDocument;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
