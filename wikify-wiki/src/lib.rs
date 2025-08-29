//! Wikify Wiki Generation Module
//!
//! This module provides functionality to generate comprehensive wiki documentation
//! from code repositories using RAG (Retrieval-Augmented Generation).

pub mod cache;
pub mod content_strategy;
pub mod enhanced_prompts;
pub mod export;
pub mod generator;
pub mod markdown_organizer;
pub mod priority_system;
pub mod structured_generator;
pub mod types;

// Re-export main types and functions
pub use cache::WikiCache;
pub use content_strategy::ContentGenerationStrategy;
pub use enhanced_prompts::{MarkdownPrompts, ResearchPrompts};
pub use export::{ExportFormat, WikiExporter};
pub use generator::WikiGenerator;
pub use markdown_organizer::{MarkdownOptions, MarkdownOrganizer};
pub use priority_system::{ContentPriorityAnalyzer, ContentPriorityResult};
pub use structured_generator::StructuredWikiGenerator;
pub use types::*;

/// Enhanced Wiki service with intelligent content generation
///
/// This service orchestrates wiki generation using advanced strategies inspired by DeepWiki,
/// including priority-based generation, multi-round research, and intelligent caching.
pub struct WikiService {
    generator: WikiGenerator,
    structured_generator: StructuredWikiGenerator,
    content_strategy: ContentGenerationStrategy,
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
            structured_generator: StructuredWikiGenerator::new(),
            content_strategy: ContentGenerationStrategy::new(),
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

    /// Generate a complete wiki for a repository using the original generator
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

    /// Generate wiki using intelligent strategy (DeepWiki-inspired approach)
    pub async fn generate_intelligent_wiki(
        &mut self,
        repo_path: &str,
        file_list: &[String],
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
    ) -> Result<WikiStructure, Box<dyn std::error::Error + Send + Sync>> {
        println!("🧠 Initializing intelligent wiki generation...");

        // Initialize RAG pipeline in structured generator
        let rag_config = wikify_rag::RagConfig::default();
        let mut rag_pipeline = wikify_rag::RagPipeline::new(rag_config);

        // Initialize the RAG pipeline
        rag_pipeline
            .initialize()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        println!("📊 Analyzing content priorities...");

        // Use intelligent content strategy
        let wiki = self
            .content_strategy
            .generate_content(repo_info, file_list, config, &rag_pipeline)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Cache the generated wiki
        if let Err(e) = self.cache.store_wiki(repo_path, &wiki).await {
            tracing::warn!("Failed to cache wiki: {}", e);
        }

        println!(
            "✅ Intelligent generation complete: {} pages, {} sections",
            wiki.pages.len(),
            wiki.sections.len()
        );

        // Print priority breakdown
        let critical_pages = wiki
            .pages
            .iter()
            .filter(|p| p.importance == ImportanceLevel::Critical)
            .count();
        let high_pages = wiki
            .pages
            .iter()
            .filter(|p| p.importance == ImportanceLevel::High)
            .count();
        let medium_pages = wiki
            .pages
            .iter()
            .filter(|p| p.importance == ImportanceLevel::Medium)
            .count();
        let low_pages = wiki
            .pages
            .iter()
            .filter(|p| p.importance == ImportanceLevel::Low)
            .count();

        println!(
            "📈 Priority breakdown: {} critical, {} high, {} medium, {} low",
            critical_pages, high_pages, medium_pages, low_pages
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
