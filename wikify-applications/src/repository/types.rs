use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Repository indexing status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IndexingStatus {
    /// Repository is queued for indexing
    Pending,
    /// Repository is currently being indexed
    Indexing,
    /// Repository has been successfully indexed
    Completed,
    /// Repository indexing failed
    Failed,
    /// Repository indexing was cancelled
    Cancelled,
}

impl Default for IndexingStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Repository information and indexing state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryIndex {
    /// Unique repository ID
    pub id: String,
    /// Repository URL
    pub url: String,
    /// Repository type (github, gitlab, local, etc.)
    pub repo_type: String,
    /// Current indexing status
    pub status: IndexingStatus,
    /// Indexing progress (0.0 to 1.0)
    pub progress: f64,
    /// When the repository was added
    pub created_at: DateTime<Utc>,
    /// When the repository was last indexed
    pub indexed_at: Option<DateTime<Utc>>,
    /// When the indexing was last updated
    pub updated_at: DateTime<Utc>,
    /// User who added this repository (None for anonymous)
    pub owner_id: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl RepositoryIndex {
    pub fn new(url: String, repo_type: String, owner_id: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            url,
            repo_type,
            status: IndexingStatus::Pending,
            progress: 0.0,
            created_at: now,
            indexed_at: None,
            updated_at: now,
            owner_id,
            metadata: HashMap::new(),
        }
    }

    /// Update indexing progress
    pub fn set_progress(&mut self, progress: f64, status: Option<IndexingStatus>) {
        self.progress = progress.clamp(0.0, 1.0);
        if let Some(status) = status {
            self.status = status;
        }
        self.updated_at = Utc::now();

        // Set indexed_at when completed
        if matches!(self.status, IndexingStatus::Completed) {
            self.indexed_at = Some(Utc::now());
        }
    }

    /// Mark as failed with error message
    pub fn set_failed(&mut self, error_message: String) {
        self.status = IndexingStatus::Failed;
        self.updated_at = Utc::now();
        self.metadata.insert("error".to_string(), error_message);
    }

    /// Check if repository is ready for querying
    pub fn is_ready(&self) -> bool {
        matches!(self.status, IndexingStatus::Completed)
    }
}

/// Options for adding a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryOptions {
    /// Whether to automatically start indexing
    pub auto_index: bool,
    /// Additional metadata
    pub metadata: Option<HashMap<String, String>>,
}

impl Default for RepositoryOptions {
    fn default() -> Self {
        Self {
            auto_index: true,
            metadata: None,
        }
    }
}

/// Repository query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryQuery {
    /// The question to ask
    pub question: String,
    /// Maximum number of results to return
    pub max_results: Option<usize>,
    /// Additional query parameters
    pub parameters: Option<HashMap<String, String>>,
}

/// Repository query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryQueryResponse {
    /// The answer to the question
    pub answer: String,
    /// Source documents used to generate the answer
    pub sources: Vec<String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: Option<f64>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Stream chunk for real-time query responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStreamChunk {
    /// Chunk type
    pub chunk_type: QueryChunkType,
    /// Content of this chunk
    pub content: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Sources found so far (only in final chunk)
    pub sources: Option<Vec<String>>,
    /// Metadata (only in final chunk)
    pub metadata: Option<HashMap<String, String>>,
}

/// Types of query stream chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryChunkType {
    /// Text content chunk
    Content,
    /// Source document found
    Source,
    /// Error occurred
    Error,
    /// Query completed
    Complete,
}

/// Indexing progress update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingUpdate {
    /// Repository ID
    pub repository_id: String,
    /// Current status
    pub status: IndexingStatus,
    /// Progress (0.0 to 1.0)
    pub progress: f64,
    /// Status message
    pub message: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl IndexingUpdate {
    pub fn progress(repository_id: String, progress: f64, message: String) -> Self {
        Self {
            repository_id,
            status: IndexingStatus::Indexing,
            progress: progress.clamp(0.0, 1.0),
            message,
            timestamp: Utc::now(),
        }
    }

    pub fn complete(repository_id: String, message: String) -> Self {
        Self {
            repository_id,
            status: IndexingStatus::Completed,
            progress: 1.0,
            message,
            timestamp: Utc::now(),
        }
    }

    pub fn error(repository_id: String, error_message: String) -> Self {
        Self {
            repository_id,
            status: IndexingStatus::Failed,
            progress: 0.0,
            message: error_message,
            timestamp: Utc::now(),
        }
    }
}
