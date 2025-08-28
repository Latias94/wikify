//! Wikify RAG - Retrieval-Augmented Generation module
//!
//! This module integrates with siumai to provide RAG (Retrieval-Augmented Generation)
//! capabilities for the Wikify system, enabling intelligent question-answering
//! based on indexed repository content.

pub mod chat;
pub mod embeddings;
pub mod llm_client;
pub mod rag_pipeline;
pub mod retriever;
pub mod storage;
pub mod token_counter;
pub mod types;

pub use chat::*;
pub use embeddings::*;
pub use llm_client::*;
pub use rag_pipeline::*;
pub use retriever::*;
pub use storage::*;
pub use token_counter::*;
pub use types::*;

// Re-export commonly used types from siumai
pub use siumai::prelude::*;
