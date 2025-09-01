//! Wiki generation and management related types

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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

/// Wiki response structure
#[derive(Serialize, ToSchema)]
pub struct WikiResponse {
    #[schema(example = "repo_123")]
    pub id: String,
    #[schema(example = "Repository Wiki")]
    pub title: String,
    #[schema(example = "Generated wiki for repository")]
    pub description: String,
    pub pages: Vec<WikiPageResponse>,
    pub sections: Vec<serde_json::Value>,
}

/// Wiki page response structure
#[derive(Serialize, ToSchema)]
pub struct WikiPageResponse {
    #[schema(example = "main")]
    pub id: String,
    #[schema(example = "Main Documentation")]
    pub title: String,
    #[schema(example = "# Main Documentation\n\nThis is the main documentation...")]
    pub content: String,
    #[schema(example = "Main documentation page")]
    pub description: String,
    #[schema(example = "Critical")]
    pub importance: String,
    pub file_paths: Vec<String>,
    pub related_pages: Vec<String>,
    pub tags: Vec<String>,
    #[schema(example = 5)]
    pub reading_time: usize,
    #[schema(example = "2024-01-01T00:00:00Z")]
    pub generated_at: String,
    pub source_documents: Vec<String>,
}
