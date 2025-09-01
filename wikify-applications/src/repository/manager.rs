use super::config::*;
use super::errors::*;
use super::storage::*;
use super::types::{QueryChunkType, QueryStreamChunk, *};
use crate::auth::PermissionContext;
use crate::{ApplicationError, ApplicationResult};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};

use wikify_rag::{RagConfig, RagPipeline};

/// Indexing command sent to the indexing worker
#[derive(Debug)]
pub enum IndexingCommand {
    /// Index a repository
    IndexRepository {
        repository_id: String,
        repository_url: String,
        response_tx: tokio::sync::oneshot::Sender<Result<String, String>>,
    },
    /// Query a repository
    QueryRepository {
        repository_id: String,
        query: RepositoryQuery,
        response_tx: tokio::sync::oneshot::Sender<Result<RepositoryQueryResponse, String>>,
    },
    /// Stream query a repository (for real-time responses)
    StreamQueryRepository {
        repository_id: String,
        query: RepositoryQuery,
        stream_tx: tokio::sync::mpsc::UnboundedSender<QueryStreamChunk>,
    },
}

/// Repository manager handles all repository operations using message passing
pub struct RepositoryManager {
    /// Storage backend for repository persistence
    storage: Arc<dyn RepositoryStorage>,
    /// Command sender to the indexing worker
    indexing_tx: mpsc::UnboundedSender<IndexingCommand>,
    /// Progress broadcaster for indexing updates
    progress_broadcaster: broadcast::Sender<IndexingUpdate>,
    /// Configuration for the repository manager
    #[allow(dead_code)]
    config: RepositoryManagerConfig,
    /// Metrics collector
    metrics: Arc<RepositoryMetrics>,
    /// Worker health status
    worker_healthy: Arc<RwLock<bool>>,
}

impl RepositoryManager {
    /// Create a new repository manager with memory storage
    pub fn new(rag_config: RagConfig) -> Self {
        let storage = Arc::new(MemoryRepositoryStorage::new());
        let config = RepositoryManagerConfig::default();
        Self::with_storage(rag_config, storage, config)
    }

    /// Create a new repository manager with custom storage backend
    pub fn with_storage(
        rag_config: RagConfig,
        storage: Arc<dyn RepositoryStorage>,
        config: RepositoryManagerConfig,
    ) -> Self {
        let (progress_broadcaster, _) = broadcast::channel(1000);
        let (indexing_tx, indexing_rx) = mpsc::unbounded_channel();
        let metrics = Arc::new(RepositoryMetrics::default());
        let worker_healthy = Arc::new(RwLock::new(false));

        // Spawn the enhanced indexing worker with better logging
        let progress_tx = progress_broadcaster.clone();
        let storage_clone = storage.clone();
        let metrics_clone = metrics.clone();
        let worker_healthy_clone = worker_healthy.clone();
        tokio::spawn(Self::enhanced_indexing_worker(
            rag_config,
            indexing_rx,
            progress_tx,
            storage_clone,
            metrics_clone,
            worker_healthy_clone,
        ));

        Self {
            storage,
            indexing_tx,
            progress_broadcaster,
            config,
            metrics,
            worker_healthy,
        }
    }

    /// Create repository manager with SQLite storage
    #[cfg(feature = "sqlite")]
    pub async fn with_sqlite(
        rag_config: RagConfig,
        database_url: &str,
        config: RepositoryManagerConfig,
    ) -> RepositoryResult<Self> {
        let sqlite_storage = SqliteRepositoryStorage::from_url(database_url).await?;
        sqlite_storage.migrate().await?;

        let storage = Arc::new(sqlite_storage);
        Ok(Self::with_storage(rag_config, storage, config))
    }

    /// Initialize the repository manager and check worker health
    pub async fn initialize(&self) -> ApplicationResult<()> {
        eprintln!("🔄 Checking repository manager worker health...");

        // Wait a bit for the worker to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        // Check if worker is healthy
        let is_healthy = *self.worker_healthy.read().await;
        eprintln!("🔍 Worker health status: {}", is_healthy);

        if !is_healthy {
            eprintln!("❌ Repository manager worker is not healthy!");
            eprintln!("Please check the logs above for RAG pipeline initialization errors.");
            return Err(ApplicationError::Config {
                message: "Repository indexing worker failed to initialize. Please check server logs for RAG pipeline initialization errors.".to_string(),
            });
        }

        eprintln!("✅ Repository manager initialized with healthy background worker");
        Ok(())
    }

    /// Enhanced background worker with better logging and error handling
    async fn enhanced_indexing_worker(
        rag_config: RagConfig,
        mut command_rx: mpsc::UnboundedReceiver<IndexingCommand>,
        progress_tx: broadcast::Sender<IndexingUpdate>,
        _storage: Arc<dyn RepositoryStorage>,
        _metrics: Arc<RepositoryMetrics>,
        worker_healthy: Arc<RwLock<bool>>,
    ) {
        info!("🚀 Starting enhanced RAG indexing worker");

        // Check environment variables before initialization with structured logging
        info!("🔍 Checking LLM API configuration...");
        let mut api_keys_found = Vec::new();

        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            info!("✅ OPENAI_API_KEY configured (length: {})", key.len());
            api_keys_found.push("OpenAI");
        } else {
            warn!("❌ OPENAI_API_KEY not found");
        }

        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            info!("✅ ANTHROPIC_API_KEY configured (length: {})", key.len());
            api_keys_found.push("Anthropic");
        } else {
            warn!("❌ ANTHROPIC_API_KEY not found");
        }

        if api_keys_found.is_empty() {
            error!("❌ No LLM API keys found. RAG pipeline will not function properly.");
        } else {
            info!("📋 Available LLM providers: {}", api_keys_found.join(", "));
        }

        // Initialize RAG pipeline once in the worker
        let mut rag_pipeline = RagPipeline::new(rag_config);

        info!("📝 Initializing RAG pipeline...");

        match rag_pipeline.initialize().await {
            Ok(()) => {
                info!("✅ RAG pipeline initialized successfully");
                *worker_healthy.write().await = true;
            }
            Err(e) => {
                error!(
                    error = %e,
                    "❌ Failed to initialize RAG pipeline"
                );
                warn!("💡 Hint: Ensure LLM API keys are configured correctly");
                debug!("📊 Error details: {:?}", e);
                *worker_healthy.write().await = false;

                // Keep the worker alive but unhealthy to handle status checks
                warn!("🔄 Worker entering unhealthy mode - will reject all requests");
                loop {
                    if let Some(command) = command_rx.recv().await {
                        Self::handle_unhealthy_command(command).await;
                    } else {
                        warn!("📡 Command channel closed, worker shutting down");
                        break;
                    }
                }
                return;
            }
        }

        eprintln!("✅ RAG pipeline initialized in worker, ready to process commands");

        // Process commands
        while let Some(command) = command_rx.recv().await {
            match command {
                IndexingCommand::IndexRepository {
                    repository_id,
                    repository_url,
                    response_tx,
                } => {
                    let start_time = Instant::now();
                    info!(
                        repository_id = %repository_id,
                        repository_url = %repository_url,
                        "🔄 Starting repository indexing"
                    );

                    // Send progress update
                    let _ = progress_tx.send(IndexingUpdate::progress(
                        repository_id.clone(),
                        0.0,
                        "Initializing repository indexing...".to_string(),
                    ));

                    // Index the repository with detailed error handling
                    let result = Self::handle_repository_indexing(
                        &mut rag_pipeline,
                        &repository_id,
                        &repository_url,
                        &progress_tx,
                        start_time,
                    )
                    .await;

                    // Send response back
                    let _ = response_tx.send(result);
                }
                IndexingCommand::QueryRepository {
                    repository_id,
                    query,
                    response_tx,
                } => {
                    let start_time = Instant::now();
                    info!(
                        repository_id = %repository_id,
                        question = %query.question,
                        "🔍 Processing repository query"
                    );

                    // Create RAG query
                    let rag_query = wikify_rag::create_simple_query(&query.question);

                    // Perform RAG query using the pipeline
                    let result = match rag_pipeline.ask(rag_query).await {
                        Ok(rag_response) => {
                            let duration = start_time.elapsed();
                            info!(
                                repository_id = %repository_id,
                                duration_ms = duration.as_millis(),
                                sources_count = rag_response.sources.len(),
                                retrieval_time_ms = rag_response.metadata.retrieval_time_ms,
                                generation_time_ms = rag_response.metadata.generation_time_ms,
                                "✅ Query completed successfully"
                            );

                            // Convert RAG response to our format
                            let mut sources = Vec::new();
                            let mut metadata = HashMap::new();

                            // Extract source information from RAG response
                            for search_result in rag_response.sources.iter().take(5) {
                                // Limit to top 5 sources
                                if let Some(file_path) =
                                    search_result.chunk.metadata.get("file_path")
                                {
                                    if let Some(path_str) = file_path.as_str() {
                                        sources.push(path_str.to_string());
                                    }
                                } else if let Some(source) =
                                    search_result.chunk.metadata.get("source")
                                {
                                    if let Some(source_str) = source.as_str() {
                                        sources.push(source_str.to_string());
                                    }
                                } else {
                                    sources.push(format!(
                                        "Document chunk: {}",
                                        search_result
                                            .chunk
                                            .content
                                            .chars()
                                            .take(50)
                                            .collect::<String>()
                                    ));
                                }
                            }

                            // Add metadata about the search
                            metadata.insert(
                                "total_sources".to_string(),
                                rag_response.sources.len().to_string(),
                            );
                            metadata.insert("repository_id".to_string(), repository_id.clone());
                            metadata.insert(
                                "retrieval_time_ms".to_string(),
                                rag_response.metadata.retrieval_time_ms.to_string(),
                            );
                            metadata.insert(
                                "generation_time_ms".to_string(),
                                rag_response.metadata.generation_time_ms.to_string(),
                            );

                            let response = RepositoryQueryResponse {
                                answer: rag_response.answer,
                                sources,
                                confidence: Some(0.8), // TODO: Calculate actual confidence from RAG response
                                metadata,
                            };

                            Ok(response)
                        }
                        Err(e) => {
                            let duration = start_time.elapsed();
                            error!(
                                repository_id = %repository_id,
                                duration_ms = duration.as_millis(),
                                error = %e,
                                "❌ Query failed"
                            );
                            Err(format!(
                                "Query failed after {:.2}s: {}",
                                duration.as_secs_f64(),
                                e
                            ))
                        }
                    };

                    let _ = response_tx.send(result);
                }
                IndexingCommand::StreamQueryRepository {
                    repository_id,
                    query,
                    stream_tx,
                } => {
                    let _start_time = Instant::now();
                    info!(
                        repository_id = %repository_id,
                        question = %query.question,
                        "🔍 Processing stream query"
                    );

                    // Send initial chunk to indicate query started
                    let _ = stream_tx.send(QueryStreamChunk {
                        chunk_type: QueryChunkType::Content,
                        content: "".to_string(),
                        is_final: false,
                        sources: None,
                        metadata: None,
                    });

                    // For now, simulate streaming by chunking a regular response
                    // TODO: Implement true streaming when wikify-rag supports it
                    let rag_query = wikify_rag::create_simple_query(&query.question);

                    match rag_pipeline.ask(rag_query).await {
                        Ok(rag_response) => {
                            // Simulate streaming by sending the response in chunks
                            let words: Vec<&str> = rag_response.answer.split_whitespace().collect();
                            let chunk_size = 5; // Send 5 words at a time

                            for (i, chunk) in words.chunks(chunk_size).enumerate() {
                                let content = chunk.join(" ");
                                let is_final = i == (words.len() + chunk_size - 1) / chunk_size - 1;

                                let _ = stream_tx.send(QueryStreamChunk {
                                    chunk_type: QueryChunkType::Content,
                                    content: if is_final {
                                        content
                                    } else {
                                        format!("{} ", content)
                                    },
                                    is_final: false,
                                    sources: None,
                                    metadata: None,
                                });

                                // Small delay to simulate streaming
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            }

                            // Extract sources
                            let mut sources = Vec::new();
                            let mut metadata = HashMap::new();

                            for search_result in rag_response.sources.iter().take(5) {
                                if let Some(file_path) =
                                    search_result.chunk.metadata.get("file_path")
                                {
                                    if let Some(path_str) = file_path.as_str() {
                                        sources.push(path_str.to_string());
                                    }
                                } else if let Some(source) =
                                    search_result.chunk.metadata.get("source")
                                {
                                    if let Some(source_str) = source.as_str() {
                                        sources.push(source_str.to_string());
                                    }
                                } else {
                                    sources.push(format!(
                                        "Document chunk: {}",
                                        search_result
                                            .chunk
                                            .content
                                            .chars()
                                            .take(50)
                                            .collect::<String>()
                                    ));
                                }
                            }

                            metadata.insert(
                                "total_sources".to_string(),
                                rag_response.sources.len().to_string(),
                            );
                            metadata.insert("repository_id".to_string(), repository_id.clone());
                            metadata.insert(
                                "retrieval_time_ms".to_string(),
                                rag_response.metadata.retrieval_time_ms.to_string(),
                            );
                            metadata.insert(
                                "generation_time_ms".to_string(),
                                rag_response.metadata.generation_time_ms.to_string(),
                            );

                            // Send final completion chunk
                            let _ = stream_tx.send(QueryStreamChunk {
                                chunk_type: QueryChunkType::Complete,
                                content: "".to_string(),
                                is_final: true,
                                sources: Some(sources),
                                metadata: Some(metadata),
                            });

                            info!(
                                "✅ Stream query completed for repository: {}",
                                repository_id
                            );
                        }
                        Err(e) => {
                            error!(
                                "❌ Stream query failed for repository {}: {}",
                                repository_id, e
                            );
                            let _ = stream_tx.send(QueryStreamChunk {
                                chunk_type: QueryChunkType::Error,
                                content: format!("Query failed: {}", e),
                                is_final: true,
                                sources: None,
                                metadata: None,
                            });
                        }
                    }
                }
            }
        }

        info!("🛑 Enhanced indexing worker shutting down");
    }

    /// Handle repository indexing with detailed progress tracking and multiple access modes
    async fn handle_repository_indexing(
        rag_pipeline: &mut RagPipeline,
        repository_id: &str,
        repository_url: &str,
        progress_tx: &broadcast::Sender<IndexingUpdate>,
        start_time: Instant,
    ) -> Result<String, String> {
        // Send initial progress
        let _ = progress_tx.send(IndexingUpdate::progress(
            repository_id.to_string(),
            0.1,
            "Analyzing repository access options...".to_string(),
        ));

        // Get repository access information using unified processor
        let _access_info = Self::get_repository_access_info(repository_url).await;

        let _ = progress_tx.send(IndexingUpdate::progress(
            repository_id.to_string(),
            0.2,
            "Preparing repository indexing...".to_string(),
        ));

        match rag_pipeline.index_repository(repository_url).await {
            Ok(stats) => {
                let duration = start_time.elapsed();
                info!(
                    repository_id = %repository_id,
                    duration_ms = duration.as_millis(),
                    total_documents = stats.total_documents,
                    total_chunks = stats.total_chunks,
                    "✅ Repository indexing completed successfully"
                );

                // Send completion update
                let _ = progress_tx.send(IndexingUpdate::complete(
                    repository_id.to_string(),
                    format!(
                        "Indexed {} documents ({} chunks) in {:.2}s",
                        stats.total_documents,
                        stats.total_chunks,
                        duration.as_secs_f64()
                    ),
                ));

                Ok(stats.summary())
            }
            Err(e) => {
                let duration = start_time.elapsed();
                error!(
                    repository_id = %repository_id,
                    duration_ms = duration.as_millis(),
                    error = %e,
                    "❌ Repository indexing failed"
                );

                // Send error update
                let _ = progress_tx.send(IndexingUpdate::error(
                    repository_id.to_string(),
                    format!(
                        "Indexing failed after {:.2}s: {}",
                        duration.as_secs_f64(),
                        e
                    ),
                ));

                Err(format!("Failed to index repository: {}", e))
            }
        }
    }

    /// Extract enhanced repository metadata using unified processor
    async fn extract_repository_metadata(
        &self,
        url: &str,
        _repo_type: &str,
    ) -> ApplicationResult<HashMap<String, String>> {
        use wikify_core::RepositoryAccessConfig;
        use wikify_repo::RepositoryProcessor;

        let mut metadata = HashMap::new();

        // Use unified processor to get repository access
        let base_path = std::env::var("WIKIFY_BASE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join(".wikify")
            });

        let processor = RepositoryProcessor::new(&base_path);
        let config = RepositoryAccessConfig::default();

        match processor.access_repository(url, Some(config)).await {
            Ok(access) => {
                let repo_info = &access.repo_info;

                // Add basic parsed information
                metadata.insert("owner".to_string(), repo_info.owner.clone());
                metadata.insert("name".to_string(), repo_info.name.clone());
                metadata.insert(
                    "parsed_repo_type".to_string(),
                    format!("{:?}", repo_info.repo_type),
                );
                metadata.insert(
                    "access_mode".to_string(),
                    format!("{:?}", access.access_mode),
                );

                // Try to get additional metadata via API if using API mode
                if matches!(access.access_mode, wikify_core::RepoAccessMode::Api) {
                    if let Ok(api_metadata) = self
                        .try_api_metadata_extraction(
                            &repo_info,
                            &format!("{:?}", repo_info.repo_type).to_lowercase(),
                        )
                        .await
                    {
                        metadata.extend(api_metadata);
                    }
                }

                info!(
                    url = %url,
                    access_mode = ?access.access_mode,
                    "✅ Repository metadata extracted via unified processor"
                );
            }
            Err(e) => {
                warn!(
                    url = %url,
                    error = %e,
                    "⚠️ Failed to extract repository metadata"
                );
            }
        }

        Ok(metadata)
    }

    /// Try to extract metadata via API (best effort, non-blocking)
    async fn try_api_metadata_extraction(
        &self,
        repo_info: &wikify_core::RepoInfo,
        repo_type: &str,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error + Send + Sync>> {
        use wikify_repo::{ApiClientConfig, ApiClientFactory};

        let mut metadata = HashMap::new();

        // Create API client configuration
        let config = match repo_type.to_lowercase().as_str() {
            "github" => ApiClientConfig::github(std::env::var("GITHUB_TOKEN").ok()),
            "gitlab" => ApiClientConfig::gitlab(None, std::env::var("GITLAB_TOKEN").ok()),
            "bitbucket" => ApiClientConfig::bitbucket(std::env::var("BITBUCKET_TOKEN").ok()),
            "gitea" => {
                if let Ok(base_url) = std::env::var("GITEA_BASE_URL") {
                    ApiClientConfig::gitea(base_url, std::env::var("GITEA_TOKEN").ok())
                } else {
                    return Ok(metadata);
                }
            }
            _ => return Ok(metadata),
        };

        // Create API client
        let client = ApiClientFactory::create_client(repo_type, config)?;

        // Get repository metadata with timeout
        let repo_metadata = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            client.get_repository_metadata(&repo_info.owner, &repo_info.name),
        )
        .await??;

        // Convert to string metadata
        let language = repo_metadata
            .language
            .as_deref()
            .unwrap_or("unknown")
            .to_string();
        let description = repo_metadata.description.unwrap_or_default();

        metadata.insert("description".to_string(), description);
        metadata.insert("default_branch".to_string(), repo_metadata.default_branch);
        metadata.insert("language".to_string(), language.clone());
        metadata.insert("topics".to_string(), repo_metadata.topics.join(","));
        metadata.insert(
            "size_kb".to_string(),
            repo_metadata.size.unwrap_or(0).to_string(),
        );
        metadata.insert("private".to_string(), repo_metadata.private.to_string());

        info!(
            owner = %repo_info.owner,
            name = %repo_info.name,
            language = %language,
            "📊 Successfully extracted API metadata"
        );

        Ok(metadata)
    }

    /// Get repository access information using the unified processor
    async fn get_repository_access_info(repository_url: &str) -> wikify_core::RepoInfo {
        use wikify_core::RepositoryAccessConfig;
        use wikify_repo::RepositoryProcessor;

        // Use the unified processor to determine access mode
        let base_path = std::env::var("WIKIFY_BASE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join(".wikify")
            });

        let processor = RepositoryProcessor::new(&base_path);
        let config = RepositoryAccessConfig::default(); // Use auto-detection

        match processor
            .access_repository(repository_url, Some(config))
            .await
        {
            Ok(access) => {
                info!(
                    repository_url = %repository_url,
                    access_mode = ?access.access_mode,
                    "✅ Repository access determined via unified processor"
                );
                access.repo_info
            }
            Err(e) => {
                warn!(
                    repository_url = %repository_url,
                    error = %e,
                    "⚠️ Failed to determine repository access, creating fallback info"
                );
                // Create fallback repo info for local paths
                wikify_core::RepoInfo {
                    owner: "local".to_string(),
                    name: std::path::Path::new(repository_url)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    repo_type: wikify_core::RepoType::Local,
                    url: repository_url.to_string(),
                    access_token: None,
                    local_path: Some(repository_url.to_string()),
                    access_mode: wikify_core::RepoAccessMode::LocalDirectory,
                }
            }
        }
    }

    /// Handle commands when worker is in unhealthy state
    async fn handle_unhealthy_command(command: IndexingCommand) {
        let error_msg =
            "RAG pipeline not initialized. Check server logs for LLM API configuration.";

        match command {
            IndexingCommand::IndexRepository {
                repository_id,
                response_tx,
                ..
            } => {
                warn!(
                    repository_id = %repository_id,
                    "🚫 Rejecting index request - worker unhealthy"
                );
                let _ = response_tx.send(Err(error_msg.to_string()));
            }
            IndexingCommand::QueryRepository {
                repository_id,
                response_tx,
                ..
            } => {
                warn!(
                    repository_id = %repository_id,
                    "🚫 Rejecting query request - worker unhealthy"
                );
                let _ = response_tx.send(Err(error_msg.to_string()));
            }
            IndexingCommand::StreamQueryRepository {
                repository_id,
                stream_tx,
                ..
            } => {
                warn!(
                    repository_id = %repository_id,
                    "🚫 Rejecting stream query request - worker unhealthy"
                );
                let _ = stream_tx.send(QueryStreamChunk {
                    chunk_type: QueryChunkType::Error,
                    content: error_msg.to_string(),
                    is_final: true,
                    sources: None,
                    metadata: None,
                });
            }
        }
    }

    /// Add a new repository with enhanced metadata extraction
    pub async fn add_repository(
        &self,
        _context: &PermissionContext,
        url: String,
        repo_type: String,
        owner_id: Option<String>,
        options: RepositoryOptions,
    ) -> ApplicationResult<String> {
        info!(
            url = %url,
            repo_type = %repo_type,
            "📁 Adding repository with enhanced metadata extraction"
        );

        // Create repository index
        let mut repo = RepositoryIndex::new(url.clone(), repo_type.clone(), owner_id);

        // Enhanced: Extract repository metadata using wikify-repo
        if let Ok(enhanced_metadata) = self.extract_repository_metadata(&url, &repo_type).await {
            info!(
                repository_id = %repo.id,
                "✅ Enhanced metadata extracted successfully"
            );
            repo.metadata.extend(enhanced_metadata);
        } else {
            warn!(
                repository_id = %repo.id,
                "⚠️ Could not extract enhanced metadata, using basic info"
            );
        }

        // Add user-provided metadata if provided
        if let Some(metadata) = options.metadata {
            repo.metadata.extend(metadata);
        }

        let repo_id = repo.id.clone();

        // Store repository in persistent storage
        self.storage
            .save_repository(&repo)
            .await
            .map_err(|e| ApplicationError::Internal {
                message: format!("Failed to save repository: {}", e),
                source: None,
            })?;

        // Update metrics
        self.metrics
            .total_repositories
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics
            .update_repository_status(None, repo.status.clone());

        info!("✅ Repository added: {} -> {}", url, repo_id);

        // Start indexing if requested
        if options.auto_index {
            info!(
                repository_id = %repo_id,
                "🔄 Auto-index enabled, starting indexing"
            );
            self.start_indexing(repo_id.clone()).await?;
        } else {
            info!(
                repository_id = %repo_id,
                "⏸️ Auto-index disabled"
            );
        }

        Ok(repo_id)
    }

    /// Start indexing a repository using message passing
    pub async fn start_indexing(&self, repository_id: String) -> ApplicationResult<()> {
        info!(
            repository_id = %repository_id,
            "🚀 Initiating repository indexing"
        );

        // Check if worker is healthy
        let is_healthy = *self.worker_healthy.read().await;
        if !is_healthy {
            error!(
                repository_id = %repository_id,
                "❌ Cannot start indexing: worker is not healthy"
            );
            return Err(ApplicationError::Config {
                message: "Cannot start indexing: RAG worker is not healthy. Please check LLM API configuration.".to_string(),
            });
        }

        // Get repository and update status
        let repo = self
            .storage
            .load_repository(&repository_id)
            .await
            .map_err(|e| ApplicationError::Internal {
                message: format!("Failed to load repository: {}", e),
                source: None,
            })?
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("Repository not found: {}", repository_id),
            })?;

        // Update status to indexing
        self.storage
            .update_status(&repository_id, IndexingStatus::Indexing, 0.0)
            .await
            .map_err(|e| ApplicationError::Internal {
                message: format!("Failed to update repository status: {}", e),
                source: None,
            })?;

        let repository_url = repo.url.clone();

        // Create response channel
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // Send indexing command to worker
        let command = IndexingCommand::IndexRepository {
            repository_id: repository_id.clone(),
            repository_url,
            response_tx,
        };

        debug!(
            repository_id = %repository_id,
            "📤 Sending indexing command to worker"
        );

        if let Err(_) = self.indexing_tx.send(command) {
            error!(
                repository_id = %repository_id,
                "❌ Failed to send indexing command - worker channel closed"
            );
            return Err(ApplicationError::Config {
                message: "RAG indexing worker is not available".to_string(),
            });
        }

        info!(
            repository_id = %repository_id,
            "✅ Indexing command queued successfully"
        );

        // Spawn a task to handle the response and update repository status
        let storage = self.storage.clone();
        let repo_id_clone = repository_id.clone();
        tokio::spawn(async move {
            match response_rx.await {
                Ok(Ok(stats_summary)) => {
                    // Update repository status to completed
                    if let Err(e) = storage
                        .update_status(&repo_id_clone, IndexingStatus::Completed, 1.0)
                        .await
                    {
                        error!(
                            repository_id = %repo_id_clone,
                            error = %e,
                            "Failed to update repository status to completed"
                        );
                    } else {
                        info!(
                            repository_id = %repo_id_clone,
                            stats = %stats_summary,
                            "✅ Repository indexing completed successfully"
                        );
                    }
                }
                Ok(Err(error_msg)) => {
                    // Update repository status to failed
                    if let Err(e) = storage
                        .update_status(&repo_id_clone, IndexingStatus::Failed, 0.0)
                        .await
                    {
                        error!(
                            repository_id = %repo_id_clone,
                            error = %e,
                            "Failed to update repository status to failed"
                        );
                    }
                    error!(
                        repository_id = %repo_id_clone,
                        error_msg = %error_msg,
                        "❌ Repository indexing failed"
                    );
                }
                Err(_) => {
                    error!(
                        repository_id = %repo_id_clone,
                        "❌ Failed to receive indexing response - worker may have crashed"
                    );
                    if let Err(e) = storage
                        .update_status(&repo_id_clone, IndexingStatus::Failed, 0.0)
                        .await
                    {
                        error!(
                            repository_id = %repo_id_clone,
                            error = %e,
                            "Failed to update repository status to failed"
                        );
                    }
                }
            }
        });

        Ok(())
    }

    /// List all repositories
    pub async fn list_repositories(
        &self,
        context: &PermissionContext,
    ) -> ApplicationResult<Vec<RepositoryIndex>> {
        let owner_id = context.identity.as_ref().map(|u| u.user_id.as_str());
        let repos = self
            .storage
            .list_repositories(owner_id)
            .await
            .map_err(|e| ApplicationError::Internal {
                message: format!("Failed to list repositories: {}", e),
                source: None,
            })?;

        info!("📋 Listed {} repositories", repos.len());
        Ok(repos)
    }

    /// Get a specific repository
    pub async fn get_repository(
        &self,
        _context: &PermissionContext,
        repository_id: &str,
    ) -> ApplicationResult<RepositoryIndex> {
        self.storage
            .load_repository(repository_id)
            .await
            .map_err(|e| ApplicationError::Internal {
                message: format!("Failed to load repository: {}", e),
                source: None,
            })?
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("Repository not found: {}", repository_id),
            })
    }

    /// Query a repository using message passing
    pub async fn query_repository(
        &self,
        _context: &PermissionContext,
        repository_id: &str,
        query: RepositoryQuery,
    ) -> ApplicationResult<RepositoryQueryResponse> {
        // Check if repository exists and is ready
        let repo = self
            .storage
            .load_repository(repository_id)
            .await
            .map_err(|e| ApplicationError::Internal {
                message: format!("Failed to load repository: {}", e),
                source: None,
            })?
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("Repository not found: {}", repository_id),
            })?;

        if !repo.is_ready() {
            return Err(ApplicationError::Config {
                message: format!(
                    "Repository not ready for querying: {} (status: {:?})",
                    repository_id, repo.status
                ),
            });
        }

        // Create response channel
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // Send query command to worker
        let command = IndexingCommand::QueryRepository {
            repository_id: repository_id.to_string(),
            query,
            response_tx,
        };

        if let Err(_) = self.indexing_tx.send(command) {
            return Err(ApplicationError::Config {
                message: "Indexing worker is not available".to_string(),
            });
        }

        // Wait for response
        match response_rx.await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(error_msg)) => Err(ApplicationError::Config {
                message: format!("Query failed: {}", error_msg),
            }),
            Err(_) => Err(ApplicationError::Config {
                message: "Failed to receive query response".to_string(),
            }),
        }
    }

    /// Stream query a repository for real-time responses
    pub async fn stream_query_repository(
        &self,
        _context: &PermissionContext,
        repository_id: &str,
        query: RepositoryQuery,
    ) -> ApplicationResult<tokio::sync::mpsc::UnboundedReceiver<QueryStreamChunk>> {
        // Check if repository exists and is ready
        let repo = self
            .storage
            .load_repository(repository_id)
            .await
            .map_err(|e| ApplicationError::Internal {
                message: format!("Failed to load repository: {}", e),
                source: None,
            })?
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("Repository not found: {}", repository_id),
            })?;

        if !repo.is_ready() {
            return Err(ApplicationError::Config {
                message: format!(
                    "Repository not ready for querying: {} (status: {:?})",
                    repository_id, repo.status
                ),
            });
        }

        // Create stream channel
        let (stream_tx, stream_rx) = tokio::sync::mpsc::unbounded_channel();

        // Send stream query command to worker
        let command = IndexingCommand::StreamQueryRepository {
            repository_id: repository_id.to_string(),
            query,
            stream_tx,
        };

        if let Err(_) = self.indexing_tx.send(command) {
            return Err(ApplicationError::Config {
                message: "Indexing worker is not available".to_string(),
            });
        }

        Ok(stream_rx)
    }

    /// Subscribe to indexing progress updates
    pub fn subscribe_to_progress(&self) -> broadcast::Receiver<IndexingUpdate> {
        self.progress_broadcaster.subscribe()
    }

    /// Remove a repository
    pub async fn remove_repository(
        &self,
        _context: &PermissionContext,
        repository_id: &str,
    ) -> ApplicationResult<()> {
        self.storage
            .delete_repository(repository_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound { .. } => ApplicationError::NotFound {
                    message: format!("Repository not found: {}", repository_id),
                },
                _ => ApplicationError::Internal {
                    message: format!("Failed to delete repository: {}", e),
                    source: None,
                },
            })?;

        info!("🗑️ Repository removed: {}", repository_id);
        Ok(())
    }
}
