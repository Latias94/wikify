//! Embedding generation and management
//!
//! This module handles the generation of embeddings for document chunks
//! using various embedding providers.

use crate::types::{EmbeddedChunk, EmbeddingConfig, RagError, RagResult};
use cheungfun_core::Node;
use indicatif::{ProgressBar, ProgressStyle};
use siumai::prelude::*;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Embedding generator that converts text chunks to vector embeddings
pub struct EmbeddingGenerator {
    config: EmbeddingConfig,
    client: Option<Box<dyn LlmClient>>,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            client: None,
        }
    }

    /// Initialize the embedding client
    pub async fn initialize(&mut self) -> RagResult<()> {
        match self.config.provider.as_str() {
            "openai" => {
                let api_key = self
                    .config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .ok_or_else(|| RagError::Config("OpenAI API key not found".to_string()))?;

                let client = LlmBuilder::new()
                    .openai()
                    .api_key(&api_key)
                    .model(&self.config.model)
                    .build()
                    .await
                    .map_err(|e| {
                        RagError::Embedding(format!("Failed to create OpenAI client: {}", e))
                    })?;

                self.client = Some(Box::new(client));

                // Log detailed configuration
                let base_url = std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

                info!(
                    "🔧 Initialized OpenAI embedding client - Provider: OpenAI, Model: {}, Endpoint: {}, Dimensions: {}, Batch Size: {}",
                    self.config.model,
                    base_url,
                    self.config.dimension,
                    self.config.batch_size
                );
            }
            provider => {
                return Err(RagError::Config(format!(
                    "Unsupported embedding provider: {}",
                    provider
                )));
            }
        }

        Ok(())
    }

    /// Generate embeddings for a batch of nodes
    pub async fn generate_embeddings(&self, nodes: Vec<Node>) -> RagResult<Vec<EmbeddedChunk>> {
        self.generate_embeddings_with_progress(nodes, None).await
    }

    /// Generate embeddings for a batch of nodes with progress reporting
    pub async fn generate_embeddings_with_progress(
        &self,
        nodes: Vec<Node>,
        progress_callback: Option<&Box<dyn Fn(String, f64, Option<String>) + Send + Sync>>,
    ) -> RagResult<Vec<EmbeddedChunk>> {
        if self.client.is_none() {
            return Err(RagError::Config(
                "Embedding client not initialized".to_string(),
            ));
        }

        info!(
            "🚀 Starting embedding generation - Provider: {}, Model: {}, Nodes: {}, Batch Size: {}",
            self.config.provider,
            self.config.model,
            nodes.len(),
            self.config.batch_size
        );

        // Create progress bar
        let pb = ProgressBar::new(nodes.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );
        pb.set_message("Generating embeddings");

        let mut embedded_chunks = Vec::new();
        let mut processed_count = 0;
        let total_batches = nodes.len().div_ceil(self.config.batch_size);
        let start_time = std::time::Instant::now();

        // Process nodes in batches to avoid rate limits
        for (batch_index, batch) in nodes.chunks(self.config.batch_size).enumerate() {
            let batch_start = std::time::Instant::now();

            info!(
                "📦 Processing batch {}/{} - Provider: {}, Model: {}, Batch size: {}",
                batch_index + 1,
                total_batches,
                self.config.provider,
                self.config.model,
                batch.len()
            );

            let batch_chunks = self
                .process_batch_with_progress(batch, &pb, &mut processed_count, nodes.len(), progress_callback.as_ref())
                .await?;

            let batch_duration = batch_start.elapsed();
            info!(
                "✅ Batch {}/{} completed - Generated: {} embeddings, Duration: {:?}, Rate: {:.2} embeddings/sec",
                batch_index + 1,
                total_batches,
                batch_chunks.len(),
                batch_duration,
                batch_chunks.len() as f64 / batch_duration.as_secs_f64()
            );

            embedded_chunks.extend(batch_chunks);

            // Progress is now reported at the individual node level to match console progress

            // Small delay to avoid rate limits
            if processed_count < nodes.len() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        let total_duration = start_time.elapsed();
        pb.finish_with_message("✅ Embeddings generated");

        // Report final completion progress
        if let Some(callback) = progress_callback {
            callback(
                "Generating embeddings".to_string(),
                95.0,
                Some(format!(
                    "Completed {}/{} nodes",
                    processed_count,
                    nodes.len()
                )),
            );
        }

        info!(
            "🎉 Embedding generation completed - Provider: {}, Model: {}, Total: {} embeddings, Duration: {:?}, Average rate: {:.2} embeddings/sec",
            self.config.provider,
            self.config.model,
            embedded_chunks.len(),
            total_duration,
            embedded_chunks.len() as f64 / total_duration.as_secs_f64()
        );
        Ok(embedded_chunks)
    }

    /// Process a batch of nodes with progress tracking
    async fn process_batch_with_progress(
        &self,
        nodes: &[Node],
        pb: &ProgressBar,
        processed_count: &mut usize,
        total_nodes: usize,
        progress_callback: Option<&Box<dyn Fn(String, f64, Option<String>) + Send + Sync>>,
    ) -> RagResult<Vec<EmbeddedChunk>> {
        let client = self.client.as_ref().unwrap();
        let mut embedded_chunks = Vec::new();

        for (index, node) in nodes.iter().enumerate() {
            // Extract text content from node
            let content = node.content.clone();
            if content.trim().is_empty() {
                debug!("Skipping empty node");
                *processed_count += 1;
                pb.set_position(*processed_count as u64);
                continue;
            }

            // Generate embedding for this chunk
            match self
                .generate_single_embedding(client.as_ref(), &content)
                .await
            {
                Ok(embedding) => {
                    let embedded_chunk = EmbeddedChunk {
                        id: Uuid::new_v4(),
                        content: content.clone(),
                        embedding,
                        metadata: node.metadata.clone(),
                        document_id: Some(node.id.to_string()),
                        chunk_index: index,
                    };
                    embedded_chunks.push(embedded_chunk);
                }
                Err(e) => {
                    warn!("Failed to generate embedding for node {}: {}", node.id, e);
                    // Continue processing other nodes
                }
            }

            *processed_count += 1;
            pb.set_position(*processed_count as u64);

            // Report fine-grained progress after each node (only every 5 nodes to reduce WebSocket traffic)
            if let Some(callback) = progress_callback {
                if *processed_count % 5 == 0 {
                    let embedding_progress = *processed_count as f64 / total_nodes as f64;
                    let percentage = 20.0 + embedding_progress * 75.0; // 20% to 95%
                    callback(
                        "Generating embeddings".to_string(),
                        percentage,
                        Some(format!(
                            "Processing {}/{} nodes",
                            *processed_count,
                            total_nodes
                        )),
                    );
                }
            }
        }

        Ok(embedded_chunks)
    }

    /// Process a batch of nodes
    async fn process_batch(&self, nodes: &[Node]) -> RagResult<Vec<EmbeddedChunk>> {
        let client = self.client.as_ref().unwrap();
        let mut embedded_chunks = Vec::new();

        for (index, node) in nodes.iter().enumerate() {
            // Extract text content from node
            let content = node.content.clone();
            if content.trim().is_empty() {
                debug!("Skipping empty node");
                continue;
            }

            // Generate embedding for this chunk
            match self
                .generate_single_embedding(client.as_ref(), &content)
                .await
            {
                Ok(embedding) => {
                    let embedded_chunk = EmbeddedChunk {
                        id: Uuid::new_v4(),
                        content: content.clone(),
                        embedding,
                        metadata: node.metadata.clone(),
                        document_id: Some(node.id.to_string()),
                        chunk_index: index,
                    };
                    embedded_chunks.push(embedded_chunk);
                }
                Err(e) => {
                    warn!("Failed to generate embedding for chunk {}: {}", index, e);
                    // Continue processing other chunks instead of failing completely
                }
            }
        }

        Ok(embedded_chunks)
    }

    /// Generate embedding for a single text
    async fn generate_single_embedding(
        &self,
        client: &dyn LlmClient,
        text: &str,
    ) -> RagResult<Vec<f32>> {
        if let Some(embedding_client) = client.as_embedding_capability() {
            let start_time = std::time::Instant::now();

            debug!(
                "📡 Calling embedding API - Provider: {}, Model: {}, Text length: {} chars",
                self.config.provider,
                self.config.model,
                text.len()
            );

            let response = embedding_client
                .embed(vec![text.to_string()])
                .await
                .map_err(|e| {
                    error!(
                        "❌ Embedding API call failed - Provider: {}, Model: {}, Error: {}",
                        self.config.provider, self.config.model, e
                    );
                    RagError::Embedding(format!("Embedding API call failed: {}", e))
                })?;

            let duration = start_time.elapsed();

            if let Some(embedding) = response.embeddings.first() {
                debug!(
                    "✅ Embedding generated - Provider: {}, Model: {}, Dimension: {}, Duration: {:?}",
                    self.config.provider,
                    self.config.model,
                    embedding.len(),
                    duration
                );
                Ok(embedding.clone())
            } else {
                error!(
                    "❌ No embedding data returned - Provider: {}, Model: {}",
                    self.config.provider, self.config.model
                );
                Err(RagError::Embedding(
                    "No embedding data returned".to_string(),
                ))
            }
        } else {
            error!(
                "❌ Provider does not support embeddings - Provider: {}",
                self.config.provider
            );
            Err(RagError::Config(format!(
                "Provider {} does not support embeddings",
                self.config.provider
            )))
        }
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        self.config.dimension
    }

    /// Get configuration
    pub fn config(&self) -> &EmbeddingConfig {
        &self.config
    }
}

/// Simple in-memory vector store for embeddings
pub struct VectorStore {
    chunks: Vec<EmbeddedChunk>,
    dimension: usize,
}

impl VectorStore {
    /// Create a new vector store
    pub fn new(dimension: usize) -> Self {
        Self {
            chunks: Vec::new(),
            dimension,
        }
    }

    /// Add embedded chunks to the store
    pub fn add_chunks(&mut self, chunks: Vec<EmbeddedChunk>) -> RagResult<()> {
        let chunks_len = chunks.len();
        for chunk in chunks {
            if chunk.embedding.len() != self.dimension {
                return Err(RagError::Config(format!(
                    "Embedding dimension mismatch: expected {}, got {}",
                    self.dimension,
                    chunk.embedding.len()
                )));
            }
            self.chunks.push(chunk);
        }

        info!(
            "Added {} chunks to vector store (total: {})",
            chunks_len,
            self.chunks.len()
        );
        Ok(())
    }

    /// Search for similar chunks using cosine similarity
    pub fn search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        threshold: f32,
    ) -> Vec<(usize, f32)> {
        if query_embedding.len() != self.dimension {
            warn!(
                "Query embedding dimension mismatch: expected {}, got {}",
                self.dimension,
                query_embedding.len()
            );
            return Vec::new();
        }

        let mut similarities: Vec<(usize, f32)> = self
            .chunks
            .iter()
            .enumerate()
            .map(|(idx, chunk)| {
                let similarity = cosine_similarity(query_embedding, &chunk.embedding);
                (idx, similarity)
            })
            .filter(|(_, similarity)| *similarity >= threshold)
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k results
        similarities.truncate(top_k);
        similarities
    }

    /// Get chunk by index
    pub fn get_chunk(&self, index: usize) -> Option<&EmbeddedChunk> {
        self.chunks.get(index)
    }

    /// Get all chunks
    pub fn chunks(&self) -> &[EmbeddedChunk] {
        &self.chunks
    }

    /// Get number of chunks
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_vector_store() {
        let mut store = VectorStore::new(3);

        let chunk = EmbeddedChunk {
            id: Uuid::new_v4(),
            content: "test content".to_string(),
            embedding: vec![1.0, 0.0, 0.0],
            metadata: HashMap::new(),
            document_id: None,
            chunk_index: 0,
        };

        store.add_chunks(vec![chunk]).unwrap();
        assert_eq!(store.len(), 1);

        let results = store.search(&[1.0, 0.0, 0.0], 1, 0.5);
        assert_eq!(results.len(), 1);
        assert!((results[0].1 - 1.0).abs() < 1e-6);
    }
}
