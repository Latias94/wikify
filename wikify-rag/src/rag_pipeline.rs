//! RAG Pipeline - Complete Retrieval-Augmented Generation system
//!
//! This module orchestrates the entire RAG workflow: indexing documents,
//! retrieving relevant context, and generating responses using LLMs.

use crate::embeddings::{EmbeddingGenerator, VectorStore};
use crate::llm_client::WikifyLlmClient;
use crate::retriever::DocumentRetriever;
use crate::types::{
    RagConfig, RagError, RagQuery, RagResponse, RagResponseMetadata, RagResult, SearchResult,
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

        info!("Initializing RAG pipeline");

        // Initialize LLM client
        self.llm_client = Some(WikifyLlmClient::new(self.config.llm.clone()).await?);
        info!("✅ LLM client initialized");

        // Test LLM connection
        if let Some(client) = &self.llm_client {
            client.test_connection().await?;
            info!("✅ LLM connection verified");
        }

        // Initialize vector store (empty for now)
        self.vector_store = Some(VectorStore::new(self.config.embeddings.dimension));
        info!("✅ Vector store initialized");

        self.is_initialized = true;

        log_operation_success!("rag_pipeline_init");
        info!("🚀 RAG pipeline initialization complete");

        Ok(())
    }

    /// Index a repository and prepare it for querying
    pub async fn index_repository<P: AsRef<Path>>(
        &mut self,
        repo_path: P,
    ) -> RagResult<IndexingStats> {
        self.index_repository_with_progress(repo_path, None).await
    }

    /// Index a repository with progress reporting
    pub async fn index_repository_with_progress<P: AsRef<Path>>(
        &mut self,
        repo_path: P,
        progress_callback: Option<Box<dyn Fn(String, f64, Option<String>) + Send + Sync>>,
    ) -> RagResult<IndexingStats> {
        if !self.is_initialized {
            return Err(RagError::Config("Pipeline not initialized".to_string()));
        }

        log_operation_start!("rag_index_repository");
        let start_time = Instant::now();

        info!("Starting repository indexing: {:?}", repo_path.as_ref());

        // Report progress: Starting
        if let Some(ref callback) = progress_callback {
            callback(
                "Starting indexing".to_string(),
                0.0,
                Some("Initializing pipeline".to_string()),
            );
        }

        // Step 1: Run document indexing pipeline
        let indexing_pipeline = wikify_indexing::create_deepwiki_compatible_pipeline(&repo_path)
            .map_err(RagError::Core)?;

        // Report progress: Document processing
        if let Some(ref callback) = progress_callback {
            callback(
                "Processing documents".to_string(),
                10.0,
                Some("Loading and chunking files".to_string()),
            );
        }

        let indexing_result = indexing_pipeline.run().await.map_err(RagError::Core)?;

        info!(
            "📚 Indexed {} documents into {} nodes",
            indexing_result.stats.total_documents, indexing_result.stats.total_nodes
        );

        // Report progress: Embedding generation
        if let Some(ref callback) = progress_callback {
            callback(
                "Generating embeddings".to_string(),
                70.0,
                Some(format!("Processing {} nodes", indexing_result.nodes.len())),
            );
        }

        // Step 2: Generate embeddings for all nodes
        let mut embedding_generator = EmbeddingGenerator::new(self.config.embeddings.clone());
        embedding_generator.initialize().await?;

        let embedded_chunks = embedding_generator
            .generate_embeddings(indexing_result.nodes)
            .await?;

        info!("🔢 Generated {} embeddings", embedded_chunks.len());

        // Report progress: Storing vectors
        if let Some(ref callback) = progress_callback {
            callback(
                "Storing vectors".to_string(),
                85.0,
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
            total_documents: indexing_result.stats.total_documents,
            total_nodes: indexing_result.stats.total_nodes,
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
        if let Some(retriever) = &self.retriever {
            Some(PipelineStats {
                is_initialized: self.is_initialized,
                total_chunks: retriever.vector_store().len(),
                retrieval_stats: retriever.get_stats(),
                llm_model: self.llm_client.as_ref().map(|c| c.model_info().summary()),
            })
        } else {
            None
        }
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
