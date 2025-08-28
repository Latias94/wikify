//! Indexing pipeline that combines document processing and indexing
//!
//! This module provides a high-level pipeline that orchestrates the entire
//! document processing and indexing workflow.

use crate::{DocumentIndexer, DocumentProcessor, IndexingConfig};
use cheungfun_core::{Document, Node};
use std::path::Path;
use tracing::info;
use wikify_core::{
    log_operation_start, log_operation_success, ErrorContext, WikifyError, WikifyResult,
};

/// Complete indexing pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Document processing configuration
    pub included_extensions: Vec<String>,
    pub excluded_dirs: Vec<String>,
    pub excluded_files: Vec<String>,
    /// Indexing configuration
    pub indexing: IndexingConfig,
    /// Processing limits
    pub max_files: Option<usize>,
    pub max_file_size_mb: Option<u64>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            included_extensions: vec![
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "java".to_string(),
                "cpp".to_string(),
                "c".to_string(),
                "h".to_string(),
                "cs".to_string(),
                "go".to_string(),
                "php".to_string(),
                "rb".to_string(),
                "md".to_string(),
                "txt".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
                "toml".to_string(),
            ],
            excluded_dirs: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
                "__pycache__".to_string(),
                ".pytest_cache".to_string(),
            ],
            excluded_files: vec![
                "*.lock".to_string(),
                "*.log".to_string(),
                "*.tmp".to_string(),
                "*.cache".to_string(),
                "*.pyc".to_string(),
                "*.so".to_string(),
                "*.dll".to_string(),
                "*.dylib".to_string(),
            ],
            indexing: IndexingConfig::default(),
            max_files: Some(10000), // Reasonable limit for large repositories
            max_file_size_mb: Some(10), // Skip very large files
        }
    }
}

/// High-level indexing pipeline
pub struct IndexingPipeline {
    config: PipelineConfig,
    processor: DocumentProcessor,
    indexer: DocumentIndexer,
}

impl IndexingPipeline {
    /// Create a new indexing pipeline with default configuration
    pub fn new<P: AsRef<Path>>(repo_path: P) -> WikifyResult<Self> {
        Self::with_config(repo_path, PipelineConfig::default())
    }

    /// Create a new indexing pipeline with custom configuration
    pub fn with_config<P: AsRef<Path>>(repo_path: P, config: PipelineConfig) -> WikifyResult<Self> {
        // Create document processor with configuration
        let processor = DocumentProcessor::new(&repo_path)
            .with_included_extensions(config.included_extensions.clone())
            .with_excluded_dirs(config.excluded_dirs.clone())
            .with_excluded_files(config.excluded_files.clone());

        // Create document indexer
        let indexer = DocumentIndexer::with_config(config.indexing.clone())?;

        Ok(Self {
            config,
            processor,
            indexer,
        })
    }

    /// Run the complete indexing pipeline
    pub async fn run(&self) -> WikifyResult<PipelineResult> {
        log_operation_start!("indexing_pipeline");

        // Step 1: Process repository documents
        info!("Step 1: Processing repository documents");
        let documents = self.processor.process_repository().await?;

        if documents.is_empty() {
            return Err(WikifyError::Indexing {
                message: "No documents found in repository".to_string(),
                source: None,
                context: ErrorContext::new("indexing_pipeline")
                    .with_operation("process_documents")
                    .with_suggestion("Check if the repository contains supported file types")
                    .with_suggestion("Verify included_extensions configuration"),
            });
        }

        // Apply file limits if configured
        let documents = self.apply_limits(documents)?;
        info!(
            "Processing {} documents after applying limits",
            documents.len()
        );

        // Step 2: Index documents into nodes
        info!("Step 2: Indexing documents into searchable nodes");
        let nodes = self.indexer.index_documents(documents.clone()).await?;

        // Step 3: Collect statistics
        let stats = PipelineStats {
            total_documents: documents.len(),
            total_nodes: nodes.len(),
            avg_nodes_per_document: if documents.is_empty() {
                0.0
            } else {
                nodes.len() as f64 / documents.len() as f64
            },
            indexing_stats: self.indexer.get_stats(),
        };

        let result = PipelineResult {
            documents,
            nodes,
            stats,
        };

        log_operation_success!(
            "indexing_pipeline",
            total_documents = result.stats.total_documents,
            total_nodes = result.stats.total_nodes
        );

        Ok(result)
    }

    /// Apply configured limits to the document list
    fn apply_limits(&self, mut documents: Vec<Document>) -> WikifyResult<Vec<Document>> {
        // Apply max_files limit
        if let Some(max_files) = self.config.max_files {
            if documents.len() > max_files {
                info!(
                    "Limiting documents to {} files (was {})",
                    max_files,
                    documents.len()
                );
                documents.truncate(max_files);
            }
        }

        // Apply max_file_size limit
        if let Some(max_size_mb) = self.config.max_file_size_mb {
            let max_size_bytes = max_size_mb * 1024 * 1024;
            let original_count = documents.len();

            documents.retain(|doc| {
                if let Some(size_value) = doc.metadata.get("file_size") {
                    if let Some(size_str) = size_value.as_str() {
                        if let Ok(size) = size_str.parse::<u64>() {
                            return size <= max_size_bytes;
                        }
                    }
                }
                true // Keep documents without size metadata
            });

            if documents.len() < original_count {
                info!(
                    "Filtered out {} large files (> {} MB)",
                    original_count - documents.len(),
                    max_size_mb
                );
            }
        }

        Ok(documents)
    }

    /// Get pipeline configuration
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }
}

/// Result of running the indexing pipeline
#[derive(Debug)]
pub struct PipelineResult {
    /// Processed documents
    pub documents: Vec<Document>,
    /// Generated nodes
    pub nodes: Vec<Node>,
    /// Pipeline statistics
    pub stats: PipelineStats,
}

/// Statistics about the pipeline execution
#[derive(Debug, Clone)]
pub struct PipelineStats {
    /// Total number of documents processed
    pub total_documents: usize,
    /// Total number of nodes generated
    pub total_nodes: usize,
    /// Average nodes per document
    pub avg_nodes_per_document: f64,
    /// Indexing statistics
    pub indexing_stats: crate::indexer::IndexingStats,
}

impl PipelineResult {
    /// Get a summary of the pipeline results
    pub fn summary(&self) -> String {
        format!(
            "Pipeline completed: {} documents → {} nodes (avg: {:.1} nodes/doc)",
            self.stats.total_documents, self.stats.total_nodes, self.stats.avg_nodes_per_document
        )
    }

    /// Get documents by file type
    pub fn documents_by_type(&self) -> std::collections::HashMap<String, Vec<&Document>> {
        let mut by_type = std::collections::HashMap::new();

        for doc in &self.documents {
            let file_type = doc
                .metadata
                .get("file_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            by_type.entry(file_type).or_insert_with(Vec::new).push(doc);
        }

        by_type
    }

    /// Get documents by programming language
    pub fn documents_by_language(&self) -> std::collections::HashMap<String, Vec<&Document>> {
        let mut by_language = std::collections::HashMap::new();

        for doc in &self.documents {
            let language = doc
                .metadata
                .get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            by_language
                .entry(language)
                .or_insert_with(Vec::new)
                .push(doc);
        }

        by_language
    }
}

/// Helper function to create a DeepWiki-compatible pipeline
pub fn create_deepwiki_compatible_pipeline<P: AsRef<Path>>(
    repo_path: P,
) -> WikifyResult<IndexingPipeline> {
    let config = PipelineConfig {
        indexing: IndexingConfig {
            chunk_size: 350,    // DeepWiki's chunk size
            chunk_overlap: 100, // DeepWiki's overlap
            sentence_aware: true,
            token_based_for_code: true,
            max_tokens_per_chunk: 250,
            preserve_markdown_structure: true,
            use_ast_code_splitting: true,
        },
        max_files: Some(10000),
        max_file_size_mb: Some(10),
        ..Default::default()
    };

    IndexingPipeline::with_config(repo_path, config)
}
