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
        repository_id: String,
        stage: String,
        percentage: f64,
        current_item: Option<String>,
        files_processed: Option<usize>,
        total_files: Option<usize>,
    },
    Complete {
        repository_id: String,
        total_files: usize,
        total_chunks: usize,
        duration_ms: u64,
    },
    Error {
        repository_id: String,
        error: String,
    },
    WikiGenerationStarted {
        repository_id: String,
    },
    WikiGenerationProgress {
        repository_id: String,
        stage: String,
        percentage: f64,
    },
    WikiGenerationComplete {
        repository_id: String,
        wiki_content: String,
    },
    WikiGenerationError {
        repository_id: String,
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

    /// Delete repository using application layer
    pub async fn delete_repository(&self, repository_id: &str) -> WebResult<()> {
        let context = self.create_anonymous_context();
        self.application
            .delete_repository(&context, repository_id)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to delete repository: {}", e)))
    }

    /// Clean up stale data using application layer
    pub async fn cleanup_old_data(&self) {
        // Note: This is now a no-op since we removed session management
        // Repository cleanup is handled by the application layer
    }

    /// Query RAG pipeline using application layer
    pub async fn query_rag(
        &self,
        repository_id: &str,
        question: &str,
    ) -> WebResult<wikify_applications::RepositoryQueryResponse> {
        info!("Processing RAG query for repository: {}", repository_id);
        debug!("Question: {}", question);

        // Create permission context for this query
        let context = self.create_anonymous_context();

        // Create repository query
        let query = wikify_applications::RepositoryQuery {
            question: question.to_string(),
            max_results: None,
            parameters: None,
        };

        // Use the application layer to execute the query
        let response = self
            .application
            .query_repository(&context, repository_id, query)
            .await
            .map_err(|e| WebError::RagQuery(format!("Query failed: {}", e)))?;

        info!("RAG query completed successfully");
        Ok(response)
    }

    /// Get repository information using application layer
    pub async fn get_repository(
        &self,
        repository_id: &str,
    ) -> Option<wikify_applications::RepositoryIndex> {
        let context = self.create_anonymous_context();
        self.application
            .get_repository(&context, repository_id)
            .await
            .ok()
    }

    /// Subscribe to progress updates
    pub async fn subscribe_to_progress(&self) -> tokio::sync::broadcast::Receiver<IndexingUpdate> {
        self.progress_broadcaster.subscribe()
    }
}
