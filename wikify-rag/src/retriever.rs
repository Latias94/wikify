//! Document retrieval system
//!
//! This module implements the retrieval component of the RAG pipeline,
//! finding relevant document chunks based on user queries.

use crate::embeddings::{EmbeddingGenerator, VectorStore};
use crate::types::{EmbeddingConfig, RagError, RagResult, RetrievalConfig, SearchResult};
use std::time::Instant;
use tracing::{debug, info};

/// Document retriever that finds relevant chunks for queries
pub struct DocumentRetriever {
    vector_store: VectorStore,
    embedding_generator: EmbeddingGenerator,
    config: RetrievalConfig,
}

impl DocumentRetriever {
    /// Create a new document retriever
    pub fn new(
        vector_store: VectorStore,
        embedding_config: EmbeddingConfig,
        retrieval_config: RetrievalConfig,
    ) -> Self {
        let embedding_generator = EmbeddingGenerator::new(embedding_config);

        Self {
            vector_store,
            embedding_generator,
            config: retrieval_config,
        }
    }

    /// Initialize the retriever (sets up embedding client)
    pub async fn initialize(&mut self) -> RagResult<()> {
        self.embedding_generator.initialize().await?;
        info!("Document retriever initialized");
        Ok(())
    }

    /// Retrieve relevant documents for a query
    pub async fn retrieve(&self, query: &str) -> RagResult<Vec<SearchResult>> {
        let start_time = Instant::now();

        debug!("Retrieving documents for query: {}", query);

        // Generate embedding for the query
        let query_embedding = self.generate_query_embedding(query).await?;

        // Search for similar chunks
        let similar_chunks = self.vector_store.search(
            &query_embedding,
            self.config.top_k,
            self.config.similarity_threshold,
        );

        // Convert to SearchResult objects
        let mut results = Vec::new();
        for (chunk_idx, similarity_score) in similar_chunks {
            if let Some(chunk) = self.vector_store.get_chunk(chunk_idx) {
                results.push(SearchResult {
                    chunk: chunk.clone(),
                    score: similarity_score,
                });
            }
        }

        // Apply reranking if enabled
        if self.config.enable_reranking {
            results = self.rerank_results(query, results).await?;
        }

        // Filter by context length limit
        results = self.filter_by_context_length(results);

        let retrieval_time = start_time.elapsed();
        info!(
            "Retrieved {} documents in {:?} (similarity threshold: {})",
            results.len(),
            retrieval_time,
            self.config.similarity_threshold
        );

        Ok(results)
    }

    /// Generate embedding for a query
    async fn generate_query_embedding(&self, query: &str) -> RagResult<Vec<f32>> {
        // For now, we'll use a simple approach - in a real implementation,
        // you might want to preprocess the query or use a different embedding model

        // Create a temporary node-like structure for the query
        let query_node = cheungfun_core::Node {
            id: uuid::Uuid::new_v4(),
            content: query.to_string(),
            metadata: std::collections::HashMap::new(),
            embedding: None,
            sparse_embedding: None,
            relationships: cheungfun_core::relationships::NodeRelationships::new(),
            source_document_id: uuid::Uuid::new_v4(),
            chunk_info: cheungfun_core::types::ChunkInfo {
                start_char_idx: Some(0),
                end_char_idx: Some(query.len()),
                chunk_index: 0,
            },
            hash: None,
            mimetype: "text/plain".to_string(),
            excluded_embed_metadata_keys: std::collections::HashSet::new(),
            excluded_llm_metadata_keys: std::collections::HashSet::new(),
            text_template: "{content}\n\n{metadata_str}".to_string(),
            metadata_separator: "\n".to_string(),
            metadata_template: "{key}: {value}".to_string(),
        };

        let embedded_chunks = self
            .embedding_generator
            .generate_embeddings(vec![query_node])
            .await?;

        if let Some(chunk) = embedded_chunks.first() {
            Ok(chunk.embedding.clone())
        } else {
            Err(RagError::Embedding(
                "Failed to generate query embedding".to_string(),
            ))
        }
    }

    /// Rerank results using a more sophisticated method
    async fn rerank_results(
        &self,
        _query: &str,
        mut results: Vec<SearchResult>,
    ) -> RagResult<Vec<SearchResult>> {
        // Simple reranking based on content length and metadata
        // In a production system, you might use a dedicated reranking model

        results.sort_by(|a, b| {
            // Prefer longer, more substantial content
            let score_a = a.score + (a.chunk.content.len() as f32 / 10000.0).min(0.1);
            let score_b = b.score + (b.chunk.content.len() as f32 / 10000.0).min(0.1);

            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        debug!("Reranked {} results", results.len());
        Ok(results)
    }

    /// Filter results by total context length
    fn filter_by_context_length(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut filtered_results = Vec::new();
        let mut total_length = 0;

        let original_count = results.len();
        for result in results {
            let content_length = result.chunk.content.len();

            if total_length + content_length <= self.config.max_context_length {
                total_length += content_length;
                filtered_results.push(result);
            } else {
                debug!(
                    "Stopping retrieval due to context length limit ({} chars)",
                    self.config.max_context_length
                );
                break;
            }
        }

        if filtered_results.len() < original_count {
            info!(
                "Filtered results from {} to {} due to context length limit",
                original_count,
                filtered_results.len()
            );
        }

        filtered_results
    }

    /// Get retrieval statistics
    pub fn get_stats(&self) -> RetrievalStats {
        RetrievalStats {
            total_chunks: self.vector_store.len(),
            top_k: self.config.top_k,
            similarity_threshold: self.config.similarity_threshold,
            max_context_length: self.config.max_context_length,
            reranking_enabled: self.config.enable_reranking,
        }
    }

    /// Update retrieval configuration
    pub fn update_config(&mut self, config: RetrievalConfig) {
        self.config = config;
        info!("Updated retrieval configuration");
    }

    /// Get the vector store reference
    pub fn vector_store(&self) -> &VectorStore {
        &self.vector_store
    }
}

/// Statistics about the retrieval system
#[derive(Debug, Clone)]
pub struct RetrievalStats {
    pub total_chunks: usize,
    pub top_k: usize,
    pub similarity_threshold: f32,
    pub max_context_length: usize,
    pub reranking_enabled: bool,
}

impl RetrievalStats {
    /// Get a summary string
    pub fn summary(&self) -> String {
        format!(
            "Retrieval: {} chunks, top-{}, threshold={:.2}, max_context={}, rerank={}",
            self.total_chunks,
            self.top_k,
            self.similarity_threshold,
            self.max_context_length,
            self.reranking_enabled
        )
    }
}

/// Helper function to create a retriever with default settings
pub async fn create_default_retriever(
    vector_store: VectorStore,
    embedding_config: EmbeddingConfig,
) -> RagResult<DocumentRetriever> {
    let retrieval_config = RetrievalConfig {
        top_k: 8,
        similarity_threshold: 0.3, // Optimized for better recall
        max_context_length: 12000,
        enable_reranking: false,
    };

    let mut retriever = DocumentRetriever::new(vector_store, embedding_config, retrieval_config);
    retriever.initialize().await?;

    Ok(retriever)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::VectorStore;
    use crate::types::{EmbeddedChunk, EmbeddingConfig};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_chunk(content: &str, embedding: Vec<f32>) -> EmbeddedChunk {
        EmbeddedChunk {
            id: Uuid::new_v4(),
            content: content.to_string(),
            embedding,
            metadata: HashMap::new(),
            document_id: None,
            chunk_index: 0,
        }
    }

    #[test]
    fn test_filter_by_context_length() {
        let embedding_config = EmbeddingConfig {
            provider: "openai".to_string(),
            model: "text-embedding-3-small".to_string(),
            api_key: None,
            dimension: 3,
            batch_size: 10,
        };

        let retrieval_config = RetrievalConfig {
            top_k: 5,
            similarity_threshold: 0.7,
            max_context_length: 50, // Very small limit for testing
            enable_reranking: false,
        };

        let vector_store = VectorStore::new(3);
        let retriever = DocumentRetriever::new(vector_store, embedding_config, retrieval_config);

        let results = vec![
            SearchResult {
                chunk: create_test_chunk("Short text", vec![1.0, 0.0, 0.0]),
                score: 0.9,
            },
            SearchResult {
                chunk: create_test_chunk(
                    "This is a much longer text that should exceed the limit",
                    vec![0.0, 1.0, 0.0],
                ),
                score: 0.8,
            },
        ];

        let filtered = retriever.filter_by_context_length(results);
        assert_eq!(filtered.len(), 1); // Only the first short text should remain
    }
}
