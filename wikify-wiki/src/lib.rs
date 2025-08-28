//! Wikify Wiki Generation Module
//!
//! This module provides functionality to generate comprehensive wiki documentation
//! from code repositories using RAG (Retrieval-Augmented Generation).

pub mod cache;
pub mod export;
pub mod generator;
pub mod prompts;
pub mod types;

// Re-export main types and functions
pub use cache::WikiCache;
pub use export::{ExportFormat, WikiExporter};
pub use generator::WikiGenerator;
pub use types::*;

/// Main Wiki service that orchestrates wiki generation with RAG
///
/// This is a high-level service that coordinates between the generator, cache, and exporter.
pub struct WikiService {
    generator: WikiGenerator,
    cache: WikiCache,
    exporter: WikiExporter,
}

impl WikiService {
    /// Create a new WikiService instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let generator = WikiGenerator::new()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        let cache = WikiCache::new()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        let exporter = WikiExporter::new()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(Self {
            generator,
            cache,
            exporter,
        })
    }

    /// Get cached wiki if available
    pub async fn get_cached_wiki(
        &self,
        repo_path: &str,
    ) -> Result<Option<WikiStructure>, Box<dyn std::error::Error + Send + Sync>> {
        match self.cache.get_wiki(repo_path).await {
            Ok(Some(wiki)) => Ok(Some(wiki)),
            Ok(None) => Ok(None),
            Err(e) => {
                // Log error but don't fail - just return None to indicate no cache
                tracing::warn!("Failed to get cached wiki: {}", e);
                Ok(None)
            }
        }
    }

    /// Generate a complete wiki for a repository
    pub async fn generate_wiki(
        &mut self,
        repo_path: &str,
        config: &WikiConfig,
    ) -> Result<WikiStructure, Box<dyn std::error::Error + Send + Sync>> {
        println!("🔍 Initializing wiki generator...");

        // Initialize RAG pipeline in generator
        self.generator
            .initialize_rag(config)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        println!("📝 Generating wiki structure...");

        // Use the professional generator
        let wiki = self
            .generator
            .generate_wiki(repo_path, config)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Cache the generated wiki
        if let Err(e) = self.cache.store_wiki(repo_path, &wiki).await {
            tracing::warn!("Failed to cache wiki: {}", e);
        }

        println!(
            "✅ Generated {} pages and {} sections",
            wiki.pages.len(),
            wiki.sections.len()
        );
        Ok(wiki)
    }

    /// Export wiki to various formats
    pub async fn export_wiki(
        &self,
        wiki: &WikiStructure,
        format: ExportFormat,
        output_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.exporter
            .export(wiki, format, output_path)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}
