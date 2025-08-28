//! Application state management
//!
//! This module manages the shared state across the web application.

use crate::{WebConfig, WebError, WebResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use wikify_rag::RagPipeline;
use wikify_wiki::WikiService;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Configuration
    pub config: WebConfig,
    /// RAG pipeline for question answering
    pub rag_pipeline: Arc<RwLock<Option<RagPipeline>>>,
    /// Wiki service for documentation generation
    pub wiki_service: Arc<RwLock<WikiService>>,
    /// Active repository sessions
    pub sessions: Arc<RwLock<HashMap<String, RepositorySession>>>,
    /// Cache for generated wikis
    pub wiki_cache: Arc<RwLock<HashMap<String, CachedWiki>>>,
}

/// Repository session information
#[derive(Debug, Clone)]
pub struct RepositorySession {
    /// Session ID
    pub id: String,
    /// Repository URL or path
    pub repository: String,
    /// Repository type (github, local, etc.)
    pub repo_type: String,
    /// When the session was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Whether the repository is indexed
    pub is_indexed: bool,
    /// Indexing progress (0.0 to 1.0)
    pub indexing_progress: f64,
}

/// Cached wiki information
#[derive(Debug, Clone)]
pub struct CachedWiki {
    /// Wiki structure
    pub wiki: wikify_wiki::WikiStructure,
    /// When it was generated
    pub generated_at: chrono::DateTime<chrono::Utc>,
    /// Repository information
    pub repository: String,
    /// Generation configuration used
    pub config: wikify_wiki::WikiConfig,
}

impl AppState {
    /// Create a new application state
    pub async fn new(config: WebConfig) -> WebResult<Self> {
        let wiki_service = WikiService::new()
            .map_err(|e| WebError::Config(format!("Failed to create wiki service: {}", e)))?;

        Ok(Self {
            config,
            rag_pipeline: Arc::new(RwLock::new(None)),
            wiki_service: Arc::new(RwLock::new(wiki_service)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            wiki_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Initialize RAG pipeline for a repository
    pub async fn initialize_rag(&self, repo_path: &str) -> WebResult<String> {
        let session_id = uuid::Uuid::new_v4().to_string();

        // Create repository session
        let session = RepositorySession {
            id: session_id.clone(),
            repository: repo_path.to_string(),
            repo_type: if repo_path.starts_with("http") {
                "remote"
            } else {
                "local"
            }
            .to_string(),
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            is_indexed: false,
            indexing_progress: 0.0,
        };

        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        // Initialize RAG pipeline
        let mut rag_config = wikify_rag::RagConfig::default();

        // Configure for web usage
        rag_config.retrieval.similarity_threshold = 0.4;
        rag_config.retrieval.top_k = 10;
        rag_config.retrieval.max_context_length = 16000;

        // Auto-detect LLM provider
        if std::env::var("OPENAI_API_KEY").is_ok() {
            rag_config.llm = wikify_rag::llm_client::configs::openai_gpt4o_mini();
            rag_config.embeddings.provider = "openai".to_string();
            rag_config.embeddings.model = "text-embedding-3-small".to_string();
        } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            rag_config.llm = wikify_rag::llm_client::configs::anthropic_claude_haiku();
        } else {
            rag_config.llm = wikify_rag::llm_client::configs::ollama_llama3(None);
        }

        let mut rag_pipeline = RagPipeline::new(rag_config);
        rag_pipeline
            .initialize()
            .await
            .map_err(|e| WebError::RagQuery(format!("Failed to initialize RAG: {}", e)))?;

        // Index the repository
        let indexing_pipeline = wikify_indexing::pipeline::IndexingPipeline::new(repo_path)
            .map_err(|e| {
                WebError::RagQuery(format!("Failed to create indexing pipeline: {}", e))
            })?;

        indexing_pipeline
            .run()
            .await
            .map_err(|e| WebError::RagQuery(format!("Failed to index repository: {}", e)))?;

        // Update session
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.is_indexed = true;
                session.indexing_progress = 1.0;
                session.last_activity = chrono::Utc::now();
            }
        }

        // Store RAG pipeline
        {
            let mut pipeline = self.rag_pipeline.write().await;
            *pipeline = Some(rag_pipeline);
        }

        Ok(session_id)
    }

    /// Get repository session
    pub async fn get_session(&self, session_id: &str) -> Option<RepositorySession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Update session activity
    pub async fn update_session_activity(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = chrono::Utc::now();
        }
    }

    /// Query RAG pipeline (placeholder implementation)
    pub async fn query_rag(&self, _question: &str) -> WebResult<wikify_rag::RagResponse> {
        // For now, return a placeholder response
        // TODO: Implement proper RAG querying with proper async handling
        Err(WebError::RagQuery(
            "RAG query not yet implemented for web interface".to_string(),
        ))
    }

    /// Generate wiki for repository
    pub async fn generate_wiki(
        &self,
        repo_path: &str,
        config: wikify_wiki::WikiConfig,
    ) -> WebResult<wikify_wiki::WikiStructure> {
        let mut wiki_service = self.wiki_service.write().await;

        let wiki = wiki_service
            .generate_wiki(repo_path, &config)
            .await
            .map_err(|e| WebError::WikiGeneration(format!("Failed to generate wiki: {}", e)))?;

        // Cache the generated wiki
        let cached_wiki = CachedWiki {
            wiki: wiki.clone(),
            generated_at: chrono::Utc::now(),
            repository: repo_path.to_string(),
            config,
        };

        {
            let mut cache = self.wiki_cache.write().await;
            cache.insert(repo_path.to_string(), cached_wiki);
        }

        Ok(wiki)
    }

    /// Get cached wiki
    pub async fn get_cached_wiki(&self, repo_path: &str) -> Option<CachedWiki> {
        let cache = self.wiki_cache.read().await;
        cache.get(repo_path).cloned()
    }

    /// Clean up old sessions
    pub async fn cleanup_old_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);

        sessions.retain(|_, session| session.last_activity > cutoff);
    }
}
