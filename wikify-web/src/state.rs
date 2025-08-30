//! Clean application state management using unified application layer

use crate::{
    auth::{
        api_keys::ApiKeyService, database::DatabaseUserStore, users::UserService, users::UserStore,
    },
    WebConfig, WebError, WebResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};
use wikify_applications::{ApplicationConfig, PermissionContext, UserIdentity, WikifyApplication};
use wikify_wiki::WikiService;

#[cfg(feature = "sqlite")]
use crate::simple_database::SimpleDatabaseService;

/// Web-specific indexing progress update
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
        wiki_content: String,
    },
    WikiGenerationError {
        session_id: String,
        error: String,
    },
}

/// Cached wiki content
#[derive(Debug, Clone)]
pub struct CachedWiki {
    pub content: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub repository: String,
    pub format: String,
}

/// Clean application state using unified application layer
#[derive(Clone)]
pub struct AppState {
    /// Configuration
    pub config: WebConfig,
    /// Main Wikify application service
    pub application: Arc<WikifyApplication>,
    /// Database service (optional)
    #[cfg(feature = "sqlite")]
    pub database: Option<Arc<SimpleDatabaseService>>,
    /// Wiki service for documentation generation
    pub wiki_service: Arc<RwLock<WikiService>>,
    /// Cache for generated wikis
    pub wiki_cache: Arc<RwLock<HashMap<String, CachedWiki>>>,
    /// Progress broadcaster for web-specific indexing updates
    pub progress_broadcaster: broadcast::Sender<IndexingUpdate>,
    /// User service for authentication and user management
    pub user_service: UserService,
    /// API Key service for API key management
    pub api_key_service: ApiKeyService,
}

impl AppState {
    /// Create a new application state
    pub async fn new(config: WebConfig) -> WebResult<Self> {
        // Create application configuration based on web config
        let app_config = match config.permission_mode.as_deref() {
            Some("open") => ApplicationConfig::web_open(),
            Some("restricted") => ApplicationConfig::web_restricted(),
            _ => ApplicationConfig::web_open(), // Default to open mode
        };

        // Create the main application service
        let application = WikifyApplication::new(app_config)
            .await
            .map_err(|e| WebError::Config(format!("Failed to create application: {}", e)))?;

        let wiki_service = WikiService::new()
            .map_err(|e| WebError::Config(format!("Failed to create wiki service: {}", e)))?;

        // Initialize database if configured
        #[cfg(feature = "sqlite")]
        let database = if let Some(database_url) = &config.database_url {
            match SimpleDatabaseService::new(database_url).await {
                Ok(db) => {
                    info!("Database initialized successfully");
                    Some(Arc::new(db))
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize database: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Create progress broadcaster with a buffer of 100 messages
        let (progress_broadcaster, _) = broadcast::channel::<IndexingUpdate>(100);

        // Create user service with appropriate storage backend
        let user_service = {
            #[cfg(feature = "sqlite")]
            {
                if let Some(ref db) = database {
                    // Use database storage for users
                    match DatabaseUserStore::new(db.pool().clone()).await {
                        Ok(db_store) => {
                            let user_store = UserStore::database(db_store).await;
                            UserService::new(user_store)
                        }
                        Err(e) => {
                            warn!(
                                "Failed to create database user store, falling back to memory: {}",
                                e
                            );
                            UserService::default()
                        }
                    }
                } else {
                    UserService::default()
                }
            }
            #[cfg(not(feature = "sqlite"))]
            {
                UserService::default()
            }
        };

        // Create API key service
        let api_key_service = ApiKeyService::memory();

        let state = Self {
            config,
            application: Arc::new(application),
            #[cfg(feature = "sqlite")]
            database,
            wiki_service: Arc::new(RwLock::new(wiki_service)),
            wiki_cache: Arc::new(RwLock::new(HashMap::new())),
            progress_broadcaster,
            user_service,
            api_key_service,
        };

        info!("Application state initialized successfully");
        Ok(state)
    }

    /// Create permission context from HTTP request headers
    pub fn create_permission_context(
        &self,
        headers: &HashMap<String, String>,
    ) -> PermissionContext {
        match self.config.permission_mode.as_deref() {
            Some("open") => PermissionContext::open(),
            Some("restricted") => {
                // Check for authentication headers
                if let Some(user_id) = headers.get("x-user-id") {
                    let display_name = headers.get("x-user-name").cloned();
                    let email = headers.get("x-user-email").cloned();

                    let identity = UserIdentity::registered(user_id.clone(), display_name, email);
                    PermissionContext::user(identity)
                } else {
                    // Anonymous user with limited permissions
                    PermissionContext::anonymous(
                        [wikify_applications::Permission::Query]
                            .into_iter()
                            .collect(),
                        wikify_applications::auth::permissions::ResourceLimits::anonymous(),
                    )
                }
            }
            Some("local") => PermissionContext::local(),
            _ => PermissionContext::open(), // Default to open
        }
    }

    /// Create permission context for anonymous users
    pub fn create_anonymous_context(&self) -> PermissionContext {
        match self.config.permission_mode.as_deref() {
            Some("restricted") => PermissionContext::anonymous(
                [wikify_applications::Permission::Query]
                    .into_iter()
                    .collect(),
                wikify_applications::auth::permissions::ResourceLimits::anonymous(),
            ),
            _ => PermissionContext::open(),
        }
    }

    /// Initialize RAG pipeline for a repository using application layer
    pub async fn initialize_rag(
        &self,
        repo_path: &str,
        auto_generate_wiki: bool,
    ) -> WebResult<String> {
        info!("Initializing RAG for repository: {}", repo_path);

        // Create permission context for this operation
        let context = self.create_anonymous_context();

        // Create session options
        let options = wikify_applications::SessionOptions {
            auto_generate_wiki,
            ..Default::default()
        };

        // Use the application layer to create session
        let session_id = self
            .application
            .create_session(&context, repo_path.to_string(), options)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to create session: {}", e)))?;

        info!(
            "Created session: {} for repository: {}",
            session_id, repo_path
        );

        // Subscribe to progress updates from the application layer
        let mut progress_receiver = self.application.subscribe_to_progress();
        let web_progress_broadcaster = self.progress_broadcaster.clone();

        // Forward application progress updates to web progress updates
        tokio::spawn(async move {
            while let Ok(app_update) = progress_receiver.recv().await {
                // Convert application IndexingUpdate to web IndexingUpdate
                let web_update = match app_update.is_complete {
                    true if app_update.error.is_some() => IndexingUpdate::Error {
                        session_id: app_update.session_id,
                        error: app_update
                            .error
                            .unwrap_or_else(|| "Unknown error".to_string()),
                    },
                    true => IndexingUpdate::Complete {
                        session_id: app_update.session_id,
                        total_files: 0,  // TODO: Extract from message
                        total_chunks: 0, // TODO: Extract from message
                        duration_ms: 0,  // TODO: Calculate duration
                    },
                    false => IndexingUpdate::Progress {
                        session_id: app_update.session_id,
                        stage: app_update.message,
                        percentage: app_update.progress,
                        current_item: None,
                        files_processed: None,
                        total_files: None,
                    },
                };

                let _ = web_progress_broadcaster.send(web_update);
            }
        });

        Ok(session_id)
    }

    /// Query RAG pipeline using application layer
    pub async fn query_rag(
        &self,
        session_id: &str,
        question: &str,
    ) -> WebResult<wikify_applications::QueryResponse> {
        info!("Processing RAG query for session: {}", session_id);
        debug!("Question: {}", question);

        // Create permission context for this query
        let context = self.create_anonymous_context();

        // Use the application layer to execute the query
        let response = self
            .application
            .query(&context, session_id, question.to_string())
            .await
            .map_err(|e| WebError::RagQuery(format!("Query failed: {}", e)))?;

        info!("RAG query completed successfully");
        Ok(response)
    }

    /// Get session information using application layer
    pub async fn get_session(&self, session_id: &str) -> Option<wikify_applications::SessionInfo> {
        let context = self.create_anonymous_context();
        self.application
            .get_session(&context, session_id)
            .await
            .ok()
    }

    /// Clean up stale sessions using application layer
    pub async fn cleanup_old_sessions(&self) {
        let cleaned_count = self.application.cleanup_stale_sessions().await;
        info!("Cleaned up {} stale sessions", cleaned_count);
    }

    /// Delete repository session using application layer
    pub async fn delete_repository(&self, session_id: &str) -> WebResult<()> {
        let context = self.create_anonymous_context();
        self.application
            .remove_session(&context, session_id)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to delete session: {}", e)))
    }

    /// Update session activity (no-op since application layer handles this)
    pub async fn update_session_activity(&self, _session_id: &str) -> WebResult<()> {
        // The application layer handles session activity updates automatically
        Ok(())
    }

    /// Generate wiki for session (placeholder implementation)
    pub async fn generate_wiki_for_session(
        &self,
        _session_id: &str,
        _repository: &str,
        _config: wikify_wiki::WikiConfig,
    ) -> WebResult<String> {
        // TODO: Implement wiki generation through application layer
        Ok("Wiki generation not yet implemented".to_string())
    }

    /// Get cached wiki (placeholder implementation)
    pub async fn get_cached_wiki(&self, _session_id: &str) -> Option<CachedWiki> {
        // TODO: Implement wiki caching
        None
    }
}
