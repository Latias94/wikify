//! Unified indexing traits and interfaces
//!
//! This module defines the common interfaces for document indexing,
//! allowing different implementations (legacy and enhanced) to be
//! used interchangeably.

use async_trait::async_trait;
use cheungfun_core::{Document, Node};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wikify_core::WikifyResult;

/// Unified configuration for document indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Base chunk configuration
    pub chunk_size: usize,
    pub chunk_overlap: usize,

    /// Processing options
    pub batch_size: usize,
    pub max_concurrency: usize,
    pub continue_on_error: bool,

    /// Feature flags
    pub enable_ast_code_splitting: bool,
    pub preserve_markdown_structure: bool,
    pub enable_semantic_splitting: bool,

    /// Implementation-specific settings
    pub implementation_settings: HashMap<String, serde_json::Value>,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 350,
            chunk_overlap: 100,
            batch_size: 32,
            max_concurrency: 4,
            continue_on_error: true,
            enable_ast_code_splitting: true,
            preserve_markdown_structure: true,
            enable_semantic_splitting: false,
            implementation_settings: HashMap::new(),
        }
    }
}

impl IndexingConfig {
    /// Create configuration optimized for code repositories
    pub fn for_code_repository() -> Self {
        let mut config = Self::default();
        config.chunk_size = 400;
        config.chunk_overlap = 80;
        config.enable_ast_code_splitting = true;
        config
    }

    /// Create configuration optimized for enterprise codebases
    pub fn for_enterprise() -> Self {
        let mut config = Self::for_code_repository();
        config.chunk_size = 600;
        config.chunk_overlap = 120;
        config.batch_size = 16;
        config.max_concurrency = 2;
        config
    }

    /// Create configuration optimized for documentation
    pub fn for_documentation() -> Self {
        let mut config = Self::default();
        config.chunk_size = 300;
        config.chunk_overlap = 75;
        config.preserve_markdown_structure = true;
        config
    }
}

/// Indexing statistics with detailed breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingStats {
    pub total_documents: usize,
    pub total_nodes: usize,
    pub processing_time_ms: u128,
    pub avg_nodes_per_document: f64,
    pub implementation_used: String,
    pub chunking_strategies: Vec<String>,
    pub languages_processed: Vec<String>,
    pub errors: Vec<String>,
}

impl IndexingStats {
    pub fn new(implementation: &str) -> Self {
        Self {
            total_documents: 0,
            total_nodes: 0,
            processing_time_ms: 0,
            avg_nodes_per_document: 0.0,
            implementation_used: implementation.to_string(),
            chunking_strategies: Vec::new(),
            languages_processed: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "{} implementation: {} nodes from {} documents in {}ms (avg: {:.1} nodes/doc)",
            self.implementation_used,
            self.total_nodes,
            self.total_documents,
            self.processing_time_ms,
            self.avg_nodes_per_document
        )
    }
}

/// Unified document indexer enum that wraps different implementations
///
/// This enum provides a type-safe way to work with different indexing implementations
/// while maintaining a unified interface.
#[derive(Debug)]
pub enum DocumentIndexer {
    Legacy(crate::indexing::legacy::LegacyDocumentIndexer),
    Enhanced(crate::indexing::enhanced::EnhancedDocumentIndexer),
}

impl DocumentIndexer {
    /// Get the implementation name
    pub fn implementation_name(&self) -> &'static str {
        match self {
            DocumentIndexer::Legacy(_) => "legacy",
            DocumentIndexer::Enhanced(_) => "enhanced",
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &IndexingConfig {
        match self {
            DocumentIndexer::Legacy(indexer) => indexer.config(),
            DocumentIndexer::Enhanced(indexer) => indexer.config(),
        }
    }

    /// Index a batch of documents
    pub async fn index_documents(&self, documents: Vec<Document>) -> WikifyResult<Vec<Node>> {
        match self {
            DocumentIndexer::Legacy(indexer) => indexer.index_documents(documents).await,
            DocumentIndexer::Enhanced(indexer) => indexer.index_documents(documents).await,
        }
    }

    /// Index a single document
    pub async fn index_document(&self, document: Document) -> WikifyResult<Vec<Node>> {
        self.index_documents(vec![document]).await
    }

    /// Get indexing statistics
    pub fn get_stats(&self) -> IndexingStats {
        match self {
            DocumentIndexer::Legacy(indexer) => {
                // Convert legacy stats to unified stats
                let legacy_stats = indexer.get_stats();
                IndexingStats {
                    total_documents: 0,          // Legacy doesn't track this
                    total_nodes: 0,              // Legacy doesn't track this
                    processing_time_ms: 0,       // Legacy doesn't track this
                    avg_nodes_per_document: 0.0, // Legacy doesn't track this
                    implementation_used: "legacy".to_string(),
                    chunking_strategies: vec!["sentence".to_string(), "token".to_string()],
                    languages_processed: Vec::new(), // Legacy doesn't track this
                    errors: Vec::new(),              // Legacy doesn't track this
                }
            }
            DocumentIndexer::Enhanced(indexer) => indexer.get_stats(),
        }
    }

    /// Check if a feature is supported by this implementation
    pub fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "ast_code_splitting" => self.config().enable_ast_code_splitting,
            "semantic_splitting" => self.config().enable_semantic_splitting,
            "markdown_structure" => self.config().preserve_markdown_structure,
            "batch_processing" => true, // All implementations support this
            _ => false,
        }
    }

    /// Get supported programming languages for code splitting
    pub fn supported_languages(&self) -> Vec<String> {
        match self {
            DocumentIndexer::Legacy(indexer) => indexer.supported_languages(),
            DocumentIndexer::Enhanced(indexer) => indexer.supported_languages(),
        }
    }
}

/// Internal trait for indexer implementations
///
/// This trait is used internally by the enum wrapper and should not be used directly.
#[async_trait]
pub trait DocumentIndexerImpl: Send + Sync {
    /// Get the current configuration
    fn config(&self) -> &IndexingConfig;

    /// Index a batch of documents
    async fn index_documents(&self, documents: Vec<Document>) -> WikifyResult<Vec<Node>>;

    /// Get indexing statistics
    fn get_stats(&self) -> IndexingStats;

    /// Get supported programming languages for code splitting
    fn supported_languages(&self) -> Vec<String>;
}

/// Indexer implementation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexerType {
    /// Legacy implementation (original wikify-rag indexer)
    Legacy,
    /// Enhanced implementation (using cheungfun's advanced features)
    Enhanced,
    /// Cheungfun pipeline implementation (full cheungfun integration)
    CheungfunPipeline,
}

impl IndexerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexerType::Legacy => "legacy",
            IndexerType::Enhanced => "enhanced",
            IndexerType::CheungfunPipeline => "cheungfun_pipeline",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "legacy" => Some(IndexerType::Legacy),
            "enhanced" => Some(IndexerType::Enhanced),
            "cheungfun_pipeline" | "cheungfun" | "pipeline" => Some(IndexerType::CheungfunPipeline),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            IndexerType::Legacy => "Original wikify-rag indexer with basic functionality",
            IndexerType::Enhanced => {
                "Enhanced indexer with AST-aware code splitting and advanced features"
            }
            IndexerType::CheungfunPipeline => {
                "Full cheungfun pipeline integration with maximum features"
            }
        }
    }
}

/// Factory trait for creating indexers
pub trait IndexerFactory {
    /// Create an indexer of the specified type with default configuration
    fn create_indexer(indexer_type: IndexerType) -> WikifyResult<DocumentIndexer>;

    /// Create an indexer with custom configuration
    fn create_indexer_with_config(
        indexer_type: IndexerType,
        config: IndexingConfig,
    ) -> WikifyResult<DocumentIndexer>;

    /// Get available indexer types
    fn available_types() -> Vec<IndexerType> {
        vec![
            IndexerType::Legacy,
            IndexerType::Enhanced,
            IndexerType::CheungfunPipeline,
        ]
    }

    /// Get recommended indexer type for a use case
    fn recommended_for_use_case(use_case: &str) -> IndexerType {
        match use_case.to_lowercase().as_str() {
            "code" | "code_repository" | "programming" => IndexerType::Enhanced,
            "enterprise" | "large_codebase" | "production" => IndexerType::CheungfunPipeline,
            "documentation" | "docs" | "markdown" => IndexerType::Enhanced,
            "simple" | "basic" | "legacy" => IndexerType::Legacy,
            _ => IndexerType::Enhanced, // Default to enhanced
        }
    }
}

/// Error types specific to indexing operations
#[derive(Debug, thiserror::Error)]
pub enum IndexingError {
    #[error("Unsupported indexer type: {0}")]
    UnsupportedIndexerType(String),

    #[error("Feature not supported by {implementation}: {feature}")]
    FeatureNotSupported {
        implementation: String,
        feature: String,
    },

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Indexing failed: {0}")]
    IndexingFailed(String),
}
