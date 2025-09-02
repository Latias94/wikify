//! Document indexing and processing module
//!
//! This module provides a unified interface for document indexing with multiple
//! implementations to choose from based on your needs.
//!
//! # Architecture Overview
//!
//! The module is organized into:
//! - **traits**: Unified interfaces and configuration
//! - **legacy**: Original wikify-rag implementation (basic functionality)
//! - **enhanced**: Advanced implementation using cheungfun's features
//! - **factory**: Factory pattern for creating indexers
//! - **pipeline**: High-level processing pipelines
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use wikify_rag::indexing::{create_default_indexer, create_code_indexer};
//!
//! // Create the default recommended indexer
//! let indexer = create_default_indexer()?;
//!
//! // Or create one optimized for code
//! let code_indexer = create_code_indexer()?;
//! ```

pub mod document_processor;
pub mod enhanced;
pub mod factory;
pub mod legacy;
pub mod pipeline;
pub mod traits;

// Re-export main interfaces and factory functions
pub use factory::*;
pub use traits::*;

// Re-export sub-modules for direct access if needed
pub use document_processor::*;
pub use enhanced::*;
pub use legacy::*;

// Re-export our own pipeline types with explicit names to avoid conflicts
pub use pipeline::{
    IndexingPipeline as WikifyIndexingPipeline, PipelineConfig as WikifyPipelineConfig,
};

// Re-export commonly used types from cheungfun
pub use cheungfun_core::{Document, Node};
// Re-export specific types from cheungfun_indexing to avoid conflicts
pub use cheungfun_indexing::{
    loaders::{DirectoryLoader, FileLoader, ProgrammingLanguage},
    node_parser::{
        config::{ChunkingStrategy, CodeSplitterConfig, TextSplitterConfig},
        text::{
            CodeSplitter, MarkdownNodeParser, SemanticSplitter, SentenceSplitter, TokenTextSplitter,
        },
        FileNodeParser, MetadataAwareTextSplitter, NodeParser, TextSplitter,
    },
    pipeline::{
        indexing::PipelineBuilder, indexing::PipelineConfig as CheungfunPipelineConfig,
        DefaultIndexingPipeline,
    },
};
