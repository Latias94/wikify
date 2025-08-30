//! Session Types and Structures
//!
//! Defines all session-related types with permission-aware design.

use crate::auth::{PermissionContext, UserIdentity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wikify_rag::RagPipeline;

/// Application session with permission-aware design
pub struct ApplicationSession {
    /// Unique session identifier
    pub id: String,
    /// Session owner (user who created the session)
    pub owner: UserIdentity,
    /// Repository information
    pub repository: RepositoryInfo,
    /// Session creation and activity timestamps
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    /// Indexing status
    pub is_indexed: bool,
    pub indexing_progress: f64,
    /// RAG pipeline (None if not initialized)
    pub rag_pipeline: Option<RagPipeline>,
    /// Session configuration
    pub config: SessionConfig,
    /// Session metadata
    pub metadata: HashMap<String, String>,
    /// Session statistics
    pub stats: SessionStats,
}

impl ApplicationSession {
    /// Create a new application session
    pub fn new(owner: UserIdentity, repository: RepositoryInfo, config: SessionConfig) -> Self {
        let now = Utc::now();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            owner,
            repository,
            created_at: now,
            last_activity: now,
            is_indexed: false,
            indexing_progress: 0.0,
            rag_pipeline: None,
            config,
            metadata: HashMap::new(),
            stats: SessionStats::default(),
        }
    }

    /// Update the last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Check if the session is ready for queries
    pub fn is_ready(&self) -> bool {
        self.is_indexed && self.rag_pipeline.is_some()
    }

    /// Get session age in minutes
    pub fn age_minutes(&self) -> i64 {
        (Utc::now() - self.created_at).num_minutes()
    }

    /// Check if session is stale based on configuration
    pub fn is_stale(&self) -> bool {
        let timeout_minutes = self.config.session_timeout_minutes;
        (Utc::now() - self.last_activity).num_minutes() > timeout_minutes as i64
    }

    /// Check if a user can access this session
    pub fn can_access(&self, context: &PermissionContext) -> bool {
        // Session owner can always access
        if let Some(user_id) = context.user_id() {
            if user_id == self.owner.user_id {
                return true;
            }
        }

        // Admin users can access any session
        if context.is_admin() {
            return true;
        }

        // Local mode allows access to all sessions
        if context.is_local() {
            return true;
        }

        // Check if repository is public
        self.repository.visibility == RepositoryVisibility::Public
    }

    /// Increment query count
    pub fn record_query(&mut self) {
        self.stats.total_queries += 1;
        self.update_activity();
    }

    /// Set indexing progress
    pub fn set_indexing_progress(&mut self, progress: f64) {
        self.indexing_progress = progress;
        if progress >= 1.0 {
            self.is_indexed = true;
        }
        self.update_activity();
    }
}

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// Repository identifier (URL or path)
    pub id: String,
    /// Repository URL or local path
    pub url: String,
    /// Repository type
    pub repo_type: RepositoryType,
    /// Repository visibility
    pub visibility: RepositoryVisibility,
    /// Repository owner (optional)
    pub owner: Option<String>,
    /// Repository metadata
    pub metadata: HashMap<String, String>,
}

impl RepositoryInfo {
    /// Create repository info from URL
    pub fn from_url(url: String) -> Self {
        let repo_type = if url.starts_with("http") {
            RepositoryType::Remote
        } else {
            RepositoryType::Local
        };

        Self {
            id: url.clone(),
            url,
            repo_type,
            visibility: RepositoryVisibility::Public, // Default to public
            owner: None,
            metadata: HashMap::new(),
        }
    }

    /// Set repository visibility
    pub fn with_visibility(mut self, visibility: RepositoryVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Set repository owner
    pub fn with_owner(mut self, owner: String) -> Self {
        self.owner = Some(owner);
        self
    }
}

/// Repository type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RepositoryType {
    /// Local repository (file system path)
    Local,
    /// Remote repository (GitHub, GitLab, etc.)
    Remote,
}

/// Repository visibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RepositoryVisibility {
    /// Public repository (anyone can access)
    Public,
    /// Internal repository (registered users can access)
    Internal,
    /// Private repository (only owner and authorized users)
    Private,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Whether to automatically generate wiki after indexing
    pub auto_generate_wiki: bool,
    /// Session timeout in minutes
    pub session_timeout_minutes: u32,
    /// Whether to persist session to disk
    pub persist_session: bool,
    /// Maximum conversation history length
    pub max_conversation_history: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_generate_wiki: false,
            session_timeout_minutes: 480, // 8 hours
            persist_session: true,
            max_conversation_history: 50,
        }
    }
}

/// Session creation options
#[derive(Debug, Clone, Default)]
pub struct SessionOptions {
    /// Repository visibility override
    pub visibility: Option<RepositoryVisibility>,
    /// Whether to automatically generate wiki
    pub auto_generate_wiki: bool,
    /// Custom session configuration
    pub config: Option<SessionConfig>,
    /// Initial metadata
    pub metadata: HashMap<String, String>,
}

/// Session statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    /// Total number of queries performed
    pub total_queries: u64,
    /// Total number of wiki generations
    pub total_wiki_generations: u64,
    /// Total number of research sessions
    pub total_research_sessions: u64,
    /// Last query timestamp
    pub last_query_at: Option<DateTime<Utc>>,
}

/// Session information for external consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub owner_id: String,
    pub owner_display_name: Option<String>,
    pub repository: RepositoryInfo,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_indexed: bool,
    pub indexing_progress: f64,
    pub is_ready: bool,
    pub age_minutes: i64,
    pub stats: SessionStats,
    pub metadata: HashMap<String, String>,
}

impl From<&ApplicationSession> for SessionInfo {
    fn from(session: &ApplicationSession) -> Self {
        Self {
            id: session.id.clone(),
            owner_id: session.owner.user_id.clone(),
            owner_display_name: session.owner.display_name.clone(),
            repository: session.repository.clone(),
            created_at: session.created_at,
            last_activity: session.last_activity,
            is_indexed: session.is_indexed,
            indexing_progress: session.indexing_progress,
            is_ready: session.is_ready(),
            age_minutes: session.age_minutes(),
            stats: session.stats.clone(),
            metadata: session.metadata.clone(),
        }
    }
}

/// Indexing progress update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingUpdate {
    pub session_id: String,
    pub progress: f64,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub is_complete: bool,
    pub error: Option<String>,
}

impl IndexingUpdate {
    /// Create a progress update
    pub fn progress(session_id: String, progress: f64, message: String) -> Self {
        Self {
            session_id,
            progress,
            message,
            timestamp: Utc::now(),
            is_complete: false,
            error: None,
        }
    }

    /// Create a completion update
    pub fn complete(session_id: String, message: String) -> Self {
        Self {
            session_id,
            progress: 1.0,
            message,
            timestamp: Utc::now(),
            is_complete: true,
            error: None,
        }
    }

    /// Create an error update
    pub fn error(session_id: String, error: String) -> Self {
        Self {
            session_id,
            progress: 0.0,
            message: "Indexing failed".to_string(),
            timestamp: Utc::now(),
            is_complete: true,
            error: Some(error),
        }
    }
}

/// Query context for conversation management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryContext {
    pub session_id: String,
    pub conversation_history: Vec<QueryMessage>,
    pub max_context_length: usize,
}

impl QueryContext {
    /// Create a new query context
    pub fn new(session_id: String, max_context_length: usize) -> Self {
        Self {
            session_id,
            conversation_history: Vec::new(),
            max_context_length,
        }
    }

    /// Add a message to the conversation history
    pub fn add_message(&mut self, message: QueryMessage) {
        self.conversation_history.push(message);

        // Trim history if it exceeds the maximum length
        if self.conversation_history.len() > self.max_context_length {
            self.conversation_history
                .drain(0..self.conversation_history.len() - self.max_context_length);
        }
    }

    /// Get recent messages for context
    pub fn get_recent_messages(&self, count: usize) -> Vec<&QueryMessage> {
        self.conversation_history
            .iter()
            .rev()
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Build conversation context string
    pub fn build_context_string(&self, max_messages: usize) -> String {
        if self.conversation_history.is_empty() {
            return String::new();
        }

        let mut context = String::new();
        context.push_str("Previous conversation:\n");

        let recent_messages = self.get_recent_messages(max_messages);
        for message in recent_messages {
            context.push_str(&format!("{}: {}\n", message.role, message.content));
        }

        context
    }
}

/// A message in the query conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl QueryMessage {
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}
