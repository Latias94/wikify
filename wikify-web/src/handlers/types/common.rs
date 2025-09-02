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
    /// Starting line number in the source file
    #[schema(example = 42)]
    pub start_line: Option<u32>,
    /// Ending line number in the source file
    #[schema(example = 58)]
    pub end_line: Option<u32>,
    /// Index of the chunk within the document
    #[schema(example = 3)]
    pub chunk_index: Option<u32>,
    /// Additional metadata about the source
    pub metadata: Option<serde_json::Value>,
}

/// Wiki generation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WikiConfig {
    /// Whether to include code examples in the wiki
    #[schema(example = true)]
    pub include_code_examples: bool,
    /// Maximum depth for wiki generation
    #[schema(example = 3)]
    pub max_depth: usize,
    /// Language for the generated wiki
    #[schema(example = "en")]
    pub language: Option<String>,
}

/// Wiki generation metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WikiMetadata {
    /// Time taken to generate the wiki in seconds
    #[schema(example = 45.2)]
    pub generation_time: f64,
    /// Total number of tokens used
    #[schema(example = 1500)]
    pub total_tokens: usize,
    /// AI model used for generation
    #[schema(example = "gpt-4")]
    pub model_used: String,
}
