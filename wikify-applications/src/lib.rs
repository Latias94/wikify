//! Wikify Applications - High-level applications built on RAG foundation
//!
//! This module provides application-layer functionality that builds upon
//! the basic RAG capabilities provided by wikify-rag. It includes:
//!
//! - Interactive chat systems with session management
//! - Deep research engines with multi-turn investigation
//! - Workshop and tutorial generation
//! - Advanced code analysis applications
//!
//! ## Architecture
//!
//! This module follows a clear separation between:
//! - **Basic RAG** (wikify-rag): Core retrieval and generation
//! - **Applications** (this module): High-level user-facing functionality
//! - **Presentation** (wikify-web/cli): User interfaces

pub mod auth;
pub mod research;
pub mod session;

// Future modules to be implemented
// pub mod wiki;
// pub mod query;

// Workshop module - to be implemented
// #[cfg(feature = "workshop")]
// pub mod workshop;

// Re-export main application types
pub use auth::{
    Permission, PermissionContext, PermissionManager, PermissionMode, UserIdentity, UserType,
};
pub use research::{
    FileResearchHistoryStorage, QuestionType, ResearchCategory, ResearchConfig, ResearchEngine,
    ResearchHistoryStorage, ResearchProgress, ResearchQuestion, ResearchResult, ResearchTemplate,
    ResearchTemplateManager,
};
pub use session::{
    ApplicationSession, IndexingUpdate, RepositoryInfo, RepositoryVisibility, SessionConfig,
    SessionInfo, SessionManager, SessionOptions, SessionStorage,
};

// Main application exports will be available directly from this module

/// Application-level error type
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("RAG error: {0}")]
    Rag(#[from] wikify_rag::RagError),

    #[error("Core error: {0}")]
    Core(#[from] wikify_core::WikifyError),

    #[error("Session error: {message}")]
    Session { message: String },

    #[error("Permission error: {message}")]
    Permission { message: String },

    #[error("Research error: {message}")]
    Research { message: String },

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type ApplicationResult<T> = Result<T, ApplicationError>;

/// Application configuration
#[derive(Debug, Clone)]
pub struct ApplicationConfig {
    /// Permission configuration
    pub permissions: auth::permissions::PermissionConfig,
    /// Session configuration
    pub session: SessionConfig,
    /// RAG configuration
    pub rag: wikify_rag::RagConfig,
    /// Storage configuration
    pub storage: StorageConfig,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            permissions: auth::permissions::PermissionConfig::default(),
            session: SessionConfig::default(),
            rag: wikify_rag::RagConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

impl ApplicationConfig {
    /// Create configuration for web deployment (open mode)
    pub fn web_open() -> Self {
        Self {
            permissions: auth::permissions::PermissionConfig::open(),
            session: SessionConfig::default(),
            rag: wikify_rag::RagConfig::default(),
            storage: StorageConfig::default(),
        }
    }

    /// Create configuration for web deployment (restricted mode)
    pub fn web_restricted() -> Self {
        Self {
            permissions: auth::permissions::PermissionConfig::restricted(),
            session: SessionConfig::default(),
            rag: wikify_rag::RagConfig::default(),
            storage: StorageConfig::default(),
        }
    }

    /// Create configuration for CLI usage (local mode)
    pub fn cli_local() -> Self {
        Self {
            permissions: auth::permissions::PermissionConfig::local(),
            session: SessionConfig {
                persist_session: false, // CLI doesn't need persistence by default
                ..SessionConfig::default()
            },
            rag: wikify_rag::RagConfig::default(),
            storage: StorageConfig::local(),
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Base directory for application data
    pub base_dir: std::path::PathBuf,
    /// Whether to enable persistence
    pub enable_persistence: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let base_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("wikify");

        Self {
            base_dir,
            enable_persistence: true,
        }
    }
}

impl StorageConfig {
    /// Create local storage configuration (for CLI)
    pub fn local() -> Self {
        Self {
            base_dir: std::path::PathBuf::from(".wikify"),
            enable_persistence: false,
        }
    }
}

/// Main Wikify application service
pub struct WikifyApplication {
    /// Permission manager
    permission_manager: auth::PermissionManager,
    /// Session manager
    session_manager: std::sync::Arc<SessionManager>,
    /// Research engine
    research_engine: Option<ResearchEngine>,
    /// Research template manager
    template_manager: ResearchTemplateManager,
    /// Research history storage
    history_storage: Option<FileResearchHistoryStorage>,
    /// Application configuration
    config: ApplicationConfig,
}

impl WikifyApplication {
    /// Create a new Wikify application
    pub async fn new(config: ApplicationConfig) -> ApplicationResult<Self> {
        // Create permission manager
        let permission_manager = auth::PermissionManager::new(config.permissions.clone());

        // Create session manager
        let session_manager = std::sync::Arc::new(SessionManager::new(
            config.session.clone(),
            config.rag.clone(),
        ));

        // Create research engine
        let research_engine = Some(ResearchEngine::new(
            ResearchConfig::default(),
            session_manager.clone(),
        ));

        // Create template manager
        let template_manager = ResearchTemplateManager::default();

        // Create history storage (file-based by default)
        let history_storage = {
            // Create history directory
            let history_dir = if let Some(home) = std::env::var_os("HOME") {
                std::path::PathBuf::from(home).join(".wikify/research_history")
            } else {
                std::path::PathBuf::from("./data/research_history")
            };

            match FileResearchHistoryStorage::new(&history_dir) {
                Ok(storage) => Some(storage),
                Err(e) => {
                    tracing::warn!(
                        "Failed to create research history storage: {}, continuing without history",
                        e
                    );
                    None
                }
            }
        };

        Ok(Self {
            permission_manager,
            session_manager,
            research_engine,
            template_manager,
            history_storage,
            config,
        })
    }

    /// Create a repository session
    pub async fn create_session(
        &self,
        context: &PermissionContext,
        repository_url: String,
        options: SessionOptions,
    ) -> ApplicationResult<String> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Create repository info
        let visibility = options
            .visibility
            .clone()
            .unwrap_or(session::RepositoryVisibility::Public);
        let repository =
            session::RepositoryInfo::from_url(repository_url).with_visibility(visibility);

        // Create session
        self.session_manager
            .create_session(context, repository, options)
            .await
    }

    /// Execute a query
    pub async fn query(
        &self,
        context: &PermissionContext,
        session_id: &str,
        question: String,
    ) -> ApplicationResult<QueryResponse> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Execute query
        let rag_response = self
            .session_manager
            .query_session(context, session_id, question)
            .await?;

        // Record usage
        self.permission_manager
            .record_usage(context, auth::permissions::ResourceType::Query, 1)
            .await;

        Ok(QueryResponse::from(rag_response))
    }

    /// Get session information
    pub async fn get_session(
        &self,
        context: &PermissionContext,
        session_id: &str,
    ) -> ApplicationResult<SessionInfo> {
        self.session_manager.get_session(context, session_id).await
    }

    /// List accessible sessions
    pub async fn list_sessions(&self, context: &PermissionContext) -> Vec<SessionInfo> {
        self.session_manager.list_sessions(context).await
    }

    /// Remove a session
    pub async fn remove_session(
        &self,
        context: &PermissionContext,
        session_id: &str,
    ) -> ApplicationResult<()> {
        self.session_manager
            .remove_session(context, session_id)
            .await
    }

    /// Subscribe to indexing progress updates
    pub fn subscribe_to_progress(&self) -> tokio::sync::broadcast::Receiver<IndexingUpdate> {
        self.session_manager.subscribe_to_progress()
    }

    /// Clean up stale sessions
    pub async fn cleanup_stale_sessions(&self) -> usize {
        self.session_manager.cleanup_stale_sessions().await
    }

    /// Start a deep research session
    pub async fn start_research(
        &self,
        context: &PermissionContext,
        session_id: String,
        topic: String,
        config: Option<ResearchConfig>,
    ) -> ApplicationResult<ResearchProgress> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(research_engine) = &self.research_engine {
            let research_config = config.unwrap_or_default();
            research_engine
                .start_research(context, session_id, topic, research_config)
                .await
        } else {
            Err(ApplicationError::Research {
                message: "Research engine not available".to_string(),
            })
        }
    }

    /// Execute one research iteration
    pub async fn research_iteration(
        &self,
        context: &PermissionContext,
        session_id: &str,
    ) -> ApplicationResult<ResearchProgress> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(research_engine) = &self.research_engine {
            research_engine
                .research_iteration(context, session_id)
                .await
        } else {
            Err(ApplicationError::Research {
                message: "Research engine not available".to_string(),
            })
        }
    }

    // Research Template Management

    /// List all available research templates
    pub async fn list_research_templates(
        &self,
        _context: &PermissionContext,
    ) -> ApplicationResult<Vec<ResearchTemplate>> {
        // Templates are public, no permission check needed
        Ok(self
            .template_manager
            .list_templates()
            .into_iter()
            .cloned()
            .collect())
    }

    /// Get a specific research template by ID
    pub async fn get_research_template(
        &self,
        _context: &PermissionContext,
        template_id: &str,
    ) -> ApplicationResult<Option<ResearchTemplate>> {
        Ok(self.template_manager.get_template(template_id).cloned())
    }

    /// List templates by category
    pub async fn list_templates_by_category(
        &self,
        _context: &PermissionContext,
        category: &ResearchCategory,
    ) -> ApplicationResult<Vec<ResearchTemplate>> {
        Ok(self
            .template_manager
            .list_templates_by_category(category)
            .into_iter()
            .cloned()
            .collect())
    }

    /// Start research from template
    pub async fn start_research_from_template(
        &self,
        context: &PermissionContext,
        session_id: String,
        template_id: String,
        topic: String,
        parameters: std::collections::HashMap<String, String>,
    ) -> ApplicationResult<ResearchProgress> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Get template configuration
        let (config, _template_questions) = self
            .template_manager
            .create_config_from_template(&template_id, parameters)
            .ok_or_else(|| ApplicationError::Research {
                message: format!("Template not found: {}", template_id),
            })?;

        // Start research with template configuration
        if let Some(research_engine) = &self.research_engine {
            let progress = research_engine
                .start_research(context, session_id.clone(), topic.clone(), config)
                .await?;

            // Save to history if storage is available
            if let Some(ref history_storage) = self.history_storage {
                let history_record = research::ResearchHistoryRecord {
                    session_id: session_id.clone(),
                    topic: topic.clone(),
                    template_id: Some(template_id),
                    context: research::ResearchContext {
                        session_id: session_id.clone(),
                        topic,
                        config: ResearchConfig::default(),
                        current_iteration: 0,
                        questions: Vec::new(),
                        findings: Vec::new(),
                        metadata: std::collections::HashMap::new(),
                        iterations: Vec::new(),
                        start_time: chrono::Utc::now(),
                    },
                    iterations: Vec::new(),
                    summary: None,
                    status: research::ResearchStatus::InProgress,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    completed_at: None,
                    metadata: research::ResearchMetadata {
                        total_iterations: 0,
                        total_questions: 0,
                        total_sources: 0,
                        duration_seconds: None,
                        user_id: context.identity.as_ref().map(|id| id.user_id.clone()),
                        repository_context: None, // TODO: Extract from session
                    },
                };

                if let Err(e) = history_storage.save_record(&history_record).await {
                    tracing::warn!("Failed to save research history: {}", e);
                }
            }

            Ok(progress)
        } else {
            Err(ApplicationError::Research {
                message: "Research engine not available".to_string(),
            })
        }
    }

    // Research History Management

    /// Get research history with filters
    pub async fn get_research_history(
        &self,
        context: &PermissionContext,
        filters: research::ResearchHistoryFilters,
    ) -> ApplicationResult<Vec<research::ResearchHistoryRecord>> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref history_storage) = self.history_storage {
            // Filter by user if not admin
            let mut user_filters = filters;
            if !context.permissions.contains(&Permission::Admin) {
                user_filters.user_id = context.identity.as_ref().map(|id| id.user_id.clone());
            }

            history_storage.list_records(&user_filters).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Get specific research record
    pub async fn get_research_record(
        &self,
        context: &PermissionContext,
        session_id: &str,
    ) -> ApplicationResult<Option<research::ResearchHistoryRecord>> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref history_storage) = self.history_storage {
            if let Some(record) = history_storage.load_record(session_id).await? {
                // Check if user can access this record
                if context.permissions.contains(&Permission::Admin)
                    || (context.identity.as_ref().map(|id| &id.user_id)
                        == record.metadata.user_id.as_ref())
                {
                    Ok(Some(record))
                } else {
                    Ok(None) // Access denied
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Delete research record
    pub async fn delete_research_record(
        &self,
        context: &PermissionContext,
        session_id: &str,
    ) -> ApplicationResult<()> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref history_storage) = self.history_storage {
            // Check if user can delete this record
            if let Some(record) = history_storage.load_record(session_id).await? {
                if context.permissions.contains(&Permission::Admin)
                    || (context.identity.as_ref().map(|id| &id.user_id)
                        == record.metadata.user_id.as_ref())
                {
                    history_storage.delete_record(session_id).await?;
                    Ok(())
                } else {
                    Err(ApplicationError::Permission {
                        message: "Access denied".to_string(),
                    })
                }
            } else {
                Err(ApplicationError::Research {
                    message: "Research record not found".to_string(),
                })
            }
        } else {
            Err(ApplicationError::Research {
                message: "History storage not available".to_string(),
            })
        }
    }

    /// Get research statistics
    pub async fn get_research_statistics(
        &self,
        context: &PermissionContext,
    ) -> ApplicationResult<research::ResearchStatistics> {
        // Check permissions (admin only for global stats)
        self.permission_manager
            .check_permission(context, &Permission::Admin)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref history_storage) = self.history_storage {
            history_storage.get_statistics().await
        } else {
            Ok(research::ResearchStatistics {
                total_sessions: 0,
                completed_sessions: 0,
                in_progress_sessions: 0,
                failed_sessions: 0,
                average_duration_seconds: None,
                popular_templates: Vec::new(),
                activity_by_date: std::collections::HashMap::new(),
            })
        }
    }

    /// Get research progress
    pub async fn get_research_progress(
        &self,
        context: &PermissionContext,
        session_id: &str,
    ) -> ApplicationResult<ResearchProgress> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(research_engine) = &self.research_engine {
            research_engine.get_progress(session_id).await
        } else {
            Err(ApplicationError::Research {
                message: "Research engine not available".to_string(),
            })
        }
    }

    /// Get application configuration
    pub fn config(&self) -> &ApplicationConfig {
        &self.config
    }

    /// Get permission manager
    pub fn permission_manager(&self) -> &auth::PermissionManager {
        &self.permission_manager
    }
}

/// Query response wrapper
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueryResponse {
    pub answer: String,
    pub sources: Vec<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl From<wikify_rag::RagResponse> for QueryResponse {
    fn from(rag_response: wikify_rag::RagResponse) -> Self {
        Self {
            answer: rag_response.answer,
            sources: rag_response
                .sources
                .into_iter()
                .map(|s| s.chunk.content)
                .collect(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// Convenience functions for quick setup
pub mod prelude {
    pub use super::{
        ApplicationConfig, ApplicationError, ApplicationResult, PermissionContext, QueryResponse,
        SessionConfig, SessionManager, SessionOptions, SessionStorage, UserIdentity,
        WikifyApplication,
    };

    /// Create a Wikify application with default configuration
    pub async fn create_application() -> ApplicationResult<WikifyApplication> {
        let config = ApplicationConfig::default();
        WikifyApplication::new(config).await
    }

    /// Create a Wikify application for web deployment (open mode)
    pub async fn create_web_application() -> ApplicationResult<WikifyApplication> {
        let config = ApplicationConfig::web_open();
        WikifyApplication::new(config).await
    }

    /// Create a Wikify application for CLI usage
    pub async fn create_cli_application() -> ApplicationResult<WikifyApplication> {
        let config = ApplicationConfig::cli_local();
        WikifyApplication::new(config).await
    }
}
