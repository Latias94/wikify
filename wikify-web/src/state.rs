//! Application state management
//!
//! This module manages the shared state across the web application.

use crate::{WebConfig, WebError, WebResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};
use wikify_rag::{IndexingManager, RagPipeline};
use wikify_wiki::WikiService;

#[cfg(feature = "sqlite")]
use crate::simple_database::SimpleDatabaseService;

/// Progress update message for indexing operations
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum IndexingUpdate {
    Progress {
        session_id: String,
        stage: String,
        percentage: f64,
        current_item: Option<String>,
        files_processed: Option<usize>,
        total_files: Option<usize>,
    },
    Complete {
        session_id: String,
        total_files: usize,
        total_chunks: usize,
        duration_ms: u64,
    },
    Error {
        session_id: String,
        error: String,
    },
    WikiGenerationStarted {
        session_id: String,
    },
    WikiGenerationProgress {
        session_id: String,
        stage: String,
        percentage: f64,
    },
    WikiGenerationComplete {
        session_id: String,
        wiki_id: String,
        pages_count: usize,
        sections_count: usize,
    },
    WikiGenerationError {
        session_id: String,
        error: String,
    },
}

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
    /// Progress broadcaster for indexing operations
    pub progress_broadcaster: broadcast::Sender<IndexingUpdate>,
    /// Indexing manager for concurrency control
    pub indexing_manager: Arc<IndexingManager>,
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
    /// Whether to automatically generate wiki after indexing
    pub auto_generate_wiki: bool,
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
            auto_generate_wiki: self.auto_generate_wiki,
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
            .field("auto_generate_wiki", &self.auto_generate_wiki)
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

        // Create progress broadcaster with a buffer of 100 messages
        let (progress_broadcaster, _) = broadcast::channel::<IndexingUpdate>(100);

        // Create indexing manager with default concurrency limit (2)
        let indexing_manager = Arc::new(IndexingManager::new(2));

        let state = Self {
            config,
            #[cfg(feature = "sqlite")]
            database,
            wiki_service: Arc::new(RwLock::new(wiki_service)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            wiki_cache: Arc::new(RwLock::new(HashMap::new())),
            progress_broadcaster,
            indexing_manager,
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
    pub async fn initialize_rag(
        &self,
        repo_path: &str,
        auto_generate_wiki: bool,
    ) -> WebResult<String> {
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
            auto_generate_wiki,
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

        // Store session in memory first (before indexing starts)
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session.clone());
        }

        // Start indexing using IndexingManager with repository path checking
        let sessions = self.sessions.clone();
        let progress_broadcaster = self.progress_broadcaster.clone();
        let wiki_service = self.wiki_service.clone();
        let wiki_cache = self.wiki_cache.clone();
        let repo_path_owned = repo_path.to_string();
        let session_id_for_task = session_id.clone();

        let indexing_result = self
            .indexing_manager
            .start_indexing_with_repo_check(
                session_id.clone(),
                repo_path_owned.clone(),
                move || {
                    let session_id_clone = session_id_for_task.clone();
                    let progress_broadcaster = progress_broadcaster.clone();
                    let repo_path = repo_path_owned.clone();
                    let sessions = sessions.clone();
                    let wiki_service = wiki_service.clone();
                    let wiki_cache = wiki_cache.clone();

                    async move {
                        // Perform the indexing operation
                        Self::perform_indexing_task(
                            session_id_clone,
                            repo_path,
                            sessions,
                            progress_broadcaster,
                            wiki_service,
                            wiki_cache,
                        )
                        .await;
                    }
                },
            )
            .await;

        match indexing_result {
            Ok(()) => {
                info!("Indexing started successfully for session: {}", session_id);

                // Save session to database if available (indexing will update it later)
                #[cfg(feature = "sqlite")]
                if let Some(database) = &self.database {
                    if let Err(e) = self.save_repository_to_database(database, &session).await {
                        warn!("Failed to save repository to database: {}", e);
                    }
                    if let Err(e) = self.save_session_to_database(database, &session).await {
                        warn!("Failed to save session to database: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to start indexing: {}", e);
                return Err(WebError::RagQuery(format!(
                    "Failed to start indexing: {}",
                    e
                )));
            }
        }

        info!("Session {} initialized successfully", session_id);
        Ok(session_id)
    }

    /// Perform indexing task in background (used by IndexingManager)
    pub async fn perform_indexing_task(
        session_id: String,
        repo_path: String,
        sessions: Arc<RwLock<HashMap<String, RepositorySession>>>,
        progress_broadcaster: broadcast::Sender<IndexingUpdate>,
        wiki_service: Arc<RwLock<WikiService>>,
        wiki_cache: Arc<RwLock<HashMap<String, CachedWiki>>>,
    ) {
        info!("Starting indexing task for session: {}", session_id);

        // Create RAG configuration
        let mut rag_config = wikify_rag::RagConfig::default();
        rag_config.retrieval.similarity_threshold = 0.2;
        rag_config.retrieval.top_k = 8;
        rag_config.retrieval.max_context_length = 12000;

        // Auto-detect LLM provider
        if std::env::var("OPENAI_API_KEY").is_ok() {
            rag_config.llm = wikify_rag::llm_client::configs::openai_gpt4o_mini();
            rag_config.embeddings.provider = "openai".to_string();
            rag_config.embeddings.model = "text-embedding-3-small".to_string();
        } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            rag_config.llm = wikify_rag::llm_client::configs::anthropic_claude_haiku();
        } else {
            rag_config.llm = wikify_rag::llm_client::configs::ollama_llama3(None);
            rag_config.embeddings.provider = "ollama".to_string();
            rag_config.embeddings.model = "nomic-embed-text".to_string();
        }

        // Create and initialize RAG pipeline
        let mut rag_pipeline = RagPipeline::new(rag_config);
        if let Err(e) = rag_pipeline.initialize().await {
            error!(
                "Failed to initialize RAG pipeline for session {}: {}",
                session_id, e
            );
            let error_msg = IndexingUpdate::Error {
                session_id: session_id.clone(),
                error: format!("Failed to initialize RAG pipeline: {}", e),
            };
            let _ = progress_broadcaster.send(error_msg);
            return;
        }

        // Index the repository with progress reporting
        let session_id_clone = session_id.clone();
        let progress_broadcaster_clone = progress_broadcaster.clone();
        let indexing_result = rag_pipeline
            .index_repository_with_progress(
                &repo_path,
                Some(Box::new(move |stage, percentage, current_item| {
                    let current_item_display = current_item.as_deref().unwrap_or("Processing...");

                    info!(
                        "[{}] {}: {:.1}% - {}",
                        session_id_clone, stage, percentage, current_item_display
                    );

                    // Send progress update via WebSocket
                    let progress = IndexingUpdate::Progress {
                        session_id: session_id_clone.clone(),
                        stage: stage.clone(),
                        percentage,
                        current_item,
                        files_processed: None,
                        total_files: None,
                    };

                    let _ = progress_broadcaster_clone.send(progress);
                })),
            )
            .await;

        // Update session based on indexing result
        {
            let mut sessions_guard = sessions.write().await;
            if let Some(session) = sessions_guard.get_mut(&session_id) {
                match indexing_result {
                    Ok(stats) => {
                        info!("Repository indexed successfully: {}", stats.summary());

                        // Send completion message
                        let completion = IndexingUpdate::Complete {
                            session_id: session_id.clone(),
                            total_files: stats.total_documents,
                            total_chunks: stats.total_nodes,
                            duration_ms: 0,
                        };
                        let _ = progress_broadcaster.send(completion);

                        // Update session with successful indexing
                        session.is_indexed = true;
                        session.indexing_progress = 1.0;
                        session.last_activity = chrono::Utc::now();
                        session.rag_pipeline = Some(rag_pipeline);

                        // Check if auto wiki generation is enabled
                        let should_generate_wiki = session.auto_generate_wiki;
                        let repo_path_for_wiki = session.repository.clone();
                        let session_id_for_wiki = session.id.clone();

                        // Drop the sessions lock before starting wiki generation
                        drop(sessions_guard);

                        // Start wiki generation if enabled
                        if should_generate_wiki {
                            info!(
                                "Starting automatic wiki generation for session: {}",
                                session_id_for_wiki
                            );

                            // Send wiki generation start notification
                            let wiki_start = IndexingUpdate::WikiGenerationStarted {
                                session_id: session_id_for_wiki.clone(),
                            };
                            let _ = progress_broadcaster.send(wiki_start);

                            // Spawn wiki generation task
                            let wiki_service_clone = wiki_service.clone();
                            let wiki_cache_clone = wiki_cache.clone();
                            let progress_broadcaster_clone = progress_broadcaster.clone();

                            tokio::spawn(async move {
                                Self::perform_wiki_generation_task(
                                    session_id_for_wiki,
                                    repo_path_for_wiki,
                                    wiki_service_clone,
                                    wiki_cache_clone,
                                    progress_broadcaster_clone,
                                )
                                .await;
                            });
                        } // Early return to avoid re-acquiring the lock
                    }
                    Err(e) => {
                        error!(
                            "Failed to index repository for session {}: {}",
                            session_id, e
                        );
                        let error_msg = IndexingUpdate::Error {
                            session_id: session_id.clone(),
                            error: format!("Failed to index repository: {}", e),
                        };
                        let _ = progress_broadcaster.send(error_msg);

                        // Reset session state on error
                        session.is_indexed = false;
                        session.indexing_progress = 0.0;
                        session.rag_pipeline = None;
                    }
                }
            } else {
                warn!(
                    "Session {} not found when updating after indexing",
                    session_id
                );
            }
        }
    }

    /// Perform wiki generation task in background
    pub async fn perform_wiki_generation_task(
        session_id: String,
        repo_path: String,
        wiki_service: Arc<RwLock<WikiService>>,
        wiki_cache: Arc<RwLock<HashMap<String, CachedWiki>>>,
        progress_broadcaster: broadcast::Sender<IndexingUpdate>,
    ) {
        info!("Starting wiki generation task for session: {}", session_id);

        // Send progress updates
        let stages = [
            "Analyzing repository structure",
            "Generating wiki structure",
            "Creating documentation pages",
            "Processing content",
            "Finalizing wiki",
        ];

        for (i, stage) in stages.iter().enumerate() {
            let progress = (i as f64) / (stages.len() as f64);
            let progress_update = IndexingUpdate::WikiGenerationProgress {
                session_id: session_id.clone(),
                stage: stage.to_string(),
                percentage: progress * 100.0,
            };
            let _ = progress_broadcaster.send(progress_update);

            // Simulate work for each stage
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        // Create wiki configuration
        let wiki_config = wikify_wiki::WikiConfig::default();

        // Generate wiki
        let wiki_result = {
            let mut wiki_service_guard = wiki_service.write().await;
            wiki_service_guard
                .generate_wiki(&repo_path, &wiki_config)
                .await
        };

        match wiki_result {
            Ok(wiki) => {
                info!("Wiki generated successfully for session: {}", session_id);

                // Cache the generated wiki
                let cached_wiki = CachedWiki {
                    wiki: wiki.clone(),
                    generated_at: chrono::Utc::now(),
                    repository: repo_path,
                    config: wiki_config,
                };

                {
                    let mut cache = wiki_cache.write().await;
                    cache.insert(session_id.clone(), cached_wiki);
                }

                // Send completion notification
                let completion = IndexingUpdate::WikiGenerationComplete {
                    session_id: session_id.clone(),
                    wiki_id: wiki.id.clone(),
                    pages_count: wiki.pages.len(),
                    sections_count: wiki.sections.len(),
                };
                let _ = progress_broadcaster.send(completion);
            }
            Err(e) => {
                error!("Failed to generate wiki for session {}: {}", session_id, e);
                let error_msg = IndexingUpdate::WikiGenerationError {
                    session_id: session_id.clone(),
                    error: format!("Failed to generate wiki: {}", e),
                };
                let _ = progress_broadcaster.send(error_msg);
            }
        }
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
                auto_generate_wiki: true, // Default to true for loaded sessions
                rag_pipeline: None,       // Will need to be re-initialized
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
            id: session.id.clone(), // 使用 session_id 作为 repository ID
            name: session
                .repository
                .split('/')
                .next_back()
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
        use tracing::{debug, error, info};
        use wikify_rag::types::RagQuery;

        info!("Processing RAG query for session: {}", session_id);
        debug!("Question: {}", question);

        // Get session and its RAG pipeline
        let mut sessions = self.sessions.write().await;

        // Debug: List all available sessions
        let available_sessions: Vec<String> = sessions.keys().cloned().collect();
        debug!("Available sessions: {:?}", available_sessions);

        let session = sessions.get_mut(session_id).ok_or_else(|| {
            error!(
                "Session not found: {}. Available sessions: {:?}",
                session_id, available_sessions
            );
            WebError::RagQuery(format!("Session not found: {}", session_id))
        })?;

        // Check if session is indexed
        if !session.is_indexed {
            error!(
                "Session {} is not indexed yet. is_indexed: {}, indexing_progress: {}",
                session_id, session.is_indexed, session.indexing_progress
            );
            return Err(WebError::RagQuery(
                "Repository not indexed yet. Please wait for indexing to complete.".to_string(),
            ));
        }

        // Get RAG pipeline from session
        let rag_pipeline = session.rag_pipeline.as_mut().ok_or_else(|| {
            error!("RAG pipeline not initialized for session {}. Session details: is_indexed: {}, indexing_progress: {}, repository: {}",
                   session_id, session.is_indexed, session.indexing_progress, session.repository);
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

    /// Delete repository and all associated data
    pub async fn delete_repository(&self, session_id: &str) -> WebResult<()> {
        use tracing::{info, warn};

        info!(
            "Starting deletion of repository with session: {}",
            session_id
        );

        // Step 1: Get session information before deletion
        let session_info = {
            let sessions = self.sessions.read().await;
            sessions.get(session_id).cloned()
        };

        let session = session_info.ok_or_else(|| {
            WebError::NotFound(format!("Repository session not found: {}", session_id))
        })?;

        info!("Found repository to delete: {}", session.repository);

        // Step 2: Clear vector storage data if RAG pipeline exists
        if let Some(rag_pipeline) = &session.rag_pipeline {
            info!(
                "Clearing vector storage for repository: {}",
                session.repository
            );
            if let Err(e) = self.clear_vector_storage(rag_pipeline).await {
                warn!("Failed to clear vector storage: {}", e);
                // Continue with deletion even if vector storage cleanup fails
            }
        }

        // Step 3: Remove from memory sessions
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(session_id);
            info!("Removed session {} from memory", session_id);
        }

        // Step 4: Remove from wiki cache
        {
            let mut cache = self.wiki_cache.write().await;
            cache.remove(&session.repository);
            info!("Removed wiki cache for repository: {}", session.repository);
        }

        // Step 5: Delete from database if available
        #[cfg(feature = "sqlite")]
        if let Some(database) = &self.database {
            if let Err(e) = self.delete_from_database(database, &session).await {
                warn!("Failed to delete from database: {}", e);
                // Continue with deletion even if database cleanup fails
            }
        }

        info!("Successfully deleted repository: {}", session.repository);
        Ok(())
    }

    /// Clear vector storage data for a RAG pipeline
    async fn clear_vector_storage(&self, _rag_pipeline: &wikify_rag::RagPipeline) -> WebResult<()> {
        // Note: This is a placeholder implementation
        // The actual implementation would depend on the RAG pipeline's vector store interface
        // For now, we'll log the operation
        tracing::info!("Vector storage clearing requested - implementation pending");

        // TODO: Implement actual vector storage clearing
        // This would involve:
        // 1. Getting the vector store from the RAG pipeline
        // 2. Calling the clear() method on the vector store
        // 3. Handling any errors appropriately

        Ok(())
    }

    /// Delete repository and session data from database
    #[cfg(feature = "sqlite")]
    async fn delete_from_database(
        &self,
        database: &std::sync::Arc<crate::simple_database::SimpleDatabaseService>,
        session: &RepositorySession,
    ) -> WebResult<()> {
        use tracing::info;

        // Delete session from database
        if let Err(e) = database.delete_session(&session.id).await {
            tracing::warn!("Failed to delete session from database: {}", e);
        } else {
            info!("Deleted session {} from database", session.id);
        }

        // Delete repository from database (using session_id as repository ID)
        if let Err(e) = database.delete_repository(&session.id).await {
            tracing::warn!("Failed to delete repository from database: {}", e);
        } else {
            info!("Deleted repository {} from database", session.id);
        }

        // Delete query history for this repository (using session_id)
        if let Err(e) = database.delete_query_history(&session.id).await {
            tracing::warn!("Failed to delete query history from database: {}", e);
        } else {
            info!(
                "Deleted query history for repository {} from database",
                session.id
            );
        }

        Ok(())
    }

    /// Clean up old sessions
    pub async fn cleanup_old_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);

        sessions.retain(|_, session| session.last_activity > cutoff);
    }

    /// Get a receiver for indexing progress updates (for testing and WebSocket connections)
    pub fn subscribe_to_progress(&self) -> broadcast::Receiver<IndexingUpdate> {
        self.progress_broadcaster.subscribe()
    }
}
