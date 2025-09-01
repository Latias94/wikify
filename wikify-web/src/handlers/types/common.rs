//! Common types used across multiple handlers

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Health check response
#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    #[schema(example = "healthy")]
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[schema(example = "0.1.0")]
    pub version: String,
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
