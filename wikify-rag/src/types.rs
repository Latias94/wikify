//! Type definitions for the RAG system
//!
//! This module defines the core types used throughout the RAG pipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration for the RAG system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// LLM provider configuration
    pub llm: LlmConfig,
    /// Embedding configuration
    pub embeddings: EmbeddingConfig,
    /// Retrieval configuration
    pub retrieval: RetrievalConfig,
    /// Generation configuration
    pub generation: GenerationConfig,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider type (openai, anthropic, ollama, etc.)
    pub provider: String,
    /// Model name
    pub model: String,
    /// API key (optional, can be set via environment)
    pub api_key: Option<String>,
    /// Base URL for custom providers
    pub base_url: Option<String>,
    /// Temperature for generation
    pub temperature: f32,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
}

/// Embedding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Embedding provider (openai, local, etc.)
    pub provider: String,
    /// Embedding model name
    pub model: String,
    /// API key for embedding service
    pub api_key: Option<String>,
    /// Dimension of embeddings
    pub dimension: usize,
    /// Batch size for embedding generation
    pub batch_size: usize,
}

/// Retrieval configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Number of top documents to retrieve
    pub top_k: usize,
    /// Minimum similarity threshold
    pub similarity_threshold: f32,
    /// Maximum total context length
    pub max_context_length: usize,
    /// Whether to rerank results
    pub enable_reranking: bool,
}

/// Generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// System prompt template
    pub system_prompt: String,
    /// User prompt template
    pub user_prompt_template: String,
    /// Whether to include source citations
    pub include_citations: bool,
    /// Maximum response length
    pub max_response_length: Option<usize>,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfig {
                provider: "openai".to_string(),
                model: "gpt-4o-mini".to_string(),
                api_key: None,
                base_url: None,
                temperature: 0.7,
                max_tokens: Some(2000),
            },
            embeddings: EmbeddingConfig {
                provider: "openai".to_string(),
                model: "text-embedding-3-small".to_string(),
                api_key: None,
                dimension: 1536,
                batch_size: 100,
            },
            retrieval: RetrievalConfig {
                top_k: 8,
                similarity_threshold: 0.3, // Lowered for better recall
                max_context_length: 12000,
                enable_reranking: false,
            },
            generation: GenerationConfig {
                system_prompt: "You are a helpful assistant that answers questions about code repositories. Use the provided context to give accurate and helpful answers. If you cannot find the answer in the context, say so clearly.".to_string(),
                user_prompt_template: "Context:\n{context}\n\nQuestion: {question}\n\nAnswer:".to_string(),
                include_citations: true,
                max_response_length: None,
            },
        }
    }
}

/// A document chunk with embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedChunk {
    /// Unique identifier
    pub id: Uuid,
    /// Text content
    pub content: String,
    /// Embedding vector
    pub embedding: Vec<f32>,
    /// Metadata from the original document
    pub metadata: HashMap<String, serde_json::Value>,
    /// Source document ID
    pub document_id: Option<String>,
    /// Chunk index within the document
    pub chunk_index: usize,
}

/// Search result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The embedded chunk
    pub chunk: EmbeddedChunk,
    /// Similarity score (0.0 to 1.0)
    pub score: f32,
}

/// RAG query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagQuery {
    /// The user's question
    pub question: String,
    /// Optional context or conversation history
    pub context: Option<String>,
    /// Optional filters for retrieval
    pub filters: Option<HashMap<String, serde_json::Value>>,
    /// Override retrieval parameters
    pub retrieval_config: Option<RetrievalConfig>,
}

/// RAG response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    /// The generated answer
    pub answer: String,
    /// Retrieved context chunks
    pub sources: Vec<SearchResult>,
    /// Metadata about the generation
    pub metadata: RagResponseMetadata,
}

/// Metadata about the RAG response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponseMetadata {
    /// Number of chunks retrieved
    pub chunks_retrieved: usize,
    /// Total tokens used in context
    pub context_tokens: usize,
    /// Tokens used in generation
    pub generation_tokens: usize,
    /// Time taken for retrieval (ms)
    pub retrieval_time_ms: u64,
    /// Time taken for generation (ms)
    pub generation_time_ms: u64,
    /// LLM model used
    pub model_used: String,
}

/// Error types for the RAG system
#[derive(Debug, thiserror::Error)]
pub enum RagError {
    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Retrieval error: {0}")]
    Retrieval(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Core error: {0}")]
    Core(Box<wikify_core::WikifyError>),
}

impl From<wikify_core::WikifyError> for RagError {
    fn from(err: wikify_core::WikifyError) -> Self {
        RagError::Core(Box::new(err))
    }
}

pub type RagResult<T> = Result<T, RagError>;

/// Storage configuration for vector database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Base directory for storing vector databases
    pub base_dir: std::path::PathBuf,
    /// Whether to enable persistent storage
    pub enable_persistence: bool,
    /// Cache size limit in MB
    pub cache_size_mb: usize,
    /// Auto-save interval in seconds
    pub auto_save_interval_secs: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let base_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("wikify")
            .join("vector_db");

        Self {
            base_dir,
            enable_persistence: true,
            cache_size_mb: 512,           // 512MB cache
            auto_save_interval_secs: 300, // 5 minutes
        }
    }
}

/// Chat session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    /// Maximum number of messages to keep in context
    pub max_context_messages: usize,
    /// Maximum context length in tokens
    pub max_context_tokens: usize,
    /// Whether to save chat history
    pub save_history: bool,
    /// Chat history directory
    pub history_dir: std::path::PathBuf,
    /// Session timeout in minutes
    pub session_timeout_minutes: u64,
}

impl Default for ChatConfig {
    fn default() -> Self {
        let history_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("wikify")
            .join("chat_history");

        Self {
            max_context_messages: 20,
            max_context_tokens: 4000,
            save_history: true,
            history_dir,
            session_timeout_minutes: 60,
        }
    }
}

/// Chat message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message ID
    pub id: String,
    /// Role: "user" or "assistant"
    pub role: String,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    /// Session ID
    pub id: String,
    /// Repository path or URL
    pub repository: String,
    /// Messages in this session
    pub messages: Vec<ChatMessage>,
    /// Session creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last activity time
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Session metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

// ============================================================================
// Deep Research Types
// ============================================================================

/// Configuration for deep research
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchConfig {
    /// Maximum number of research iterations
    pub max_iterations: usize,
    /// Minimum confidence threshold for completion
    pub completion_threshold: f32,
    /// Whether to enable automatic iteration
    pub auto_iterate: bool,
    /// Delay between iterations (ms)
    pub iteration_delay_ms: u64,
    /// Custom prompt templates for different stages
    pub prompt_templates: ResearchPromptTemplates,
}

impl Default for DeepResearchConfig {
    fn default() -> Self {
        Self {
            max_iterations: 5,
            completion_threshold: 0.8,
            auto_iterate: true,
            iteration_delay_ms: 1000,
            prompt_templates: ResearchPromptTemplates::default(),
        }
    }
}

/// Prompt templates for different research stages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchPromptTemplates {
    /// Template for the first iteration
    pub first_iteration: String,
    /// Template for intermediate iterations
    pub intermediate_iteration: String,
    /// Template for the final iteration
    pub final_iteration: String,
    /// Template for generating follow-up queries
    pub follow_up_query: String,
    /// Template for synthesizing final results
    pub synthesis: String,
}

impl Default for ResearchPromptTemplates {
    fn default() -> Self {
        Self {
            first_iteration: r#"You are conducting a deep research investigation. This is the initial research phase.

Research Question: {query}

Create a comprehensive research plan and provide initial findings based on the available context. Structure your response with:

1. **Research Plan**: Outline your approach to investigating this topic
2. **Initial Findings**: Present what you've discovered so far
3. **Key Questions**: Identify specific areas that need further investigation

Use the provided context to give detailed, accurate information. If certain aspects need deeper investigation, mention them clearly."#.to_string(),

            intermediate_iteration: r#"Continue your deep research investigation (iteration {iteration}/{max_iterations}).

Original Question: {query}

Previous Findings Summary:
{previous_findings}

Dive deeper into specific aspects that need further investigation. Focus on:
- Addressing gaps from previous iterations
- Exploring technical details and implementation specifics
- Investigating relationships and dependencies
- Analyzing patterns and best practices

Structure your response with clear sections and build upon previous findings."#.to_string(),

            final_iteration: r#"This is the final iteration of your deep research on: {query}

Previous Research Summary:
{previous_findings}

Provide a comprehensive conclusion and synthesis. Your response should include:

1. **Executive Summary**: Key findings and insights
2. **Detailed Analysis**: Comprehensive technical details
3. **Conclusions**: Final answers to the original question
4. **Recommendations**: Actionable insights or next steps

This concludes the deep research process. Ensure your response is complete and addresses all aspects of the original question."#.to_string(),

            follow_up_query: r#"Based on the previous research findings, generate a focused follow-up question that will help deepen the investigation.

Original Question: {original_query}
Previous Findings: {previous_findings}
Current Iteration: {iteration}

Generate a specific, targeted question that addresses gaps or areas needing deeper investigation. The question should:
- Build upon previous findings
- Focus on specific technical aspects
- Help complete the overall research objective

Return only the follow-up question, nothing else."#.to_string(),

            synthesis: r#"Synthesize all research findings into a comprehensive final answer.

Original Question: {original_query}

All Research Iterations:
{all_findings}

Create a well-structured, comprehensive response that:
1. Directly answers the original question
2. Integrates insights from all research iterations
3. Provides technical details and examples
4. Offers actionable conclusions

Structure the response with clear headings and ensure it's complete and authoritative."#.to_string(),
        }
    }
}

/// Status of a deep research session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResearchStatus {
    /// Research is in progress
    InProgress,
    /// Research completed successfully
    Completed,
    /// Research was cancelled
    Cancelled,
    /// Research failed with error
    Failed(String),
}

/// A single iteration in the research process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchIteration {
    /// Iteration number (1-based)
    pub iteration: usize,
    /// Query used for this iteration
    pub query: String,
    /// Generated response
    pub response: String,
    /// Retrieved sources for this iteration
    pub sources: Vec<SearchResult>,
    /// Time taken for this iteration (ms)
    pub duration_ms: u64,
    /// Timestamp of this iteration
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Confidence score for this iteration
    pub confidence_score: Option<f32>,
}

/// Complete deep research result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchResult {
    /// Research session ID
    pub id: String,
    /// Original research question
    pub original_query: String,
    /// All research iterations
    pub iterations: Vec<ResearchIteration>,
    /// Final synthesized answer
    pub final_synthesis: String,
    /// Overall research status
    pub status: ResearchStatus,
    /// Total research time (ms)
    pub total_duration_ms: u64,
    /// Research start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Research completion time
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Research configuration used
    pub config: DeepResearchConfig,
    /// All unique sources across iterations
    pub all_sources: Vec<SearchResult>,
}

/// Progress update for ongoing research
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchProgress {
    /// Research session ID
    pub id: String,
    /// Current iteration number
    pub current_iteration: usize,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Current status
    pub status: ResearchStatus,
    /// Progress percentage (0.0 to 1.0)
    pub progress: f32,
    /// Current iteration response (if available)
    pub current_response: Option<String>,
    /// Estimated time remaining (ms)
    pub estimated_remaining_ms: Option<u64>,
    /// Last update timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}
