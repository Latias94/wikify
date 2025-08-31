use super::config::*;
use super::errors::*;
use super::storage::*;
use super::types::*;
use crate::auth::PermissionContext;
use crate::{ApplicationError, ApplicationResult};

use std::collections::HashMap;
use std::sync::Arc;
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
    config: RepositoryManagerConfig,
    /// Metrics collector
    metrics: Arc<RepositoryMetrics>,
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

        // Spawn the indexing worker
        let progress_tx = progress_broadcaster.clone();
        let storage_clone = storage.clone();
        let metrics_clone = metrics.clone();
        tokio::spawn(Self::indexing_worker(
            rag_config,
            indexing_rx,
            progress_tx,
            storage_clone,
            metrics_clone,
        ));

        Self {
            storage,
            indexing_tx,
            progress_broadcaster,
            config,
            metrics,
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

    /// Initialize the repository manager (no-op now, worker handles initialization)
    pub async fn initialize(&self) -> ApplicationResult<()> {
        info!("✅ Repository manager initialized with background worker");
        Ok(())
    }

    /// Background worker that handles all RAG operations
    async fn indexing_worker(
        rag_config: RagConfig,
        mut command_rx: mpsc::UnboundedReceiver<IndexingCommand>,
        progress_tx: broadcast::Sender<IndexingUpdate>,
        storage: Arc<dyn RepositoryStorage>,
        metrics: Arc<RepositoryMetrics>,
    ) {
        info!("🚀 Starting indexing worker");

        // Initialize RAG pipeline once in the worker
        let mut rag_pipeline = RagPipeline::new(rag_config);
        if let Err(e) = rag_pipeline.initialize().await {
            error!("❌ Failed to initialize RAG pipeline in worker: {}", e);
            return;
        }

        info!("✅ RAG pipeline initialized in worker");

        // Process commands
        while let Some(command) = command_rx.recv().await {
            match command {
                IndexingCommand::IndexRepository {
                    repository_id,
                    repository_url,
                    response_tx,
                } => {
                    info!("🔄 Worker processing index request for: {}", repository_id);

                    // Send progress update
                    let _ = progress_tx.send(IndexingUpdate::progress(
                        repository_id.clone(),
                        0.0,
                        "Starting repository indexing...".to_string(),
                    ));

                    // Index the repository
                    let result = match rag_pipeline.index_repository(&repository_url).await {
                        Ok(stats) => {
                            info!(
                                "✅ Repository indexed successfully: {} -> {}",
                                repository_id,
                                stats.summary()
                            );

                            // Send completion update
                            let _ = progress_tx.send(IndexingUpdate::complete(
                                repository_id.clone(),
                                format!("Repository indexed successfully: {}", stats.summary()),
                            ));

                            Ok(stats.summary())
                        }
                        Err(e) => {
                            error!("❌ Failed to index repository {}: {}", repository_id, e);

                            // Send error update
                            let _ = progress_tx.send(IndexingUpdate::error(
                                repository_id.clone(),
                                format!("Failed to index repository: {}", e),
                            ));

                            Err(format!("Failed to index repository: {}", e))
                        }
                    };

                    // Send response back
                    let _ = response_tx.send(result);
                }
                IndexingCommand::QueryRepository {
                    repository_id,
                    query,
                    response_tx,
                } => {
                    info!(
                        "🔍 Worker processing query for repository: {}",
                        repository_id
                    );

                    // Create RAG query
                    let rag_query = wikify_rag::create_simple_query(&query.question);

                    // Perform RAG query using the pipeline
                    let result = match rag_pipeline.ask(rag_query).await {
                        Ok(rag_response) => {
                            info!("✅ Query completed for repository: {}", repository_id);

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
                            error!("❌ Query failed for repository {}: {}", repository_id, e);
                            Err(format!("Query failed: {}", e))
                        }
                    };

                    let _ = response_tx.send(result);
                }
                IndexingCommand::StreamQueryRepository {
                    repository_id,
                    query,
                    stream_tx,
                } => {
                    info!(
                        "🔍 Worker processing stream query for repository: {}",
                        repository_id
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

        info!("🛑 Indexing worker shutting down");
    }

    /// Add a new repository
    pub async fn add_repository(
        &self,
        _context: &PermissionContext,
        url: String,
        repo_type: String,
        owner_id: Option<String>,
        options: RepositoryOptions,
    ) -> ApplicationResult<String> {
        info!("📁 Adding repository: {} (type: {})", url, repo_type);

        // Create repository index
        let mut repo = RepositoryIndex::new(url.clone(), repo_type, owner_id);

        // Add metadata if provided
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
            self.start_indexing(repo_id.clone()).await?;
        }

        Ok(repo_id)
    }

    /// Start indexing a repository using message passing
    pub async fn start_indexing(&self, repository_id: String) -> ApplicationResult<()> {
        info!("🚀 Starting indexing for repository: {}", repository_id);

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

        if let Err(_) = self.indexing_tx.send(command) {
            return Err(ApplicationError::Config {
                message: "Indexing worker is not available".to_string(),
            });
        }

        // Spawn a task to handle the response and update repository status
        let storage = self.storage.clone();
        let repo_id_clone = repository_id.clone();
        tokio::spawn(async move {
            match response_rx.await {
                Ok(Ok(_stats)) => {
                    // Update repository status to completed
                    if let Err(e) = storage
                        .update_status(&repo_id_clone, IndexingStatus::Completed, 1.0)
                        .await
                    {
                        error!("Failed to update repository status to completed: {}", e);
                    }
                    info!("✅ Repository indexing completed: {}", repo_id_clone);
                }
                Ok(Err(error_msg)) => {
                    // Update repository status to failed
                    if let Err(e) = storage
                        .update_status(&repo_id_clone, IndexingStatus::Failed, 0.0)
                        .await
                    {
                        error!("Failed to update repository status to failed: {}", e);
                    }
                    error!(
                        "❌ Repository indexing failed: {} -> {}",
                        repo_id_clone, error_msg
                    );
                }
                Err(_) => {
                    error!(
                        "❌ Failed to receive indexing response for: {}",
                        repo_id_clone
                    );
                    if let Err(e) = storage
                        .update_status(&repo_id_clone, IndexingStatus::Failed, 0.0)
                        .await
                    {
                        error!("Failed to update repository status to failed: {}", e);
                    }
                }
            }
        });

        info!(
            "✅ Indexing command sent to worker for repository: {}",
            repository_id
        );
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
