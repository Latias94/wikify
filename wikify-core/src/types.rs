//! Core data type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Repository information - basic metadata about a repository
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

/// Repository access handle - encapsulates all information needed to access a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryAccess {
    /// Basic repository information
    pub repo_info: RepoInfo,
    /// Determined access mode (after auto-detection)
    pub access_mode: RepoAccessMode,
    /// Local path for GitClone and LocalDirectory modes
    pub local_path: Option<std::path::PathBuf>,
    /// Whether the repository is ready for access
    pub is_ready: bool,
}

/// Repository file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryFile {
    /// Relative path from repository root
    pub path: String,
    /// File type (blob, tree, etc.)
    pub file_type: String,
    /// File size in bytes
    pub size: Option<u64>,
    /// Git SHA hash (if available)
    pub sha: Option<String>,
    /// Last modified time (if available)
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
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

/// Repository access mode - three distinct ways to access repository content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoAccessMode {
    /// Access remote repository via API (fast, minimal storage, requires network)
    Api,
    /// Clone remote repository locally (complete, offline capable, more storage)
    GitClone,
    /// Access local directory directly (immediate, no network, user-provided path)
    LocalDirectory,
}

/// Repository access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryAccessConfig {
    /// Preferred access mode (None = auto-detect)
    pub preferred_mode: Option<RepoAccessMode>,
    /// API token for authenticated access
    pub api_token: Option<String>,
    /// Force the specified mode (ignore auto-detection)
    pub force_mode: bool,
    /// Clone depth for GitClone mode (None = full clone, Some(1) = shallow)
    pub clone_depth: Option<u32>,
    /// Custom local path for GitClone mode
    pub custom_local_path: Option<String>,
}

impl Default for RepositoryAccessConfig {
    fn default() -> Self {
        Self {
            preferred_mode: None, // Auto-detect
            api_token: None,
            force_mode: false,
            clone_depth: Some(1), // Shallow clone by default
            custom_local_path: None,
        }
    }
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

// Wiki相关类型已移动到wikify-wiki crate中
// 这里保留核心的基础类型定义

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
