//! Wikify Indexing - Document indexing and processing module
//!
//! This module integrates with cheungfun to provide document loading, parsing,
//! and indexing capabilities for the Wikify system.

pub mod document_processor;
pub mod indexer;
pub mod pipeline;

pub use document_processor::*;
pub use indexer::*;

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
        text::{CodeSplitter, MarkdownNodeParser, SentenceSplitter, TokenTextSplitter},
        NodeParser,
    },
};
