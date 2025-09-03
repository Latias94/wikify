//! Wikify RAG - Retrieval-Augmented Generation module
//!
//! This module integrates with siumai to provide RAG (Retrieval-Augmented Generation)
//! capabilities for the Wikify system, enabling intelligent question-answering
//! based on indexed repository content.

pub mod chat;
pub mod embeddings;
pub mod indexing;
pub mod indexing_manager;
pub mod llm_client;
pub mod rag_pipeline;
pub mod retriever;
pub mod storage;
pub mod token_counter;
pub mod types;

pub use chat::*;
pub use embeddings::*;
pub use indexing::*;
pub use indexing_manager::*;
pub use rag_pipeline::*;
pub use retriever::*;
pub use storage::*;
pub use token_counter::*;

// Re-export our own types with explicit names to avoid conflicts
pub use types::{
    ChatMessage as WikifyChatMessage, DeepResearchConfig, DeepResearchResult, LlmConfig, RagConfig,
    RagError, RagQuery, RagResponse, RagResult, ResearchIteration, ResearchProgress,
    ResearchStatus,
};

// Re-export commonly used types from siumai
pub use siumai::prelude::{LlmClient, Provider as LlmProvider};

// Re-export unified indexing functions for convenience
pub use indexing::{
    create_auto_indexer, create_code_indexer, create_default_indexer, create_documentation_indexer,
    create_enterprise_indexer, create_indexer_from_string, create_indexer_with_config,
    create_legacy_indexer, IndexerType, IndexingConfig,
};

// Legacy compatibility functions
/// Create an enhanced document indexer optimized for code repositories
///
/// **Deprecated**: Use `create_code_indexer()` instead for the unified interface
#[deprecated(since = "0.2.0", note = "Use create_code_indexer() instead")]
pub fn create_enhanced_indexer() -> crate::types::RagResult<crate::indexing::DocumentIndexer> {
    create_code_indexer().map_err(|e| RagError::Core(e))
}

/// Create an enhanced indexing pipeline
pub fn create_enhanced_pipeline<P: AsRef<std::path::Path>>(
    repo_path: P,
) -> crate::types::RagResult<crate::indexing::pipeline::EnhancedIndexingPipeline> {
    crate::indexing::pipeline::EnhancedIndexingPipeline::new(repo_path)
        .map_err(|e| RagError::Core(e))
}
