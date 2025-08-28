//! Persistent storage for vector databases and chat sessions
//!
//! This module provides persistent storage capabilities for vector databases
//! and chat session management.

use crate::embeddings::VectorStore;
use crate::types::{ChatMessage, ChatSession, EmbeddedChunk, RagError, RagResult, StorageConfig};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Persistent vector database with file-based storage
pub struct PersistentVectorStore {
    /// In-memory vector store
    vector_store: VectorStore,
    /// Storage configuration
    config: StorageConfig,
    /// Repository identifier (hash of repo path/URL)
    repo_id: String,
    /// Storage file path
    storage_path: PathBuf,
    /// Whether the store has been modified since last save
    dirty: bool,
}

impl PersistentVectorStore {
    /// Create a new persistent vector store
    pub fn new(config: StorageConfig, repo_path: &str, dimension: usize) -> RagResult<Self> {
        // Generate repository ID from path/URL
        let repo_id = Self::generate_repo_id(repo_path);

        // Create storage directory
        let storage_dir = config.base_dir.join(&repo_id);
        std::fs::create_dir_all(&storage_dir).map_err(|e| RagError::Io(e))?;

        let storage_path = storage_dir.join("vectors.json");

        // Try to load existing data
        let vector_store = if storage_path.exists() && config.enable_persistence {
            info!("Loading existing vector store from {:?}", storage_path);
            Self::load_vector_store(&storage_path, dimension)?
        } else {
            info!("Creating new vector store");
            VectorStore::new(dimension)
        };

        Ok(Self {
            vector_store,
            config,
            repo_id,
            storage_path,
            dirty: false,
        })
    }

    /// Generate a unique repository ID from path/URL
    fn generate_repo_id(repo_path: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        repo_path.hash(&mut hasher);
        format!("repo_{:x}", hasher.finish())
    }

    /// Load vector store from file
    fn load_vector_store(path: &Path, dimension: usize) -> RagResult<VectorStore> {
        let data = std::fs::read_to_string(path).map_err(|e| RagError::Io(e))?;

        let chunks: Vec<EmbeddedChunk> =
            serde_json::from_str(&data).map_err(|e| RagError::Serialization(e))?;

        let mut vector_store = VectorStore::new(dimension);
        vector_store.add_chunks(chunks)?;

        info!("Loaded {} chunks from storage", vector_store.len());
        Ok(vector_store)
    }

    /// Save vector store to file
    pub fn save(&mut self) -> RagResult<()> {
        if !self.dirty || !self.config.enable_persistence {
            return Ok(());
        }

        debug!("Saving vector store to {:?}", self.storage_path);

        let chunks = self.vector_store.chunks();
        let data = serde_json::to_string_pretty(chunks).map_err(|e| RagError::Serialization(e))?;

        std::fs::write(&self.storage_path, data).map_err(|e| RagError::Io(e))?;

        self.dirty = false;
        info!("Saved {} chunks to storage", chunks.len());
        Ok(())
    }

    /// Add chunks to the store
    pub fn add_chunks(&mut self, chunks: Vec<EmbeddedChunk>) -> RagResult<()> {
        self.vector_store.add_chunks(chunks)?;
        self.dirty = true;
        Ok(())
    }

    /// Search for similar chunks
    pub fn search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        threshold: f32,
    ) -> Vec<(usize, f32)> {
        self.vector_store.search(query_embedding, top_k, threshold)
    }

    /// Get chunk by index
    pub fn get_chunk(&self, index: usize) -> Option<&EmbeddedChunk> {
        self.vector_store.get_chunk(index)
    }

    /// Get all chunks
    pub fn chunks(&self) -> &[EmbeddedChunk] {
        self.vector_store.chunks()
    }

    /// Get number of chunks
    pub fn len(&self) -> usize {
        self.vector_store.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.vector_store.is_empty()
    }

    /// Get repository ID
    pub fn repo_id(&self) -> &str {
        &self.repo_id
    }

    /// Check if store needs saving
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl Drop for PersistentVectorStore {
    fn drop(&mut self) {
        if self.dirty && self.config.enable_persistence {
            if let Err(e) = self.save() {
                error!("Failed to save vector store on drop: {}", e);
            }
        }
    }
}

/// Chat session manager with persistent storage
pub struct ChatSessionManager {
    /// Storage configuration
    config: crate::types::ChatConfig,
    /// Current active sessions
    sessions: HashMap<String, ChatSession>,
}

impl ChatSessionManager {
    /// Create a new chat session manager
    pub fn new(config: crate::types::ChatConfig) -> RagResult<Self> {
        // Create history directory
        std::fs::create_dir_all(&config.history_dir).map_err(|e| RagError::Io(e))?;

        Ok(Self {
            config,
            sessions: HashMap::new(),
        })
    }

    /// Create a new chat session
    pub fn create_session(&mut self, repository: String) -> String {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let session = ChatSession {
            id: session_id.clone(),
            repository,
            messages: Vec::new(),
            created_at: now,
            last_activity: now,
            metadata: HashMap::new(),
        };

        self.sessions.insert(session_id.clone(), session);
        info!("Created new chat session: {}", session_id);

        session_id
    }

    /// Add message to session
    pub fn add_message(&mut self, session_id: &str, role: &str, content: &str) -> RagResult<()> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| RagError::Config(format!("Session not found: {}", session_id)))?;

        let message = ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            metadata: None,
        };

        session.messages.push(message);
        session.last_activity = Utc::now();

        // Trim messages if exceeding limit
        if session.messages.len() > self.config.max_context_messages {
            let excess = session.messages.len() - self.config.max_context_messages;
            session.messages.drain(0..excess);
            debug!("Trimmed {} old messages from session", excess);
        }

        Ok(())
    }

    /// Get session messages for context
    pub fn get_context_messages(&self, session_id: &str) -> Vec<&ChatMessage> {
        if let Some(session) = self.sessions.get(session_id) {
            session.messages.iter().collect()
        } else {
            Vec::new()
        }
    }

    /// Save session to disk
    pub fn save_session(&self, session_id: &str) -> RagResult<()> {
        if !self.config.save_history {
            return Ok(());
        }

        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| RagError::Config(format!("Session not found: {}", session_id)))?;

        let file_path = self.config.history_dir.join(format!("{}.json", session_id));
        let data = serde_json::to_string_pretty(session).map_err(|e| RagError::Serialization(e))?;

        std::fs::write(file_path, data).map_err(|e| RagError::Io(e))?;

        debug!("Saved session {} to disk", session_id);
        Ok(())
    }

    /// Load session from disk
    pub fn load_session(&mut self, session_id: &str) -> RagResult<()> {
        let file_path = self.config.history_dir.join(format!("{}.json", session_id));

        if !file_path.exists() {
            return Err(RagError::Config(format!(
                "Session file not found: {}",
                session_id
            )));
        }

        let data = std::fs::read_to_string(file_path).map_err(|e| RagError::Io(e))?;

        let session: ChatSession =
            serde_json::from_str(&data).map_err(|e| RagError::Serialization(e))?;

        self.sessions.insert(session_id.to_string(), session);
        info!("Loaded session {} from disk", session_id);

        Ok(())
    }

    /// List available sessions
    pub fn list_sessions(&self) -> RagResult<Vec<String>> {
        let mut session_files = Vec::new();

        if self.config.history_dir.exists() {
            for entry in std::fs::read_dir(&self.config.history_dir).map_err(|e| RagError::Io(e))? {
                let entry = entry.map_err(|e| RagError::Io(e))?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        session_files.push(stem.to_string());
                    }
                }
            }
        }

        Ok(session_files)
    }

    /// Clean up old sessions
    pub fn cleanup_old_sessions(&mut self) -> RagResult<usize> {
        let cutoff =
            Utc::now() - chrono::Duration::minutes(self.config.session_timeout_minutes as i64);
        let mut removed_count = 0;

        self.sessions.retain(|session_id, session| {
            if session.last_activity < cutoff {
                info!("Removing expired session: {}", session_id);
                removed_count += 1;
                false
            } else {
                true
            }
        });

        Ok(removed_count)
    }
}
