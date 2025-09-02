//! Integration with cheungfun's DefaultIndexingPipeline
//!
//! This module provides a bridge between wikify-rag and cheungfun's complete
//! indexing pipeline system, leveraging advanced features like:
//! - Batch processing with concurrency control
//! - Type-safe transformations
//! - Built-in caching and deduplication
//! - Comprehensive error handling and statistics

use cheungfun_core::{
    deduplication::DocstoreStrategy,
    traits::{DocumentState, IndexingPipeline, NodeState, TypedTransform},
    IndexingStats, Node,
};
use cheungfun_indexing::{
    loaders::{DirectoryLoader, ProgrammingLanguage},
    node_parser::{
        config::ChunkingStrategy,
        text::{CodeSplitter, SentenceSplitter},
    },
    pipeline::{indexing::PipelineBuilder, indexing::PipelineConfig, DefaultIndexingPipeline},
};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};
use wikify_core::{ErrorContext, WikifyError, WikifyResult};

/// Configuration for cheungfun pipeline integration
#[derive(Debug, Clone)]
pub struct CheungfunPipelineConfig {
    /// Batch size for processing documents
    pub batch_size: usize,
    /// Maximum concurrency for parallel processing
    pub max_concurrency: usize,
    /// Whether to continue processing on errors
    pub continue_on_error: bool,
    /// Enable progress reporting
    pub enable_progress_reporting: bool,
    /// Enable caching for performance
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Enable document deduplication
    pub enable_deduplication: bool,
    /// Chunking strategy for different content types
    pub text_chunking_strategy: ChunkingStrategy,
    pub code_chunking_strategy: ChunkingStrategy,
    /// Chunk size and overlap
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

impl Default for CheungfunPipelineConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            max_concurrency: 4,
            continue_on_error: true,
            enable_progress_reporting: true,
            enable_caching: true,
            cache_ttl_seconds: 3600, // 1 hour
            enable_deduplication: true,
            text_chunking_strategy: ChunkingStrategy::Balanced,
            code_chunking_strategy: ChunkingStrategy::Optimal, // Uses SweepAI
            chunk_size: 350,
            chunk_overlap: 100,
        }
    }
}

impl CheungfunPipelineConfig {
    /// Create configuration optimized for code repositories
    pub fn for_code_repository() -> Self {
        let mut config = Self::default();
        config.code_chunking_strategy = ChunkingStrategy::Optimal; // SweepAI algorithm
        config.chunk_size = 400;
        config.chunk_overlap = 80;
        config.batch_size = 16; // Smaller batches for complex AST processing
        config
    }

    /// Create configuration optimized for enterprise codebases
    pub fn for_enterprise() -> Self {
        let mut config = Self::for_code_repository();
        config.code_chunking_strategy = ChunkingStrategy::Enterprise;
        config.chunk_size = 600;
        config.chunk_overlap = 120;
        config.batch_size = 8; // Even smaller batches for stability
        config.max_concurrency = 2;
        config
    }

    /// Create configuration optimized for documentation
    pub fn for_documentation() -> Self {
        let mut config = Self::default();
        config.text_chunking_strategy = ChunkingStrategy::Fine;
        config.chunk_size = 300;
        config.chunk_overlap = 75;
        config
    }
}

/// Wikify's integration with cheungfun's DefaultIndexingPipeline
pub struct WikifyCheungfunPipeline {
    pipeline: DefaultIndexingPipeline,
    config: CheungfunPipelineConfig,
}

impl WikifyCheungfunPipeline {
    /// Create a new pipeline for the given repository path
    pub async fn new<P: AsRef<Path>>(repo_path: P) -> WikifyResult<Self> {
        Self::with_config(repo_path, CheungfunPipelineConfig::default()).await
    }

    /// Create a new pipeline with custom configuration
    pub async fn with_config<P: AsRef<Path>>(
        repo_path: P,
        config: CheungfunPipelineConfig,
    ) -> WikifyResult<Self> {
        info!(
            "Creating cheungfun pipeline for repository: {:?}",
            repo_path.as_ref()
        );

        // Create directory loader
        let loader = Arc::new(
            DirectoryLoader::new(repo_path.as_ref().to_path_buf()).map_err(|e| {
                WikifyError::Indexing {
                    message: format!("Failed to create directory loader: {}", e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("cheungfun_pipeline")
                        .with_operation("create_loader"),
                }
            })?,
        );

        // Create document processors (transformers)
        let document_processors = Self::create_document_processors(&config)?;

        // Create pipeline configuration
        let pipeline_config = PipelineConfig {
            max_concurrency: config.max_concurrency,
            batch_size: config.batch_size,
            continue_on_error: config.continue_on_error,
            operation_timeout_seconds: Some(300), // 5 minutes
            enable_progress_reporting: config.enable_progress_reporting,
            enable_caching: config.enable_caching,
            cache_ttl_seconds: config.cache_ttl_seconds,
            docstore_strategy: DocstoreStrategy::Upserts,
            enable_deduplication: config.enable_deduplication,
        };

        // Build the pipeline
        let mut builder = PipelineBuilder::default()
            .with_loader(loader)
            .with_config(pipeline_config);

        // Add document processors
        for processor in document_processors {
            builder = builder.with_document_processor(processor);
        }

        let pipeline = builder.build().map_err(|e| WikifyError::Indexing {
            message: format!("Failed to build cheungfun pipeline: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("cheungfun_pipeline").with_operation("build_pipeline"),
        })?;

        Ok(Self { pipeline, config })
    }

    /// Create pipeline optimized for code repositories
    pub async fn for_code_repository<P: AsRef<Path>>(repo_path: P) -> WikifyResult<Self> {
        Self::with_config(repo_path, CheungfunPipelineConfig::for_code_repository()).await
    }

    /// Create pipeline optimized for enterprise codebases
    pub async fn for_enterprise<P: AsRef<Path>>(repo_path: P) -> WikifyResult<Self> {
        Self::with_config(repo_path, CheungfunPipelineConfig::for_enterprise()).await
    }

    /// Create pipeline optimized for documentation
    pub async fn for_documentation<P: AsRef<Path>>(repo_path: P) -> WikifyResult<Self> {
        Self::with_config(repo_path, CheungfunPipelineConfig::for_documentation()).await
    }

    /// Run the complete indexing pipeline
    pub async fn run(&self) -> WikifyResult<WikifyCheungfunResult> {
        info!("Running cheungfun indexing pipeline");

        let (nodes, stats) = IndexingPipeline::run(
            &self.pipeline,
            None, // documents (loaded by pipeline)
            None, // num_workers (use default)
            self.config.enable_progress_reporting,
            true, // store_doc_text
            None, // in_place
            true, // show_progress
        )
        .await
        .map_err(|e| WikifyError::Indexing {
            message: format!("Cheungfun pipeline execution failed: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("cheungfun_pipeline").with_operation("run_pipeline"),
        })?;

        info!(
            "Cheungfun pipeline completed: {} nodes, {} documents processed in {:?}",
            nodes.len(),
            stats.documents_processed,
            stats.processing_time
        );

        Ok(WikifyCheungfunResult { nodes, stats })
    }

    /// Create document processors based on configuration
    fn create_document_processors(
        config: &CheungfunPipelineConfig,
    ) -> WikifyResult<Vec<Arc<dyn TypedTransform<DocumentState, NodeState>>>> {
        let mut processors: Vec<Arc<dyn TypedTransform<DocumentState, NodeState>>> = Vec::new();

        // Create sentence splitter for general text
        let sentence_splitter =
            SentenceSplitter::from_defaults(config.chunk_size, config.chunk_overlap).map_err(
                |e| WikifyError::Indexing {
                    message: format!("Failed to create sentence splitter: {}", e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("cheungfun_pipeline")
                        .with_operation("create_sentence_splitter"),
                },
            )?;
        processors.push(Arc::new(sentence_splitter));

        // Create code splitters for common languages
        let common_languages = vec![
            ProgrammingLanguage::Rust,
            ProgrammingLanguage::Python,
            ProgrammingLanguage::JavaScript,
            ProgrammingLanguage::TypeScript,
            ProgrammingLanguage::Java,
            ProgrammingLanguage::Cpp,
            ProgrammingLanguage::Go,
        ];

        for lang in common_languages {
            let code_splitter = match config.code_chunking_strategy {
                ChunkingStrategy::Optimal => CodeSplitter::optimal(lang),
                ChunkingStrategy::Enterprise => CodeSplitter::enterprise(lang),
                ChunkingStrategy::Fine => CodeSplitter::fine_grained(lang),
                ChunkingStrategy::Balanced => CodeSplitter::balanced(lang),
                ChunkingStrategy::Coarse => CodeSplitter::coarse_grained(lang),
                ChunkingStrategy::Minimal => CodeSplitter::fine_grained(lang),
            };

            match code_splitter {
                Ok(splitter) => {
                    debug!(
                        "Created {:?} code splitter for {:?}",
                        config.code_chunking_strategy, lang
                    );
                    processors.push(Arc::new(splitter));
                }
                Err(e) => {
                    warn!("Failed to create code splitter for {:?}: {}", lang, e);
                    // Continue with other languages
                }
            }
        }

        Ok(processors)
    }
}

/// Result from cheungfun pipeline execution
#[derive(Debug)]
pub struct WikifyCheungfunResult {
    pub nodes: Vec<Node>,
    pub stats: IndexingStats,
}
