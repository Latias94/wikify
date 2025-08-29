//! Core data type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub owner: String,
    pub name: String,
    pub repo_type: RepoType,
    pub url: String,
    pub access_token: Option<String>,
    pub local_path: Option<String>,
    pub access_mode: RepoAccessMode,
}

/// Supported repository types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoType {
    GitHub,
    GitLab,
    Bitbucket,
    Gitea,
    Local,
}

/// Repository access mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoAccessMode {
    /// Clone the entire repository using git
    GitClone,
    /// Access files via API without cloning
    Api,
}

/// Document information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: String,
    pub file_path: String,
    pub content: String,
    pub file_type: FileType,
    pub metadata: HashMap<String, String>,
    pub token_count: usize,
}

/// 文件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    Code { language: String },
    Documentation,
    Configuration,
    Other,
}

/// Wiki页面
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub id: String,
    pub title: String,
    pub content: String,
    pub file_paths: Vec<String>,
    pub importance: Importance,
    pub related_pages: Vec<String>,
    pub sections: Vec<WikiSection>,
}

/// Wiki章节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiSection {
    pub id: String,
    pub title: String,
    pub content: String,
    pub subsections: Vec<WikiSection>,
}

/// 重要性级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Importance {
    High,
    Medium,
    Low,
}

/// Wiki结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiStructure {
    pub id: String,
    pub title: String,
    pub description: String,
    pub pages: Vec<WikiPage>,
    pub root_sections: Vec<String>,
}

/// RAG查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    pub answer: String,
    pub sources: Vec<DocumentInfo>,
    pub confidence: f32,
}

/// 深度研究状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchState {
    pub iteration: usize,
    pub max_iterations: usize,
    pub topic: String,
    pub findings: Vec<String>,
    pub is_complete: bool,
}

/// 配置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikifyConfig {
    pub embedding: EmbeddingConfig,
    pub llm: LlmConfig,
    pub repository: RepositoryConfig,
    pub storage: StorageConfig,
    pub rag: RagConfig,
    pub indexing: IndexingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub dimensions: usize,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub max_size_mb: usize,
    pub excluded_dirs: Vec<String>,
    pub excluded_files: Vec<String>,
    pub included_extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: String,
    pub cache_dir: String,
    pub use_database: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Similarity threshold for document retrieval (0.0-1.0)
    pub similarity_threshold: f32,
    /// Number of top documents to retrieve
    pub top_k: usize,
    /// Maximum context length for RAG
    pub max_context_length: usize,
    /// Whether to enable reranking
    pub enable_reranking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Chunk size for text splitting (in characters)
    pub chunk_size: usize,
    /// Overlap between chunks (in characters)
    pub chunk_overlap: usize,
    /// Whether to use sentence-aware splitting
    pub sentence_aware: bool,
    /// Whether to use token-based splitting for code files
    pub token_based_for_code: bool,
    /// Maximum tokens per chunk (for token-based splitting)
    pub max_tokens_per_chunk: usize,
    /// Whether to preserve markdown structure
    pub preserve_markdown_structure: bool,
    /// Whether to use AST-aware code splitting
    pub use_ast_code_splitting: bool,
    /// Maximum file size to process (in MB)
    pub max_file_size_mb: u64,
    /// Maximum number of files to process
    pub max_files: Option<usize>,
}
