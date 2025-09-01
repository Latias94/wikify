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
pub mod repository;
pub mod research;

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
pub use repository::{
    IndexingStatus, IndexingUpdate as RepositoryIndexingUpdate, RepositoryAccessMode,
    RepositoryIndex, RepositoryManager, RepositoryOptions, RepositoryQuery,
    RepositoryQueryResponse,
};
pub use research::{
    FileResearchHistoryStorage, QuestionType, ResearchCategory, ResearchConfig, ResearchEngine,
    ResearchHistoryStorage, ResearchProgress, ResearchQuestion, ResearchResult, ResearchTemplate,
    ResearchTemplateManager,
};
// Session module removed - using Repository-based architecture

// Main application exports will be available directly from this module

/// Application-level error type
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("RAG error: {0}")]
    Rag(#[from] wikify_rag::RagError),

    #[error("Core error: {0}")]
    Core(#[from] wikify_core::WikifyError),

    #[error("Repository error: {message}")]
    Repository { message: String },

    #[error("Permission error: {message}")]
    Permission { message: String },

    #[error("Research error: {message}")]
    Research { message: String },

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Not found: {message}")]
    NotFound { message: String },

    #[error("Internal error: {message}")]
    Internal {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

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
    /// RAG configuration
    pub rag: wikify_rag::RagConfig,
    /// Storage configuration
    pub storage: StorageConfig,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            permissions: auth::permissions::PermissionConfig::default(),
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
            rag: wikify_rag::RagConfig::default(),
            storage: StorageConfig::default(),
        }
    }

    /// Create configuration for web deployment (restricted mode)
    pub fn web_restricted() -> Self {
        Self {
            permissions: auth::permissions::PermissionConfig::restricted(),
            rag: wikify_rag::RagConfig::default(),
            storage: StorageConfig::default(),
        }
    }

    /// Create configuration for CLI usage (local mode)
    pub fn cli_local() -> Self {
        Self {
            permissions: auth::permissions::PermissionConfig::local(),
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
    /// Repository manager (new architecture)
    repository_manager: std::sync::Arc<RepositoryManager>,
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

        // Create repository manager (new architecture)
        let repository_manager = std::sync::Arc::new(RepositoryManager::new(config.rag.clone()));

        // Initialize the global RAG pipeline
        repository_manager.initialize().await?;

        // Create research engine (now based on Repository Manager)
        let research_engine = Some(ResearchEngine::new(
            ResearchConfig::default(),
            repository_manager.clone(),
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
            repository_manager,
            research_engine,
            template_manager,
            history_storage,
            config,
        })
    }

    // ========================================
    // New Repository Management API
    // ========================================

    /// Add a new repository for indexing
    pub async fn add_repository(
        &self,
        context: &PermissionContext,
        url: String,
        repo_type: String,
        options: RepositoryOptions,
    ) -> ApplicationResult<String> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Add repository using the new manager
        self.repository_manager
            .add_repository(
                context,
                url,
                repo_type,
                context.user_id().map(|s| s.to_string()),
                options,
            )
            .await
    }

    /// List all repositories
    pub async fn list_repositories(
        &self,
        context: &PermissionContext,
    ) -> ApplicationResult<Vec<RepositoryIndex>> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // List repositories using the new manager
        self.repository_manager.list_repositories(context).await
    }

    /// Get a specific repository
    pub async fn get_repository(
        &self,
        context: &PermissionContext,
        repository_id: &str,
    ) -> ApplicationResult<RepositoryIndex> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Get repository using the new manager
        self.repository_manager
            .get_repository(context, repository_id)
            .await
    }

    /// Reindex a repository
    pub async fn reindex_repository(
        &self,
        context: &PermissionContext,
        repository_id: &str,
    ) -> ApplicationResult<()> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Start reindexing using the repository manager
        self.repository_manager
            .start_indexing(repository_id.to_string())
            .await
    }

    /// Delete a repository
    pub async fn delete_repository(
        &self,
        context: &PermissionContext,
        repository_id: &str,
    ) -> ApplicationResult<()> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::ManageRepository)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Delete repository using the repository manager
        self.repository_manager
            .remove_repository(context, repository_id)
            .await
    }

    /// Query a repository
    pub async fn query_repository(
        &self,
        context: &PermissionContext,
        repository_id: &str,
        query: RepositoryQuery,
    ) -> ApplicationResult<RepositoryQueryResponse> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Query repository using the new manager
        self.repository_manager
            .query_repository(context, repository_id, query)
            .await
    }

    /// Remove a repository
    pub async fn remove_repository(
        &self,
        context: &PermissionContext,
        repository_id: &str,
    ) -> ApplicationResult<()> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Remove repository using the new manager
        self.repository_manager
            .remove_repository(context, repository_id)
            .await
    }

    // ========================================
    // Research Engine API
    // ========================================

    /// Start research on a repository
    pub async fn start_research(
        &self,
        context: &PermissionContext,
        repository_id: &str,
        research_question: String,
        config: Option<research::ResearchConfig>,
    ) -> ApplicationResult<String> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Verify repository exists
        self.repository_manager
            .get_repository(context, repository_id)
            .await?;

        if let Some(ref engine) = self.research_engine {
            let research_config = config.unwrap_or_default();
            engine
                .start_research(
                    repository_id.to_string(),
                    research_question,
                    research_config,
                )
                .await
        } else {
            Err(ApplicationError::Research {
                message: "Research engine is not available".to_string(),
            })
        }
    }

    /// Execute a research iteration
    pub async fn research_iteration(
        &self,
        context: &PermissionContext,
        repository_id: &str,
        research_session_id: &str,
    ) -> ApplicationResult<research::ResearchProgress> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref engine) = self.research_engine {
            engine
                .research_iteration(context, repository_id, research_session_id)
                .await
        } else {
            Err(ApplicationError::Research {
                message: "Research engine is not available".to_string(),
            })
        }
    }

    /// Get research progress
    pub async fn get_research_progress(
        &self,
        context: &PermissionContext,
        research_session_id: &str,
    ) -> ApplicationResult<research::ResearchProgress> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref engine) = self.research_engine {
            engine.get_progress(research_session_id).await
        } else {
            Err(ApplicationError::Research {
                message: "Research engine is not available".to_string(),
            })
        }
    }

    /// List active research processes
    pub async fn list_active_research(
        &self,
        context: &PermissionContext,
    ) -> ApplicationResult<Vec<String>> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref engine) = self.research_engine {
            Ok(engine.list_active_research().await)
        } else {
            Err(ApplicationError::Research {
                message: "Research engine is not available".to_string(),
            })
        }
    }

    /// Get research details
    pub async fn get_research_details(
        &self,
        context: &PermissionContext,
        research_id: &str,
    ) -> ApplicationResult<research::ResearchContext> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref engine) = self.research_engine {
            engine.get_research_details(research_id).await
        } else {
            Err(ApplicationError::Research {
                message: "Research engine is not available".to_string(),
            })
        }
    }

    /// Cancel research
    pub async fn cancel_research(
        &self,
        context: &PermissionContext,
        research_id: &str,
    ) -> ApplicationResult<()> {
        // Check permissions
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref engine) = self.research_engine {
            engine.cancel_research(research_id).await
        } else {
            Err(ApplicationError::Research {
                message: "Research engine is not available".to_string(),
            })
        }
    }

    /// Subscribe to repository indexing progress updates
    pub fn subscribe_to_repository_progress(
        &self,
    ) -> tokio::sync::broadcast::Receiver<crate::repository::IndexingUpdate> {
        self.repository_manager.subscribe_to_progress()
    }

    // ========================================
}

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::{
        ApplicationConfig, ApplicationError, ApplicationResult, PermissionContext, RepositoryQuery,
        RepositoryQueryResponse, UserIdentity, WikifyApplication,
    };
}

impl WikifyApplication {
    /// Create a Wikify application with predefined configuration
    pub async fn create_with_config(
        config: ApplicationConfig,
    ) -> ApplicationResult<WikifyApplication> {
        WikifyApplication::new(config).await
    }

    /// Create a Wikify application for web deployment (open mode)
    pub async fn create_web_application() -> ApplicationResult<WikifyApplication> {
        Self::create_with_config(ApplicationConfig::web_open()).await
    }

    /// Create a Wikify application for web deployment (restricted mode)
    pub async fn create_web_restricted_application() -> ApplicationResult<WikifyApplication> {
        Self::create_with_config(ApplicationConfig::web_restricted()).await
    }

    /// Create a Wikify application for CLI usage
    pub async fn create_cli_application() -> ApplicationResult<WikifyApplication> {
        Self::create_with_config(ApplicationConfig::cli_local()).await
    }

    // ========================================
    // Research Template Management (TODO)
    // ========================================

    /// List all research templates
    pub async fn list_research_templates(
        &self,
    ) -> ApplicationResult<Vec<research::ResearchTemplate>> {
        // TODO: Implement research template listing
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
        template_id: &str,
    ) -> ApplicationResult<research::ResearchTemplate> {
        // TODO: Implement research template retrieval
        self.template_manager
            .get_template(template_id)
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("Research template not found: {}", template_id),
            })
    }

    /// List research templates by category
    pub async fn list_templates_by_category(
        &self,
        category: research::ResearchCategory,
    ) -> ApplicationResult<Vec<research::ResearchTemplate>> {
        // TODO: Implement template filtering by category
        Ok(self
            .template_manager
            .list_templates_by_category(&category)
            .into_iter()
            .cloned()
            .collect())
    }

    /// Start research from a template
    pub async fn start_research_from_template(
        &self,
        context: &PermissionContext,
        repository_id: &str,
        template_id: &str,
        custom_questions: Option<Vec<String>>,
        config_overrides: Option<serde_json::Value>,
    ) -> ApplicationResult<String> {
        // Get the template
        let mut template = self.get_research_template(template_id).await?;

        // Apply custom questions if provided
        if let Some(questions) = custom_questions {
            template.initial_questions = questions
                .into_iter()
                .enumerate()
                .map(|(i, text)| research::TemplateQuestion {
                    text,
                    question_type: research::QuestionType::Conceptual,
                    priority: 5,
                    complexity: 5,
                    keywords: vec![],
                })
                .collect();
        }

        // Apply config overrides if provided
        if let Some(overrides) = config_overrides {
            // TODO: Implement config override logic
            // For now, we'll use the template config as-is
        }

        // Use the first question as the main research question
        let research_question = template
            .initial_questions
            .first()
            .map(|q| q.text.clone())
            .unwrap_or_else(|| "Template-based research".to_string());

        self.start_research(
            context,
            repository_id,
            research_question,
            Some(template.config),
        )
        .await
    }

    // ========================================
    // Research History Management API
    // ========================================

    /// Get research history with optional filters
    pub async fn get_research_history(
        &self,
        context: &PermissionContext,
        filters: Option<research::ResearchHistoryFilters>,
        limit: Option<usize>,
    ) -> ApplicationResult<Vec<research::ResearchHistoryRecord>> {
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref storage) = self.history_storage {
            let mut search_filters = filters.unwrap_or_default();
            if let Some(limit_val) = limit {
                search_filters.limit = Some(limit_val);
            }
            storage.list_records(&search_filters).await
        } else {
            Ok(vec![])
        }
    }

    /// Get a specific research record by repository ID
    pub async fn get_research_record(
        &self,
        context: &PermissionContext,
        repository_id: &str,
    ) -> ApplicationResult<research::ResearchHistoryRecord> {
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref storage) = self.history_storage {
            // For now, we'll search by repository context in metadata
            // In a real implementation, you might want a more direct lookup
            let filters = research::ResearchHistoryFilters {
                limit: Some(1),
                ..Default::default()
            };

            let records = storage.list_records(&filters).await?;
            records
                .into_iter()
                .find(|r| r.context.repository_id == repository_id)
                .ok_or_else(|| ApplicationError::NotFound {
                    message: format!("No research record found for repository: {}", repository_id),
                })
        } else {
            Err(ApplicationError::Research {
                message: "Research history storage is not available".to_string(),
            })
        }
    }

    /// Delete a research record
    pub async fn delete_research_record(
        &self,
        context: &PermissionContext,
        repository_id: &str,
    ) -> ApplicationResult<()> {
        self.permission_manager
            .check_permission(context, &Permission::ManageRepository)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref storage) = self.history_storage {
            // First find the record to get the session ID
            let record = self.get_research_record(context, repository_id).await?;
            storage.delete_record(&record.session_id).await
        } else {
            Err(ApplicationError::Research {
                message: "Research history storage is not available".to_string(),
            })
        }
    }

    /// Get research statistics
    pub async fn get_research_statistics(
        &self,
        context: &PermissionContext,
    ) -> ApplicationResult<research::ResearchStatistics> {
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        if let Some(ref storage) = self.history_storage {
            storage.get_statistics().await
        } else {
            // Return empty statistics if storage is not available
            Ok(research::ResearchStatistics {
                total_sessions: 0,
                completed_sessions: 0,
                in_progress_sessions: 0,
                failed_sessions: 0,
                average_duration_seconds: None,
                popular_templates: vec![],
                activity_by_date: std::collections::HashMap::new(),
            })
        }
    }

    // ========================================
    // File Operations API
    // ========================================

    /// Get file tree for a repository
    pub async fn get_repository_file_tree(
        &self,
        context: &PermissionContext,
        repository_id: &str,
        branch: Option<String>,
    ) -> ApplicationResult<Vec<wikify_core::RepositoryFile>> {
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Get repository info from storage
        let repository = self
            .repository_manager
            .get_repository(context, repository_id)
            .await?;

        // Note: Access mode detection is now handled by UnifiedRepositoryProcessor

        // Create repository access configuration from metadata
        let config = self.create_access_config_from_metadata(&repository.metadata);

        // Create unified processor
        let base_path = self.get_base_path();
        let processor = wikify_repo::RepositoryProcessor::new(&base_path);

        // Access repository with unified interface
        let access = processor
            .access_repository(&repository.url, Some(config))
            .await
            .map_err(|e| ApplicationError::Repository {
                message: format!("Failed to access repository: {}", e),
            })?;

        // Get file tree using unified interface
        processor
            .get_file_tree(&access, branch.as_deref())
            .await
            .map_err(|e| ApplicationError::Repository {
                message: format!("Failed to get file tree: {}", e),
            })
    }

    /// Create repository access configuration from metadata
    fn create_access_config_from_metadata(
        &self,
        metadata: &std::collections::HashMap<String, String>,
    ) -> wikify_core::RepositoryAccessConfig {
        let preferred_mode = metadata
            .get("access_mode")
            .and_then(|mode| match mode.as_str() {
                "Api" => Some(wikify_core::RepoAccessMode::Api),
                "GitClone" => Some(wikify_core::RepoAccessMode::GitClone),
                "LocalDirectory" => Some(wikify_core::RepoAccessMode::LocalDirectory),
                _ => None, // Auto-detect
            });

        let force_mode = metadata
            .get("force_mode")
            .map(|v| v == "true")
            .unwrap_or(false);

        let clone_depth = metadata.get("clone_depth").and_then(|d| d.parse().ok());

        wikify_core::RepositoryAccessConfig {
            preferred_mode,
            api_token: metadata.get("api_token").cloned(),
            force_mode,
            clone_depth,
            custom_local_path: metadata.get("custom_local_path").cloned(),
        }
    }

    /// Get base path for repository operations
    fn get_base_path(&self) -> std::path::PathBuf {
        std::env::var("WIKIFY_BASE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join(".wikify")
            })
    }

    /// Get file content from a repository
    pub async fn get_repository_file_content(
        &self,
        context: &PermissionContext,
        repository_id: &str,
        file_path: &str,
        branch: Option<String>,
    ) -> ApplicationResult<String> {
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Get repository info from storage
        let repository = self
            .repository_manager
            .get_repository(context, repository_id)
            .await?;

        // Use the same access mode detection logic as file tree
        let access_mode_str = repository
            .metadata
            .get("access_mode")
            .map(|s| s.as_str())
            .unwrap_or("Auto");
        let preferred_access_mode = match access_mode_str {
            "Api" => wikify_core::RepoAccessMode::Api,
            "GitClone" => wikify_core::RepoAccessMode::GitClone,
            _ => {
                // Auto-detect based on available tokens
                let has_api_token = match repository.repo_type.as_str() {
                    "github" => std::env::var("GITHUB_TOKEN").is_ok(),
                    "gitlab" => std::env::var("GITLAB_TOKEN").is_ok(),
                    "bitbucket" => std::env::var("BITBUCKET_TOKEN").is_ok(),
                    "gitea" => {
                        std::env::var("GITEA_TOKEN").is_ok()
                            && std::env::var("GITEA_BASE_URL").is_ok()
                    }
                    _ => false,
                };

                if has_api_token {
                    wikify_core::RepoAccessMode::Api
                } else {
                    wikify_core::RepoAccessMode::GitClone
                }
            }
        };

        // Create repository access configuration from metadata
        let config = self.create_access_config_from_metadata(&repository.metadata);

        // Create unified processor
        let base_path = self.get_base_path();
        let processor = wikify_repo::RepositoryProcessor::new(&base_path);

        // Access repository with unified interface
        let access = processor
            .access_repository(&repository.url, Some(config))
            .await
            .map_err(|e| ApplicationError::Repository {
                message: format!("Failed to access repository: {}", e),
            })?;

        // Get file content using unified interface
        processor
            .get_file_content(&access, file_path, branch.as_deref())
            .await
            .map_err(|e| ApplicationError::Repository {
                message: format!("Failed to get file content: {}", e),
            })
    }

    /// Get README content for a repository
    pub async fn get_repository_readme(
        &self,
        context: &PermissionContext,
        repository_id: &str,
        branch: Option<String>,
    ) -> ApplicationResult<Option<String>> {
        self.permission_manager
            .check_permission(context, &Permission::Query)
            .await
            .map_err(|msg| ApplicationError::Permission { message: msg })?;

        // Get repository info from storage
        let repository = self
            .repository_manager
            .get_repository(context, repository_id)
            .await?;

        // Note: Access mode detection is now handled by UnifiedRepositoryProcessor

        // Create repository access configuration from metadata
        let config = self.create_access_config_from_metadata(&repository.metadata);

        // Create unified processor
        let base_path = self.get_base_path();
        let processor = wikify_repo::RepositoryProcessor::new(&base_path);

        // Access repository with unified interface
        let access = processor
            .access_repository(&repository.url, Some(config))
            .await
            .map_err(|e| ApplicationError::Repository {
                message: format!("Failed to access repository: {}", e),
            })?;

        // Get README using unified interface
        processor
            .get_readme(&access, branch.as_deref())
            .await
            .map_err(|e| ApplicationError::Repository {
                message: format!("Failed to get README: {}", e),
            })
    }
}
