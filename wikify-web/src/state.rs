//! Application state management
//!
//! This module manages the shared state across the web application.

use crate::{WebConfig, WebError, WebResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use wikify_rag::RagPipeline;
use wikify_wiki::WikiService;

#[cfg(feature = "sqlite")]
use crate::simple_database::SimpleDatabaseService;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Configuration
    pub config: WebConfig,
    /// Database service (optional)
    #[cfg(feature = "sqlite")]
    pub database: Option<Arc<SimpleDatabaseService>>,
    /// Wiki service for documentation generation
    pub wiki_service: Arc<RwLock<WikiService>>,
    /// Active repository sessions (each with its own RAG pipeline)
    pub sessions: Arc<RwLock<HashMap<String, RepositorySession>>>,
    /// Cache for generated wikis
    pub wiki_cache: Arc<RwLock<HashMap<String, CachedWiki>>>,
}

/// Repository session information
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
    /// RAG pipeline for this session
    pub rag_pipeline: Option<RagPipeline>,
}

impl Clone for RepositorySession {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            repository: self.repository.clone(),
            repo_type: self.repo_type.clone(),
            created_at: self.created_at,
            last_activity: self.last_activity,
            is_indexed: self.is_indexed,
            indexing_progress: self.indexing_progress,
            rag_pipeline: None, // RAG pipeline cannot be cloned, will be recreated if needed
        }
    }
}

impl std::fmt::Debug for RepositorySession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepositorySession")
            .field("id", &self.id)
            .field("repository", &self.repository)
            .field("repo_type", &self.repo_type)
            .field("created_at", &self.created_at)
            .field("last_activity", &self.last_activity)
            .field("is_indexed", &self.is_indexed)
            .field("indexing_progress", &self.indexing_progress)
            .field("rag_pipeline", &self.rag_pipeline.is_some())
            .finish()
    }
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

        // Initialize database service if SQLite feature is enabled and database URL is provided
        #[cfg(feature = "sqlite")]
        let database = if let Some(database_url) = config.database_url.as_ref() {
            tracing::info!("🗄️  Initializing database: {}", database_url);

            // Check if it's a file database and log directory info
            if database_url.starts_with("sqlite:") && !database_url.contains(":memory:") {
                let db_path = database_url.strip_prefix("sqlite:").unwrap_or(database_url);
                let absolute_path = std::path::Path::new(db_path)
                    .canonicalize()
                    .unwrap_or_else(|_| std::env::current_dir().unwrap().join(db_path));
                tracing::info!(
                    "📁 Database file path: {} (absolute: {})",
                    db_path,
                    absolute_path.display()
                );

                // Check if directory exists
                if let Some(parent_dir) = std::path::Path::new(db_path).parent() {
                    if parent_dir.exists() {
                        tracing::info!("✅ Database directory exists: {}", parent_dir.display());

                        // Check directory permissions
                        match std::fs::metadata(parent_dir) {
                            Ok(metadata) => {
                                tracing::info!(
                                    "📋 Directory permissions: readonly={}",
                                    metadata.permissions().readonly()
                                );
                            }
                            Err(e) => {
                                tracing::warn!("⚠️  Could not read directory metadata: {}", e);
                            }
                        }
                    } else {
                        tracing::warn!(
                            "⚠️  Database directory does not exist: {}",
                            parent_dir.display()
                        );
                        tracing::info!("🏗️  Attempting to create directory...");
                        if let Err(e) = std::fs::create_dir_all(parent_dir) {
                            tracing::error!("❌ Failed to create directory: {}", e);
                        } else {
                            tracing::info!("✅ Directory created successfully");
                        }
                    }
                }

                // Check if database file already exists
                if std::path::Path::new(db_path).exists() {
                    tracing::info!("📄 Database file already exists");
                    match std::fs::metadata(db_path) {
                        Ok(metadata) => {
                            tracing::info!(
                                "📋 File size: {} bytes, readonly: {}",
                                metadata.len(),
                                metadata.permissions().readonly()
                            );
                        }
                        Err(e) => {
                            tracing::warn!("⚠️  Could not read file metadata: {}", e);
                        }
                    }
                } else {
                    tracing::info!("📄 Database file does not exist, will be created");
                }
            }

            match SimpleDatabaseService::new(database_url).await {
                Ok(db_service) => {
                    tracing::info!("✅ Database initialized successfully");
                    Some(Arc::new(db_service))
                }
                Err(e) => {
                    tracing::warn!(
                        "❌ Failed to initialize database: {}, trying memory database as fallback",
                        e
                    );

                    // 尝试使用内存数据库作为后备
                    match SimpleDatabaseService::new("sqlite::memory:").await {
                        Ok(db_service) => {
                            tracing::info!("✅ Fallback memory database initialized successfully");
                            Some(Arc::new(db_service))
                        }
                        Err(fallback_e) => {
                            tracing::error!(
                                "❌ Even memory database failed: {}, continuing without database",
                                fallback_e
                            );
                            None
                        }
                    }
                }
            }
        } else {
            tracing::info!("🗄️  Database disabled, using memory-only storage");
            None
        };

        let state = Self {
            config,
            #[cfg(feature = "sqlite")]
            database,
            wiki_service: Arc::new(RwLock::new(wiki_service)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            wiki_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load existing sessions from database if available
        #[cfg(feature = "sqlite")]
        if let Some(database) = &state.database {
            if let Err(e) = state.load_sessions_from_database(database).await {
                tracing::warn!("Failed to load sessions from database: {}", e);
            }
        }

        Ok(state)
    }

    /// Initialize RAG pipeline for a repository
    pub async fn initialize_rag(&self, repo_path: &str) -> WebResult<String> {
        use tracing::{debug, info, warn};

        // Check if repository already exists
        {
            let sessions = self.sessions.read().await;
            for (existing_session_id, existing_session) in sessions.iter() {
                if existing_session.repository == repo_path {
                    if existing_session.is_indexed {
                        info!(
                            "[{}] Repository already indexed: {} - returning existing session",
                            existing_session_id, repo_path
                        );
                        return Ok(existing_session_id.clone());
                    } else {
                        warn!("[{}] Repository currently being indexed: {} - returning existing session",
                              existing_session_id, repo_path);
                        return Ok(existing_session_id.clone());
                    }
                }
            }
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        info!(
            "[{}] Initializing RAG for new repository: {}",
            session_id, repo_path
        );

        // Create repository session (without RAG pipeline initially)
        let mut session = RepositorySession {
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
            rag_pipeline: None,
        };

        // Initialize RAG pipeline configuration
        let mut rag_config = wikify_rag::RagConfig::default();

        // Configure for web usage with optimized thresholds
        rag_config.retrieval.similarity_threshold = 0.2; // Lower threshold for better recall
        rag_config.retrieval.top_k = 8;
        rag_config.retrieval.max_context_length = 12000;

        // Auto-detect LLM provider
        if std::env::var("OPENAI_API_KEY").is_ok() {
            rag_config.llm = wikify_rag::llm_client::configs::openai_gpt4o_mini();
            rag_config.embeddings.provider = "openai".to_string();
            rag_config.embeddings.model = "text-embedding-3-small".to_string();
            debug!("Using OpenAI for LLM and embeddings");
        } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            rag_config.llm = wikify_rag::llm_client::configs::anthropic_claude_haiku();
            debug!("Using Anthropic for LLM");
        } else {
            rag_config.llm = wikify_rag::llm_client::configs::ollama_llama3(None);
            debug!("Using Ollama for LLM");
        }

        // Create and initialize RAG pipeline
        let mut rag_pipeline = RagPipeline::new(rag_config);
        rag_pipeline
            .initialize()
            .await
            .map_err(|e| WebError::RagQuery(format!("Failed to initialize RAG: {}", e)))?;

        info!(
            "[{}] RAG pipeline initialized, starting repository indexing for: {}",
            session_id, repo_path
        );

        // Index the repository using the RAG pipeline with progress reporting
        let session_id_clone = session_id.clone();
        let indexing_stats = rag_pipeline
            .index_repository_with_progress(
                repo_path,
                Some(Box::new(move |stage, percentage, current_item| {
                    info!(
                        "[{}] {}: {:.1}% - {}",
                        session_id_clone,
                        stage,
                        percentage,
                        current_item.unwrap_or_else(|| "Processing...".to_string())
                    );
                })),
            )
            .await
            .map_err(|e| WebError::RagQuery(format!("Failed to index repository: {}", e)))?;

        info!("Repository indexed: {}", indexing_stats.summary());

        // Update session with RAG pipeline and indexing status
        session.is_indexed = true;
        session.indexing_progress = 1.0;
        session.last_activity = chrono::Utc::now();
        session.rag_pipeline = Some(rag_pipeline);

        // Store session in memory
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session.clone());
        }

        // Save session and repository to database if available
        #[cfg(feature = "sqlite")]
        if let Some(database) = &self.database {
            // Save repository information
            if let Err(e) = self.save_repository_to_database(database, &session).await {
                tracing::warn!("Failed to save repository to database: {}", e);
            }

            // Save session information
            if let Err(e) = self.save_session_to_database(database, &session).await {
                tracing::warn!("Failed to save session to database: {}", e);
            }
        }

        info!("Session {} initialized successfully", session_id);
        Ok(session_id)
    }

    /// Get repository session
    pub async fn get_session(&self, session_id: &str) -> Option<RepositorySession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Update session activity
    pub async fn update_session_activity(&self, session_id: &str) -> WebResult<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = chrono::Utc::now();

            // Update in database if available
            #[cfg(feature = "sqlite")]
            if let Some(database) = &self.database {
                if let Err(e) = self.save_session_to_database(database, session).await {
                    tracing::warn!("Failed to update session in database: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Save session to database
    #[cfg(feature = "sqlite")]
    async fn save_session_to_database(
        &self,
        database: &std::sync::Arc<crate::simple_database::SimpleDatabaseService>,
        session: &RepositorySession,
    ) -> WebResult<()> {
        use crate::simple_database::SimpleSession;

        let simple_session = SimpleSession {
            id: session.id.clone(),
            repository_id: session.repository.clone(),
            created_at: session.created_at,
            last_activity: session.last_activity,
            is_active: true,
        };

        database.save_session(&simple_session).await?;
        tracing::debug!("Session {} saved to database", session.id);
        Ok(())
    }

    /// Load sessions from database
    #[cfg(feature = "sqlite")]
    async fn load_sessions_from_database(
        &self,
        database: &std::sync::Arc<crate::simple_database::SimpleDatabaseService>,
    ) -> WebResult<()> {
        let simple_sessions = database.get_sessions().await?;
        tracing::info!("Loading {} sessions from database", simple_sessions.len());

        for simple_session in simple_sessions {
            // Create a basic RepositorySession from database data
            // Note: RAG pipeline will need to be re-initialized
            let session = RepositorySession {
                id: simple_session.id.clone(),
                repository: simple_session.repository_id.clone(),
                repo_type: if simple_session.repository_id.starts_with("http") {
                    "remote"
                } else {
                    "local"
                }
                .to_string(),
                created_at: simple_session.created_at,
                last_activity: simple_session.last_activity,
                is_indexed: false, // Will need to be re-indexed
                indexing_progress: 0.0,
                rag_pipeline: None, // Will need to be re-initialized
            };

            // Store in memory
            let mut sessions = self.sessions.write().await;
            sessions.insert(simple_session.id.clone(), session);
            tracing::debug!("Loaded session {} from database", simple_session.id);
        }

        tracing::info!("Successfully loaded sessions from database");
        Ok(())
    }

    /// Save repository to database
    #[cfg(feature = "sqlite")]
    async fn save_repository_to_database(
        &self,
        database: &std::sync::Arc<crate::simple_database::SimpleDatabaseService>,
        session: &RepositorySession,
    ) -> WebResult<()> {
        use crate::simple_database::SimpleRepository;

        let simple_repository = SimpleRepository {
            id: session.repository.clone(),
            name: session
                .repository
                .split('/')
                .last()
                .unwrap_or(&session.repository)
                .to_string(),
            repo_path: session.repository.clone(),
            repo_type: session.repo_type.clone(),
            status: if session.is_indexed {
                "indexed"
            } else {
                "created"
            }
            .to_string(),
            created_at: session.created_at,
            last_indexed_at: if session.is_indexed {
                Some(session.last_activity)
            } else {
                None
            },
        };

        database.save_repository(&simple_repository).await?;
        tracing::debug!("Repository {} saved to database", session.repository);
        Ok(())
    }

    /// Query RAG pipeline for a specific session
    pub async fn query_rag(
        &self,
        session_id: &str,
        question: &str,
    ) -> WebResult<wikify_rag::RagResponse> {
        use tracing::{debug, info};
        use wikify_rag::types::RagQuery;

        info!("Processing RAG query for session: {}", session_id);
        debug!("Question: {}", question);

        // Get session and its RAG pipeline
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| WebError::RagQuery(format!("Session not found: {}", session_id)))?;

        // Check if session is indexed
        if !session.is_indexed {
            return Err(WebError::RagQuery(
                "Repository not indexed yet. Please wait for indexing to complete.".to_string(),
            ));
        }

        // Get RAG pipeline from session
        let rag_pipeline = session.rag_pipeline.as_mut().ok_or_else(|| {
            WebError::RagQuery("RAG pipeline not initialized for this session".to_string())
        })?;

        // Update session activity
        session.last_activity = chrono::Utc::now();

        // Create query
        let query = RagQuery {
            question: question.to_string(),
            context: None,
            filters: None,
            retrieval_config: None,
        };

        // Execute RAG query
        let response = rag_pipeline
            .ask(query)
            .await
            .map_err(|e| WebError::RagQuery(format!("RAG query failed: {}", e)))?;

        info!(
            "RAG query completed: {} chunks retrieved, {} tokens generated",
            response.metadata.chunks_retrieved, response.metadata.generation_tokens
        );

        Ok(response)
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

    // 暂时禁用数据库相关方法，将在后续版本中实现
    // TODO: 实现数据库集成功能
}
