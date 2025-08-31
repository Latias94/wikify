//! Wikify RAG - Retrieval-Augmented Generation module
//!
//! This module integrates with siumai to provide RAG (Retrieval-Augmented Generation)
//! capabilities for the Wikify system, enabling intelligent question-answering
//! based on indexed repository content.

pub mod chat;
pub mod embeddings;
pub mod indexing_manager;
pub mod llm_client;
pub mod rag_pipeline;
pub mod retriever;
pub mod storage;
pub mod token_counter;
pub mod types;

pub use chat::*;
pub use embeddings::*;
pub use indexing_manager::*;
pub use rag_pipeline::*;
pub use retriever::*;
pub use storage::*;
pub use token_counter::*;

// Re-export our own types with explicit names to avoid conflicts
pub use types::{
    ChatMessage as WikifyChatMessage, LlmConfig, RagConfig, RagError, RagQuery, RagResponse,
    RagResult,
};

// Re-export commonly used types from siumai
pub use siumai::prelude::{LlmClient, Provider as LlmProvider};
