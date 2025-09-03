//! RAG Pipeline - Complete Retrieval-Augmented Generation system
//!
//! This module orchestrates the entire RAG workflow: indexing documents,
//! retrieving relevant context, and generating responses using LLMs.

use crate::embeddings::{EmbeddingGenerator, VectorStore};
use crate::indexing::traits::DocumentIndexerImpl;
use crate::llm_client::WikifyLlmClient;
use crate::retriever::DocumentRetriever;
use crate::types::{
    DeepResearchConfig, DeepResearchResult, RagConfig, RagError, RagQuery, RagResponse,
    RagResponseMetadata, RagResult, ResearchStatus, SearchResult,
};
use wikify_core::{log_operation_start, log_operation_success};

use std::path::Path;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Complete RAG pipeline that handles indexing, retrieval, and generation
pub struct RagPipeline {
    config: RagConfig,
    vector_store: Option<VectorStore>,
    retriever: Option<DocumentRetriever>,
    llm_client: Option<WikifyLlmClient>,
    is_initialized: bool,
}

impl RagPipeline {
    /// Create a new RAG pipeline with configuration
    pub fn new(config: RagConfig) -> Self {
        Self {
            config,
            vector_store: None,
            retriever: None,
            llm_client: None,
            is_initialized: false,
        }
    }

    /// Create a RAG pipeline with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RagConfig::default())
    }

    /// Initialize the RAG pipeline
    pub async fn initialize(&mut self) -> RagResult<()> {
        log_operation_start!("rag_pipeline_init");

        eprintln!("🚀 Initializing RAG pipeline");

        // Initialize LLM client
        eprintln!("📝 Creating LLM client...");
        self.llm_client = Some(WikifyLlmClient::new(self.config.llm.clone()).await?);
        eprintln!("✅ LLM client initialized");

        // Initialize vector store (empty for now)
        self.vector_store = Some(VectorStore::new(self.config.embeddings.dimension));
        eprintln!("✅ Vector store initialized");

        self.is_initialized = true;

        log_operation_success!("rag_pipeline_init");
        eprintln!("🎉 RAG pipeline initialization complete");

        Ok(())
    }

    /// Index a repository and prepare it for querying
    pub async fn index_repository<P: AsRef<Path>>(
        &mut self,
        repo_path_or_url: P,
    ) -> RagResult<IndexingStats> {
        self.index_repository_with_progress(repo_path_or_url, None)
            .await
    }

    /// Index a repository using enhanced indexer with advanced AST-aware code splitting
    pub async fn index_repository_enhanced<P: AsRef<Path>>(
        &mut self,
        repo_path_or_url: P,
    ) -> RagResult<IndexingStats> {
        self.index_repository_enhanced_with_progress(repo_path_or_url, None)
            .await
    }

    /// Index a repository using enhanced indexer with progress reporting
    pub async fn index_repository_enhanced_with_progress<P: AsRef<Path>>(
        &mut self,
        repo_path_or_url: P,
        progress_callback: Option<Box<dyn Fn(String, f64, Option<String>) + Send + Sync>>,
    ) -> RagResult<IndexingStats> {
        if !self.is_initialized {
            return Err(RagError::Config("Pipeline not initialized".to_string()));
        }

        log_operation_start!("rag_index_repository_enhanced");
        let start_time = Instant::now();

        let path_str = repo_path_or_url.as_ref().to_string_lossy();
        eprintln!("📁 Starting enhanced repository indexing: {}", path_str);

        // Check if this is a URL or local path
        let local_path = if path_str.starts_with("http://") || path_str.starts_with("https://") {
            eprintln!("🌐 Remote repository detected, cloning...");

            // Report progress: Cloning
            if let Some(ref callback) = progress_callback {
                callback(
                    "Cloning repository".to_string(),
                    5.0,
                    Some("Downloading remote repository".to_string()),
                );
            }

            // Clone the repository
            let cloned_path = self.clone_repository(&path_str).await?;
            eprintln!("✅ Repository cloned to: {}", cloned_path);
            std::path::PathBuf::from(cloned_path)
        } else {
            repo_path_or_url.as_ref().to_path_buf()
        };

        // Report progress: Starting
        if let Some(ref callback) = progress_callback {
            callback(
                "Starting enhanced indexing".to_string(),
                0.0,
                Some("Initializing enhanced pipeline with AST-aware code splitting".to_string()),
            );
        }

        // Step 1: Create enhanced indexing pipeline
        eprintln!("🔧 Creating enhanced document indexing pipeline...");
        let enhanced_indexer = crate::create_enhanced_indexer()?;

        // Report progress: Document processing
        if let Some(ref callback) = progress_callback {
            callback(
                "Processing documents with enhanced indexer".to_string(),
                5.0,
                Some("Loading and chunking files with AST-aware splitting".to_string()),
            );
        }

        // Load documents from repository
        eprintln!("📄 Loading documents from repository...");
        let documents = self.load_repository_documents(&local_path).await?;
        let documents_count = documents.len();
        eprintln!("📚 Found {} documents to process", documents_count);

        // Report progress: Document loading complete
        if let Some(ref callback) = progress_callback {
            callback(
                "Documents loaded".to_string(),
                10.0,
                Some(format!("Found {} documents", documents_count)),
            );
        }

        // Index the documents using enhanced indexer
        eprintln!("⚙️ Enhanced indexing with AST-aware code splitting...");

        // Report progress: Starting enhanced indexing
        if let Some(ref callback) = progress_callback {
            callback(
                "Enhanced indexing in progress".to_string(),
                15.0,
                Some(format!(
                    "Processing {} documents with advanced algorithms",
                    documents_count
                )),
            );
        }

        let nodes = enhanced_indexer
            .index_documents(documents)
            .await
            .map_err(RagError::Core)?;

        eprintln!("📚 Enhanced indexing created {} nodes", nodes.len());

        // Report progress: Enhanced indexing complete
        if let Some(ref callback) = progress_callback {
            callback(
                "Enhanced indexing complete".to_string(),
                20.0,
                Some(format!(
                    "Created {} nodes with advanced parsing",
                    nodes.len()
                )),
            );
        }

        // Step 2: Generate embeddings for all nodes
        let mut embedding_generator = EmbeddingGenerator::new(self.config.embeddings.clone());
        embedding_generator.initialize().await?;

        let embedded_chunks = embedding_generator
            .generate_embeddings_with_progress(nodes, progress_callback.as_ref())
            .await?;

        let embedded_chunks_count = embedded_chunks.len();
        info!("🔢 Generated {} embeddings", embedded_chunks_count);

        // Report progress: Storing vectors
        if let Some(ref callback) = progress_callback {
            callback(
                "Storing vectors".to_string(),
                96.0,
                Some(format!(
                    "Adding {} chunks to vector store",
                    embedded_chunks.len()
                )),
            );
        }

        // Step 3: Add to vector store
        if let Some(vector_store) = &mut self.vector_store {
            vector_store.add_chunks(embedded_chunks)?;
            info!("💾 Added chunks to vector store");
        }

        // Report progress: Finalizing
        if let Some(ref callback) = progress_callback {
            callback(
                "Finalizing enhanced indexing".to_string(),
                100.0,
                Some("Enhanced repository indexing complete".to_string()),
            );
        }

        let total_time = start_time.elapsed();
        let stats = IndexingStats {
            total_documents: documents_count,
            total_chunks: embedded_chunks_count,
            indexing_time_ms: total_time.as_millis() as u64,
            total_nodes: embedded_chunks_count,
        };

        log_operation_success!(
            "rag_index_repository_enhanced",
            total_documents = stats.total_documents,
            total_chunks = stats.total_chunks,
            indexing_time_ms = stats.indexing_time_ms
        );

        info!(
            "✅ Enhanced repository indexing complete: {}",
            stats.summary()
        );
        Ok(stats)
    }

    /// Index a repository with progress reporting
    pub async fn index_repository_with_progress<P: AsRef<Path>>(
        &mut self,
        repo_path_or_url: P,
        progress_callback: Option<Box<dyn Fn(String, f64, Option<String>) + Send + Sync>>,
    ) -> RagResult<IndexingStats> {
        if !self.is_initialized {
            return Err(RagError::Config("Pipeline not initialized".to_string()));
        }

        log_operation_start!("rag_index_repository");
        let start_time = Instant::now();

        let path_str = repo_path_or_url.as_ref().to_string_lossy();
        eprintln!("📁 Starting repository indexing: {}", path_str);

        // Check if this is a URL or local path
        let local_path = if path_str.starts_with("http://") || path_str.starts_with("https://") {
            eprintln!("🌐 Remote repository detected, cloning...");

            // Report progress: Cloning
            if let Some(ref callback) = progress_callback {
                callback(
                    "Cloning repository".to_string(),
                    5.0,
                    Some("Downloading remote repository".to_string()),
                );
            }

            // Clone the repository
            let cloned_path = self.clone_repository(&path_str).await?;
            eprintln!("✅ Repository cloned to: {}", cloned_path);
            std::path::PathBuf::from(cloned_path)
        } else {
            repo_path_or_url.as_ref().to_path_buf()
        };

        // Report progress: Starting
        if let Some(ref callback) = progress_callback {
            callback(
                "Starting indexing".to_string(),
                0.0,
                Some("Initializing pipeline".to_string()),
            );
        }

        // Step 1: Run document indexing pipeline
        eprintln!("🔧 Creating document indexing pipeline...");
        let indexing_pipeline =
            crate::create_deepwiki_compatible_indexer().map_err(RagError::Core)?;

        // Report progress: Document processing
        if let Some(ref callback) = progress_callback {
            callback(
                "Processing documents".to_string(),
                5.0,
                Some("Loading and chunking files".to_string()),
            );
        }

        // Report progress: Loading documents
        if let Some(ref callback) = progress_callback {
            callback(
                "Loading documents from repository".to_string(),
                5.0,
                Some(format!("Scanning repository: {}", local_path.display())),
            );
        }

        // Load documents from repository first
        eprintln!("📄 Loading documents from repository...");
        let documents = self.load_repository_documents(&local_path).await?;
        let documents_count = documents.len();
        eprintln!("📚 Found {} documents to process", documents_count);

        // Report progress: Document loading complete
        if let Some(ref callback) = progress_callback {
            callback(
                "Documents loaded".to_string(),
                10.0,
                Some(format!("Found {} documents", documents_count)),
            );
        }

        // Index the documents
        eprintln!("⚙️ Indexing documents...");

        // Report progress: Starting indexing
        if let Some(ref callback) = progress_callback {
            callback(
                "Indexing documents".to_string(),
                15.0,
                Some(format!("Processing {} documents", documents_count)),
            );
        }

        let nodes = indexing_pipeline
            .index_documents(documents)
            .await
            .map_err(RagError::Core)?;

        eprintln!("📚 Indexed documents into {} nodes", nodes.len());

        // Report progress: Indexing complete
        if let Some(ref callback) = progress_callback {
            callback(
                "Document indexing complete".to_string(),
                20.0,
                Some(format!("Created {} nodes", nodes.len())),
            );
        }

        // Note: Embedding generation progress will be reported by the embedding generator itself

        // Step 2: Generate embeddings for all nodes
        let mut embedding_generator = EmbeddingGenerator::new(self.config.embeddings.clone());
        embedding_generator.initialize().await?;

        let embedded_chunks = embedding_generator
            .generate_embeddings_with_progress(nodes, progress_callback.as_ref())
            .await?;

        let embedded_chunks_count = embedded_chunks.len();
        info!("🔢 Generated {} embeddings", embedded_chunks_count);

        // Report progress: Storing vectors
        if let Some(ref callback) = progress_callback {
            callback(
                "Storing vectors".to_string(),
                96.0,
                Some(format!(
                    "Adding {} chunks to vector store",
                    embedded_chunks.len()
                )),
            );
        }

        // Step 3: Add to vector store
        if let Some(vector_store) = &mut self.vector_store {
            vector_store.add_chunks(embedded_chunks)?;
            info!("💾 Added chunks to vector store");
        }

        // Report progress: Finalizing
        if let Some(ref callback) = progress_callback {
            callback(
                "Finalizing".to_string(),
                95.0,
                Some("Initializing retriever".to_string()),
            );
        }

        // Step 4: Initialize retriever
        let vector_store = self.vector_store.take().unwrap();
        let mut retriever = DocumentRetriever::new(
            vector_store,
            self.config.embeddings.clone(),
            self.config.retrieval.clone(),
        );
        retriever.initialize().await?;

        // Store the vector store back (retriever owns it now)
        self.vector_store = Some(VectorStore::new(self.config.embeddings.dimension));
        self.retriever = Some(retriever);

        let indexing_time = start_time.elapsed();
        let stats = IndexingStats {
            total_documents: documents_count,
            total_nodes: embedded_chunks_count,
            total_chunks: self.retriever.as_ref().unwrap().vector_store().len(),
            indexing_time_ms: indexing_time.as_millis() as u64,
        };

        // Report progress: Complete
        if let Some(ref callback) = progress_callback {
            callback(
                "Complete".to_string(),
                100.0,
                Some(format!(
                    "Indexed {} documents into {} chunks",
                    stats.total_documents, stats.total_chunks
                )),
            );
        }

        log_operation_success!(
            "rag_index_repository",
            total_documents = stats.total_documents,
            total_chunks = stats.total_chunks,
            indexing_time_ms = stats.indexing_time_ms
        );

        info!("✅ Repository indexing complete: {}", stats.summary());
        Ok(stats)
    }

    /// Ask a question and get a RAG response
    pub async fn ask(&self, query: RagQuery) -> RagResult<RagResponse> {
        if !self.is_initialized {
            return Err(RagError::Config("Pipeline not initialized".to_string()));
        }

        let retriever = self
            .retriever
            .as_ref()
            .ok_or_else(|| RagError::Config("No documents indexed yet".to_string()))?;

        let llm_client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| RagError::Config("LLM client not initialized".to_string()))?;

        log_operation_start!("rag_ask");
        let start_time = Instant::now();

        debug!("Processing query: {}", query.question);

        // Step 1: Retrieve relevant documents
        let retrieval_start = Instant::now();
        let search_results = retriever.retrieve(&query.question).await?;
        let retrieval_time = retrieval_start.elapsed();

        info!(
            "🔍 Retrieved {} relevant chunks in {:?}",
            search_results.len(),
            retrieval_time
        );

        if search_results.is_empty() {
            warn!("No relevant documents found for query");
            return Ok(RagResponse {
                answer: "I couldn't find any relevant information in the repository to answer your question.".to_string(),
                sources: vec![],
                metadata: RagResponseMetadata {
                    chunks_retrieved: 0,
                    context_tokens: 0,
                    generation_tokens: 0,
                    retrieval_time_ms: retrieval_time.as_millis() as u64,
                    generation_time_ms: 0,
                    model_used: llm_client.model_info().summary(),
                },
            });
        }

        // Step 2: Prepare context from retrieved chunks
        let context = self.prepare_context(&search_results);
        let context_tokens = self.estimate_tokens(&context);

        debug!("Prepared context with ~{} tokens", context_tokens);

        // Step 3: Generate response using LLM
        let generation_start = Instant::now();
        let prompt = self.build_prompt(&query.question, &context, query.context.as_deref());
        let answer = llm_client
            .generate_with_system(&self.config.generation.system_prompt, &prompt)
            .await?;
        let generation_time = generation_start.elapsed();

        let generation_tokens = self.estimate_tokens(&answer);

        info!(
            "💬 Generated response in {:?} (~{} tokens)",
            generation_time, generation_tokens
        );

        let total_time = start_time.elapsed();
        let chunks_retrieved = search_results.len();
        let response = RagResponse {
            answer,
            sources: search_results,
            metadata: RagResponseMetadata {
                chunks_retrieved,
                context_tokens,
                generation_tokens,
                retrieval_time_ms: retrieval_time.as_millis() as u64,
                generation_time_ms: generation_time.as_millis() as u64,
                model_used: llm_client.model_info().summary(),
            },
        };

        log_operation_success!(
            "rag_ask",
            chunks_retrieved = response.metadata.chunks_retrieved,
            total_time_ms = total_time.as_millis() as u64
        );

        Ok(response)
    }

    /// Prepare context string from search results
    fn prepare_context(&self, search_results: &[SearchResult]) -> String {
        let mut context_parts = Vec::new();

        for (i, result) in search_results.iter().enumerate() {
            let chunk = &result.chunk;

            // Add source information if citations are enabled
            let source_info = if self.config.generation.include_citations {
                if let Some(file_path) = chunk.metadata.get("file_path").and_then(|v| v.as_str()) {
                    format!("[Source {}: {}]", i + 1, file_path)
                } else {
                    format!("[Source {}]", i + 1)
                }
            } else {
                String::new()
            };

            let content = if source_info.is_empty() {
                chunk.content.clone()
            } else {
                format!("{}\n{}", source_info, chunk.content)
            };

            context_parts.push(content);
        }

        context_parts.join("\n\n---\n\n")
    }

    /// Build the final prompt for the LLM
    fn build_prompt(
        &self,
        question: &str,
        context: &str,
        conversation_context: Option<&str>,
    ) -> String {
        let mut prompt = self
            .config
            .generation
            .user_prompt_template
            .replace("{context}", context)
            .replace("{question}", question);

        if let Some(conv_context) = conversation_context {
            prompt = format!("Previous conversation:\n{}\n\n{}", conv_context, prompt);
        }

        prompt
    }

    /// Estimate token count (rough approximation)
    fn estimate_tokens(&self, text: &str) -> usize {
        // Rough approximation: 1 token ≈ 4 characters for English text
        (text.len() as f32 / 4.0).ceil() as usize
    }

    /// Get pipeline statistics
    pub fn get_stats(&self) -> Option<PipelineStats> {
        self.retriever.as_ref().map(|retriever| PipelineStats {
            is_initialized: self.is_initialized,
            total_chunks: retriever.vector_store().len(),
            retrieval_stats: retriever.get_stats(),
            llm_model: self.llm_client.as_ref().map(|c| c.model_info().summary()),
        })
    }

    /// Update configuration
    pub fn update_config(&mut self, config: RagConfig) {
        self.config = config;
        info!("Updated RAG pipeline configuration");
    }

    /// Get current configuration
    pub fn config(&self) -> &RagConfig {
        &self.config
    }

    /// Check if pipeline is ready for queries
    pub fn is_ready(&self) -> bool {
        self.is_initialized && self.retriever.is_some() && self.llm_client.is_some()
    }

    /// Load documents from repository
    async fn load_repository_documents<P: AsRef<std::path::Path>>(
        &self,
        repo_path: P,
    ) -> RagResult<Vec<cheungfun_core::Document>> {
        use crate::DirectoryLoader;
        use cheungfun_core::traits::Loader;

        let mut documents = Vec::new();

        eprintln!("📂 Loading repository from path: {:?}", repo_path.as_ref());

        // Use cheungfun's DirectoryLoader to load documents
        let loader = DirectoryLoader::new(repo_path.as_ref().to_path_buf()).map_err(|e| {
            eprintln!("❌ Failed to create DirectoryLoader: {}", e);
            RagError::Core(Box::new(wikify_core::WikifyError::Indexing {
                message: format!("Failed to create directory loader: {}", e),
                source: None,
                context: wikify_core::ErrorContext::new("rag_pipeline")
                    .with_operation("create_loader"),
            }))
        })?;

        eprintln!("🔍 DirectoryLoader created, starting document loading...");

        let loaded_docs = loader.load().await.map_err(|e| {
            RagError::Core(Box::new(wikify_core::WikifyError::Indexing {
                message: format!("Failed to load documents: {}", e),
                source: None,
                context: wikify_core::ErrorContext::new("rag_pipeline")
                    .with_operation("load_documents"),
            }))
        })?;
        documents.extend(loaded_docs);

        info!("Loaded {} documents from repository", documents.len());
        Ok(documents)
    }

    /// Clone a remote repository to local storage
    async fn clone_repository(&self, repo_url: &str) -> RagResult<String> {
        use wikify_core::RepositoryAccessConfig;
        use wikify_repo::RepositoryProcessor;

        // Use default base path for cloned repositories
        let base_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("wikify")
            .join("repos");

        // Create processor
        let processor = RepositoryProcessor::new(&base_path);

        // Configure for Git clone mode with shallow clone
        let config = RepositoryAccessConfig {
            preferred_mode: Some(wikify_core::RepoAccessMode::GitClone),
            api_token: None,      // Force Git clone, don't use API
            force_mode: true,     // Force the preferred mode
            clone_depth: Some(1), // Shallow clone for efficiency
            custom_local_path: None,
        };

        // Access repository using processor
        let access = processor
            .access_repository(repo_url, Some(config))
            .await
            .map_err(|e| RagError::Core(e))?;

        // Return the local path
        match access.local_path {
            Some(path) => Ok(path.to_string_lossy().to_string()),
            None => Err(RagError::Config(
                "Repository access did not provide local path".to_string(),
            )),
        }
    }

    // ============================================================================
    // Deep Research Methods
    // ============================================================================

    /// Execute deep research with multiple iterations
    ///
    /// This method implements a multi-stage research process similar to DeepWiki's
    /// deep research functionality. It iteratively refines queries based on previous
    /// results to provide comprehensive answers to complex questions.
    pub async fn deep_research(
        &mut self,
        query: &str,
        config: Option<DeepResearchConfig>,
    ) -> RagResult<DeepResearchResult> {
        let research_config = config.unwrap_or_default();

        // Execute research directly in this pipeline instead of using a separate engine
        self.execute_research_internal(query, research_config).await
    }

    /// Internal implementation of deep research
    async fn execute_research_internal(
        &mut self,
        query: &str,
        config: DeepResearchConfig,
    ) -> RagResult<DeepResearchResult> {
        let research_id = uuid::Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();
        let started_at = chrono::Utc::now();

        info!("Starting deep research: {} (ID: {})", query, research_id);

        let mut iterations = Vec::new();
        let mut current_query = query.to_string();
        let mut all_sources = Vec::new();

        // Execute research iterations
        for iteration_num in 1..=config.max_iterations {
            info!(
                "Executing research iteration {}/{}",
                iteration_num, config.max_iterations
            );

            let iteration_start = std::time::Instant::now();
            let iteration_timestamp = chrono::Utc::now();

            // Build stage-specific prompt
            let stage_prompt = self.build_research_stage_prompt(
                &current_query,
                iteration_num,
                &config,
                &iterations,
            );

            // Execute RAG query
            let rag_query = RagQuery {
                question: stage_prompt,
                context: None,
                filters: None,
                retrieval_config: None,
            };

            let response = self.ask(rag_query).await?;
            let iteration_duration = iteration_start.elapsed().as_millis() as u64;

            // Create iteration record
            let iteration = crate::types::ResearchIteration {
                iteration: iteration_num,
                query: current_query.clone(),
                response: response.answer.clone(),
                sources: response.sources.clone(),
                duration_ms: iteration_duration,
                timestamp: iteration_timestamp,
                confidence_score: None, // TODO: Implement confidence scoring
            };

            // Collect unique sources
            for source in &response.sources {
                if !all_sources
                    .iter()
                    .any(|s: &crate::types::SearchResult| s.chunk.id == source.chunk.id)
                {
                    all_sources.push(source.clone());
                }
            }

            iterations.push(iteration);

            // Check if research is complete
            if self.is_research_complete(&response.answer, iteration_num, &config) {
                info!("Research completed after {} iterations", iteration_num);
                break;
            }

            // Generate follow-up query for next iteration
            if iteration_num < config.max_iterations {
                current_query = self
                    .generate_research_follow_up_query(query, &iterations)
                    .await?;

                debug!("Generated follow-up query: {}", current_query);

                // Add delay between iterations if configured
                if config.iteration_delay_ms > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        config.iteration_delay_ms,
                    ))
                    .await;
                }
            }
        }

        // Synthesize final result
        let final_synthesis = self
            .synthesize_research_final_result(query, &iterations)
            .await?;

        let total_duration = start_time.elapsed().as_millis() as u64;
        let completed_at = chrono::Utc::now();

        let result = DeepResearchResult {
            id: research_id,
            original_query: query.to_string(),
            iterations,
            final_synthesis,
            status: ResearchStatus::Completed,
            total_duration_ms: total_duration,
            started_at,
            completed_at: Some(completed_at),
            config,
            all_sources,
        };

        info!("Deep research completed in {}ms", total_duration);
        Ok(result)
    }

    /// Create a clone of this pipeline for research purposes
    /// This is a simplified approach - in production we'd use better sharing mechanisms
    fn clone_for_research(&self) -> RagResult<RagPipeline> {
        if !self.is_initialized {
            return Err(RagError::Config("Pipeline not initialized".to_string()));
        }

        // For now, we'll return a reference to self instead of cloning
        // This is a temporary solution - in production we'd use Arc<Mutex<>> or similar
        // Since we can't easily clone the complex internal state, we'll use a different approach
        Err(RagError::Config(
            "Clone for research not yet implemented - use direct pipeline reference".to_string(),
        ))
    }

    /// Generate a follow-up research query
    pub async fn generate_follow_up_query(
        &self,
        original_query: &str,
        previous_responses: &[String],
    ) -> RagResult<String> {
        let context = previous_responses.join("\n\n");

        let prompt = format!(
            "Based on the previous research findings, generate a focused follow-up question that will help deepen the investigation.\n\n\
            Original Question: {}\n\n\
            Previous Findings:\n{}\n\n\
            Generate a specific, targeted question that addresses gaps or areas needing deeper investigation. \
            Return only the follow-up question, nothing else.",
            original_query, context
        );

        let query = RagQuery {
            question: prompt,
            context: None,
            filters: None,
            retrieval_config: None,
        };

        let response = self.ask(query).await?;

        // Extract just the question from the response
        let follow_up = response
            .answer
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or(&response.answer)
            .trim()
            .to_string();

        Ok(follow_up)
    }

    /// Synthesize research findings into a comprehensive answer
    pub async fn synthesize_research_findings(
        &self,
        original_query: &str,
        all_responses: &[String],
    ) -> RagResult<String> {
        let all_findings = all_responses
            .iter()
            .enumerate()
            .map(|(i, response)| format!("## Research Iteration {}\n{}", i + 1, response))
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            "Synthesize all research findings into a comprehensive final answer.\n\n\
            Original Question: {}\n\n\
            All Research Iterations:\n{}\n\n\
            Create a well-structured, comprehensive response that:\n\
            1. Directly answers the original question\n\
            2. Integrates insights from all research iterations\n\
            3. Provides technical details and examples\n\
            4. Offers actionable conclusions\n\n\
            Structure the response with clear headings and ensure it's complete and authoritative.",
            original_query, all_findings
        );

        let query = RagQuery {
            question: prompt,
            context: None,
            filters: None,
            retrieval_config: None,
        };

        let response = self.ask(query).await?;
        Ok(response.answer)
    }

    /// Build a stage-specific prompt for research iterations
    fn build_research_stage_prompt(
        &self,
        query: &str,
        iteration: usize,
        config: &DeepResearchConfig,
        previous_iterations: &[crate::types::ResearchIteration],
    ) -> String {
        let templates = &config.prompt_templates;

        match iteration {
            1 => {
                // First iteration - research planning
                templates.first_iteration.replace("{query}", query)
            }
            i if i == config.max_iterations => {
                // Final iteration - synthesis
                let previous_findings = self.summarize_research_iterations(previous_iterations);
                templates
                    .final_iteration
                    .replace("{query}", query)
                    .replace("{previous_findings}", &previous_findings)
            }
            _ => {
                // Intermediate iteration - deep dive
                let previous_findings = self.summarize_research_iterations(previous_iterations);
                templates
                    .intermediate_iteration
                    .replace("{query}", query)
                    .replace("{iteration}", &iteration.to_string())
                    .replace("{max_iterations}", &config.max_iterations.to_string())
                    .replace("{previous_findings}", &previous_findings)
            }
        }
    }

    /// Generate a follow-up query for research
    async fn generate_research_follow_up_query(
        &mut self,
        original_query: &str,
        iterations: &[crate::types::ResearchIteration],
    ) -> RagResult<String> {
        let previous_findings = self.summarize_research_iterations(iterations);
        let current_iteration = iterations.len();

        let prompt = format!(
            "Based on the previous research findings, generate a focused follow-up question that will help deepen the investigation.\n\n\
            Original Question: {}\n\n\
            Previous Findings:\n{}\n\n\
            Current Iteration: {}\n\n\
            Generate a specific, targeted question that addresses gaps or areas needing deeper investigation. \
            The question should:\n\
            - Build upon previous findings\n\
            - Focus on specific technical aspects\n\
            - Help complete the overall research objective\n\n\
            Return only the follow-up question, nothing else.",
            original_query, previous_findings, current_iteration
        );

        let rag_query = RagQuery {
            question: prompt,
            context: None,
            filters: None,
            retrieval_config: None,
        };

        let response = self.ask(rag_query).await?;

        // Extract just the question from the response
        let follow_up = response
            .answer
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or(&response.answer)
            .trim()
            .to_string();

        Ok(follow_up)
    }

    /// Synthesize final research result
    async fn synthesize_research_final_result(
        &mut self,
        original_query: &str,
        iterations: &[crate::types::ResearchIteration],
    ) -> RagResult<String> {
        let all_findings = iterations
            .iter()
            .enumerate()
            .map(|(i, iter)| {
                format!(
                    "## Iteration {}\nQuery: {}\nFindings: {}",
                    i + 1,
                    iter.query,
                    iter.response
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            "Synthesize all research findings into a comprehensive final answer.\n\n\
            Original Question: {}\n\n\
            All Research Iterations:\n{}\n\n\
            Create a well-structured, comprehensive response that:\n\
            1. Directly answers the original question\n\
            2. Integrates insights from all research iterations\n\
            3. Provides technical details and examples\n\
            4. Offers actionable conclusions\n\n\
            Structure the response with clear headings and ensure it's complete and authoritative.",
            original_query, all_findings
        );

        let rag_query = RagQuery {
            question: prompt,
            context: None,
            filters: None,
            retrieval_config: None,
        };

        let response = self.ask(rag_query).await?;
        Ok(response.answer)
    }

    /// Check if research should be considered complete
    fn is_research_complete(
        &self,
        response: &str,
        iteration: usize,
        config: &DeepResearchConfig,
    ) -> bool {
        // Check for explicit completion indicators (inspired by DeepWiki)
        let completion_indicators = [
            "this concludes our research",
            "this completes our investigation",
            "this concludes the deep research process",
            "final conclusion",
            "in conclusion,",
            "to summarize,",
            "comprehensive analysis complete",
        ];

        let response_lower = response.to_lowercase();
        let has_completion_indicator = completion_indicators
            .iter()
            .any(|indicator| response_lower.contains(indicator));

        // Force completion at max iterations
        let at_max_iterations = iteration >= config.max_iterations;

        has_completion_indicator || at_max_iterations
    }

    /// Summarize previous research iterations
    fn summarize_research_iterations(
        &self,
        iterations: &[crate::types::ResearchIteration],
    ) -> String {
        if iterations.is_empty() {
            return "No previous findings.".to_string();
        }

        iterations
            .iter()
            .map(|iter| {
                format!(
                    "Iteration {}: {}",
                    iter.iteration,
                    iter.response.lines().take(3).collect::<Vec<_>>().join(" ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Statistics about the indexing process
#[derive(Debug, Clone)]
pub struct IndexingStats {
    pub total_documents: usize,
    pub total_nodes: usize,
    pub total_chunks: usize,
    pub indexing_time_ms: u64,
}

impl IndexingStats {
    pub fn summary(&self) -> String {
        format!(
            "{} docs → {} nodes → {} chunks ({}ms)",
            self.total_documents, self.total_nodes, self.total_chunks, self.indexing_time_ms
        )
    }
}

/// Statistics about the pipeline
#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub is_initialized: bool,
    pub total_chunks: usize,
    pub retrieval_stats: crate::retriever::RetrievalStats,
    pub llm_model: Option<String>,
}

impl PipelineStats {
    pub fn summary(&self) -> String {
        format!(
            "RAG Pipeline: {} chunks indexed, model: {}",
            self.total_chunks,
            self.llm_model.as_deref().unwrap_or("none")
        )
    }
}

/// Helper function to create a RAG pipeline with auto-detected LLM
pub async fn create_auto_rag_pipeline() -> RagResult<RagPipeline> {
    let mut config = RagConfig::default();

    // Try to auto-detect available LLM provider
    if std::env::var("OPENAI_API_KEY").is_ok() {
        config.llm = crate::llm_client::configs::openai_gpt4o_mini();
        config.embeddings.provider = "openai".to_string();
        config.embeddings.model = "text-embedding-3-small".to_string();
    } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        config.llm = crate::llm_client::configs::anthropic_claude_haiku();
        // Note: Anthropic doesn't provide embeddings, so keep OpenAI for embeddings
        // or use a local embedding model
    } else if std::env::var("GROQ_API_KEY").is_ok() {
        config.llm = crate::llm_client::configs::groq_llama3();
    } else {
        // Fallback to Ollama
        config.llm = crate::llm_client::configs::ollama_llama3(None);
    }

    let mut pipeline = RagPipeline::new(config);
    pipeline.initialize().await?;

    Ok(pipeline)
}

/// Helper function to create a simple RAG query
pub fn create_simple_query(question: &str) -> RagQuery {
    RagQuery {
        question: question.to_string(),
        context: None,
        filters: None,
        retrieval_config: None,
    }
}
