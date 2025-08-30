//! Session Storage - Persistence layer for sessions
//!
//! Handles saving and loading session data with permission-aware design.

use super::{ApplicationSession, QueryContext, SessionConfig};
use crate::auth::UserIdentity;
use crate::{ApplicationError, ApplicationResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Serializable session data for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableSession {
    pub id: String,
    pub owner: UserIdentity,
    pub repository: super::RepositoryInfo,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub is_indexed: bool,
    pub indexing_progress: f64,
    pub config: SessionConfig,
    pub metadata: HashMap<String, String>,
    pub stats: super::SessionStats,
}

impl From<&ApplicationSession> for SerializableSession {
    fn from(session: &ApplicationSession) -> Self {
        Self {
            id: session.id.clone(),
            owner: session.owner.clone(),
            repository: session.repository.clone(),
            created_at: session.created_at,
            last_activity: session.last_activity,
            is_indexed: session.is_indexed,
            indexing_progress: session.indexing_progress,
            config: session.config.clone(),
            metadata: session.metadata.clone(),
            stats: session.stats.clone(),
        }
    }
}

impl From<SerializableSession> for ApplicationSession {
    fn from(serializable: SerializableSession) -> Self {
        Self {
            id: serializable.id,
            owner: serializable.owner,
            repository: serializable.repository,
            created_at: serializable.created_at,
            last_activity: serializable.last_activity,
            is_indexed: serializable.is_indexed,
            indexing_progress: serializable.indexing_progress,
            rag_pipeline: None, // RAG pipelines are not persisted
            config: serializable.config,
            metadata: serializable.metadata,
            stats: serializable.stats,
        }
    }
}

/// Session storage manager
pub struct SessionStorage {
    /// Base directory for session storage
    storage_dir: PathBuf,
}

impl SessionStorage {
    /// Create a new session storage manager
    pub fn new<P: AsRef<Path>>(storage_dir: P) -> ApplicationResult<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();

        // Create storage directory if it doesn't exist
        std::fs::create_dir_all(&storage_dir).map_err(ApplicationError::Io)?;

        info!("Session storage initialized at: {}", storage_dir.display());

        Ok(Self { storage_dir })
    }

    /// Save a session to disk
    pub fn save_session(&self, session: &ApplicationSession) -> ApplicationResult<()> {
        let serializable = SerializableSession::from(session);
        let session_file = self.storage_dir.join(format!("{}.json", session.id));

        let json_data =
            serde_json::to_string_pretty(&serializable).map_err(ApplicationError::Serialization)?;

        std::fs::write(&session_file, json_data).map_err(ApplicationError::Io)?;

        debug!("Saved session {} to {}", session.id, session_file.display());
        Ok(())
    }

    /// Load a session from disk
    pub fn load_session(&self, session_id: &str) -> ApplicationResult<ApplicationSession> {
        let session_file = self.storage_dir.join(format!("{}.json", session_id));

        if !session_file.exists() {
            return Err(ApplicationError::Session {
                message: format!("Session file not found: {}", session_id),
            });
        }

        let json_data = std::fs::read_to_string(&session_file).map_err(ApplicationError::Io)?;

        let serializable: SerializableSession =
            serde_json::from_str(&json_data).map_err(ApplicationError::Serialization)?;

        let session = ApplicationSession::from(serializable);
        debug!(
            "Loaded session {} from {}",
            session_id,
            session_file.display()
        );

        Ok(session)
    }

    /// Load all sessions from disk
    pub fn load_all_sessions(&self) -> ApplicationResult<HashMap<String, ApplicationSession>> {
        let mut sessions = HashMap::new();

        let entries = std::fs::read_dir(&self.storage_dir).map_err(ApplicationError::Io)?;

        for entry in entries {
            let entry = entry.map_err(ApplicationError::Io)?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Skip context files
                    if stem.ends_with("_context") {
                        continue;
                    }

                    match self.load_session(stem) {
                        Ok(session) => {
                            sessions.insert(session.id.clone(), session);
                        }
                        Err(e) => {
                            warn!("Failed to load session from {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        info!("Loaded {} sessions from storage", sessions.len());
        Ok(sessions)
    }

    /// Delete a session from disk
    pub fn delete_session(&self, session_id: &str) -> ApplicationResult<()> {
        let session_file = self.storage_dir.join(format!("{}.json", session_id));
        let context_file = self
            .storage_dir
            .join(format!("{}_context.json", session_id));

        // Remove session file
        if session_file.exists() {
            std::fs::remove_file(&session_file).map_err(ApplicationError::Io)?;
            debug!("Deleted session file: {}", session_file.display());
        }

        // Remove context file if it exists
        if context_file.exists() {
            std::fs::remove_file(&context_file).map_err(ApplicationError::Io)?;
            debug!("Deleted context file: {}", context_file.display());
        }

        Ok(())
    }

    /// Save query context to disk
    pub fn save_context(&self, context: &QueryContext) -> ApplicationResult<()> {
        let context_file = self
            .storage_dir
            .join(format!("{}_context.json", context.session_id));

        let json_data =
            serde_json::to_string_pretty(context).map_err(ApplicationError::Serialization)?;

        std::fs::write(&context_file, json_data).map_err(ApplicationError::Io)?;

        debug!(
            "Saved context for session {} to {}",
            context.session_id,
            context_file.display()
        );
        Ok(())
    }

    /// Load query context from disk
    pub fn load_context(&self, session_id: &str) -> ApplicationResult<QueryContext> {
        let context_file = self
            .storage_dir
            .join(format!("{}_context.json", session_id));

        if !context_file.exists() {
            // Return empty context if file doesn't exist
            return Ok(QueryContext::new(session_id.to_string(), 50));
        }

        let json_data = std::fs::read_to_string(&context_file).map_err(ApplicationError::Io)?;

        let context: QueryContext =
            serde_json::from_str(&json_data).map_err(ApplicationError::Serialization)?;

        debug!(
            "Loaded context for session {} from {}",
            session_id,
            context_file.display()
        );
        Ok(context)
    }

    /// Clean up old session files
    pub fn cleanup_old_sessions(&self, max_age_days: u32) -> ApplicationResult<usize> {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
        let mut cleaned_count = 0;

        let entries = std::fs::read_dir(&self.storage_dir).map_err(ApplicationError::Io)?;

        for entry in entries {
            let entry = entry.map_err(ApplicationError::Io)?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Skip context files, they'll be cleaned up with their sessions
                    if stem.ends_with("_context") {
                        continue;
                    }

                    match self.load_session(stem) {
                        Ok(session) => {
                            if session.last_activity < cutoff_time {
                                if let Err(e) = self.delete_session(&session.id) {
                                    warn!("Failed to delete old session {}: {}", session.id, e);
                                } else {
                                    cleaned_count += 1;
                                    info!("Cleaned up old session: {}", session.id);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to load session {} for cleanup: {}", stem, e);
                        }
                    }
                }
            }
        }

        info!("Cleaned up {} old sessions", cleaned_count);
        Ok(cleaned_count)
    }

    /// Get storage statistics
    pub fn get_storage_stats(&self) -> ApplicationResult<StorageStats> {
        let mut total_sessions = 0;
        let mut total_size = 0;

        let entries = std::fs::read_dir(&self.storage_dir).map_err(ApplicationError::Io)?;

        for entry in entries {
            let entry = entry.map_err(ApplicationError::Io)?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !stem.ends_with("_context") {
                        total_sessions += 1;
                    }
                }

                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }

        Ok(StorageStats {
            total_sessions,
            total_size_bytes: total_size,
            storage_dir: self.storage_dir.clone(),
        })
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_sessions: usize,
    pub total_size_bytes: u64,
    pub storage_dir: PathBuf,
}

impl StorageStats {
    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0)
    }

    pub fn summary(&self) -> String {
        format!(
            "Sessions: {}, Size: {:.2} MB, Dir: {}",
            self.total_sessions,
            self.total_size_mb(),
            self.storage_dir.display()
        )
    }
}
