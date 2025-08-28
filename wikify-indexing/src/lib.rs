//! Wikify Indexing - Document indexing and processing module
//!
//! This module integrates with cheungfun to provide document loading, parsing,
//! and indexing capabilities for the Wikify system.

pub mod document_processor;
pub mod indexer;
pub mod pipeline;

pub use document_processor::*;
pub use indexer::*;
pub use pipeline::*;

// Re-export commonly used types from cheungfun
pub use cheungfun_core::{Document, Node};
pub use cheungfun_indexing::prelude::*;
