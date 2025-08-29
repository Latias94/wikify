//! Intelligent content generation strategy
//!
//! This module implements sophisticated content generation strategies
//! inspired by DeepWiki's multi-round research and iterative improvement approach.

use crate::enhanced_prompts::MarkdownPrompts;
use crate::priority_system::{ContentPriorityAnalyzer, ContentPriorityResult, PageTemplate};
use crate::types::{ImportanceLevel, RepositoryInfo, WikiConfig, WikiPage, WikiStructure};
use chrono::Utc;
use std::collections::HashMap;
use tracing::{info, warn};
use wikify_core::{ErrorContext, WikifyError, WikifyResult};
use wikify_rag::RagPipeline;

/// Content generation strategy that implements intelligent, priority-based generation
pub struct ContentGenerationStrategy {
    priority_analyzer: ContentPriorityAnalyzer,
    generation_stats: GenerationStats,
}

impl ContentGenerationStrategy {
    /// Create a new content generation strategy
    pub fn new() -> Self {
        Self {
            priority_analyzer: ContentPriorityAnalyzer::new(),
            generation_stats: GenerationStats::new(),
        }
    }

    /// Generate wiki content using intelligent strategy
    pub async fn generate_content(
        &mut self,
        repo_info: &RepositoryInfo,
        file_list: &[String],
        config: &WikiConfig,
        rag_pipeline: &RagPipeline,
    ) -> WikifyResult<WikiStructure> {
        info!(
            "Starting intelligent content generation for {}",
            repo_info.title
        );

        // Step 1: Analyze content priorities
        let priority_result = self
            .priority_analyzer
            .analyze_priorities(repo_info, file_list);
        info!(
            "Priority analysis complete. Estimated {} tokens needed",
            priority_result.estimate_total_tokens()
        );

        // Step 2: Create wiki structure
        let mut wiki_structure = WikiStructure::new(
            repo_info.title.clone(),
            repo_info.description.clone(),
            repo_info.title.clone(),
        );

        // Step 3: Generate content by priority (Critical -> High -> Medium -> Low)
        self.generate_critical_content(
            &mut wiki_structure,
            &priority_result,
            repo_info,
            config,
            rag_pipeline,
        )
        .await?;
        self.generate_high_priority_content(
            &mut wiki_structure,
            &priority_result,
            repo_info,
            config,
            rag_pipeline,
        )
        .await?;

        // Only generate medium/low priority if we haven't hit limits
        if !self.should_stop_generation(config) {
            self.generate_medium_priority_content(
                &mut wiki_structure,
                &priority_result,
                repo_info,
                config,
                rag_pipeline,
            )
            .await?;
        }

        // Step 4: Post-process and optimize
        self.post_process_wiki(&mut wiki_structure, config).await?;

        info!(
            "Content generation complete: {} pages, {} sections",
            wiki_structure.pages.len(),
            wiki_structure.sections.len()
        );

        Ok(wiki_structure)
    }

    /// Generate critical content (must-have pages)
    async fn generate_critical_content(
        &mut self,
        wiki_structure: &mut WikiStructure,
        priority_result: &ContentPriorityResult,
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
        rag_pipeline: &RagPipeline,
    ) -> WikifyResult<()> {
        info!(
            "Generating critical content ({} pages)",
            priority_result.critical_pages.len()
        );

        for page_template in &priority_result.critical_pages {
            let page = self
                .generate_page_from_template(
                    page_template,
                    repo_info,
                    config,
                    rag_pipeline,
                    &priority_result.file_priorities,
                )
                .await?;

            wiki_structure.pages.push(page);
            self.generation_stats.pages_generated += 1;
            self.generation_stats.tokens_used += page_template.estimated_tokens;
        }

        Ok(())
    }

    /// Generate high priority content
    async fn generate_high_priority_content(
        &mut self,
        wiki_structure: &mut WikiStructure,
        priority_result: &ContentPriorityResult,
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
        rag_pipeline: &RagPipeline,
    ) -> WikifyResult<()> {
        info!(
            "Generating high priority content ({} pages)",
            priority_result.high_priority_pages.len()
        );

        for page_template in &priority_result.high_priority_pages {
            if self.should_stop_generation(config) {
                warn!("Stopping generation due to limits");
                break;
            }

            let page = self
                .generate_page_from_template(
                    page_template,
                    repo_info,
                    config,
                    rag_pipeline,
                    &priority_result.file_priorities,
                )
                .await?;

            wiki_structure.pages.push(page);
            self.generation_stats.pages_generated += 1;
            self.generation_stats.tokens_used += page_template.estimated_tokens;
        }

        Ok(())
    }

    /// Generate medium priority content
    async fn generate_medium_priority_content(
        &mut self,
        wiki_structure: &mut WikiStructure,
        priority_result: &ContentPriorityResult,
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
        rag_pipeline: &RagPipeline,
    ) -> WikifyResult<()> {
        info!(
            "Generating medium priority content ({} pages)",
            priority_result.medium_priority_pages.len()
        );

        for page_template in &priority_result.medium_priority_pages {
            if self.should_stop_generation(config) {
                warn!("Stopping generation due to limits");
                break;
            }

            let page = self
                .generate_page_from_template(
                    page_template,
                    repo_info,
                    config,
                    rag_pipeline,
                    &priority_result.file_priorities,
                )
                .await?;

            wiki_structure.pages.push(page);
            self.generation_stats.pages_generated += 1;
            self.generation_stats.tokens_used += page_template.estimated_tokens;
        }

        Ok(())
    }

    /// Generate a wiki page from a template
    async fn generate_page_from_template(
        &self,
        template: &PageTemplate,
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
        rag_pipeline: &RagPipeline,
        file_priorities: &HashMap<String, f64>,
    ) -> WikifyResult<WikiPage> {
        info!("Generating page: {}", template.title);

        // Create the page
        let mut page = WikiPage::new(
            template.id.clone(),
            template.title.clone(),
            template.description.clone(),
        );
        page.importance = template.importance.clone();

        // Find relevant files for this page
        let relevant_files = self.find_relevant_files(&template.id, file_priorities);
        page.file_paths = relevant_files.clone();

        // Generate content using multi-round approach
        let content = self
            .generate_page_content_multi_round(
                template,
                &relevant_files,
                repo_info,
                config,
                rag_pipeline,
            )
            .await?;

        page.content = content;
        page.estimate_reading_time();

        Ok(page)
    }

    /// Generate page content using multi-round research approach
    async fn generate_page_content_multi_round(
        &self,
        template: &PageTemplate,
        relevant_files: &[String],
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
        rag_pipeline: &RagPipeline,
    ) -> WikifyResult<String> {
        // Round 1: Direct markdown generation using enhanced prompts
        let wiki_page = WikiPage {
            id: template.title.to_lowercase().replace(' ', "-"),
            title: template.title.clone(),
            content: String::new(),
            description: template.description.clone(),
            importance: template.importance.clone(),
            file_paths: relevant_files.to_vec(),
            related_pages: Vec::new(),
            parent_section: None,
            tags: Vec::new(),
            reading_time: 0,
            generated_at: Utc::now(),
            source_documents: Vec::new(),
        };

        let initial_prompt = MarkdownPrompts::create_direct_markdown_prompt(
            &wiki_page,
            relevant_files,
            repo_info,
            config,
        );
        let initial_query = wikify_rag::create_simple_query(&initial_prompt);

        let initial_response =
            rag_pipeline
                .ask(initial_query)
                .await
                .map_err(|e| WikifyError::Rag {
                    message: format!(
                        "Failed to generate initial content for {}: {}",
                        template.title, e
                    ),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("content_strategy"),
                })?;

        // For critical and high importance pages, do a refinement round
        if matches!(
            template.importance,
            ImportanceLevel::Critical | ImportanceLevel::High
        ) {
            let refinement_prompt = format!(
                r#"Review and enhance the following markdown documentation for "{}":

CURRENT CONTENT:
{}

ENHANCEMENT REQUIREMENTS:
- Add missing technical details and examples
- Improve clarity and structure
- Add troubleshooting tips if relevant
- Ensure completeness for {} importance level
- Maintain proper markdown formatting

Provide additional content or improvements that would make this documentation more comprehensive and useful."#,
                template.title,
                initial_response.answer,
                template.importance.as_str()
            );

            let refinement_query = wikify_rag::create_simple_query(&refinement_prompt);

            let refinement_response =
                rag_pipeline
                    .ask(refinement_query)
                    .await
                    .map_err(|e| WikifyError::Rag {
                        message: format!("Failed to refine content for {}: {}", template.title, e),
                        source: Some(Box::new(e)),
                        context: ErrorContext::new("content_strategy"),
                    })?;

            // Combine initial and refined content
            Ok(format!(
                "{}\n\n---\n\n## 🔄 Enhanced Content\n\n{}",
                initial_response.answer, refinement_response.answer
            ))
        } else {
            Ok(initial_response.answer)
        }
    }

    /// Find files relevant to a specific page
    fn find_relevant_files(
        &self,
        _page_id: &str,
        file_priorities: &HashMap<String, f64>,
    ) -> Vec<String> {
        let mut relevant_files: Vec<_> = file_priorities
            .iter()
            .filter(|(_, &score)| score >= 0.5) // Only include moderately important files
            .map(|(path, score)| (path.clone(), *score))
            .collect();

        // Sort by importance
        relevant_files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top 10 files
        relevant_files
            .into_iter()
            .take(10)
            .map(|(path, _)| path)
            .collect()
    }

    /// Check if generation should stop due to limits
    fn should_stop_generation(&self, config: &WikiConfig) -> bool {
        if let Some(max_pages) = config.max_pages {
            if self.generation_stats.pages_generated >= max_pages {
                return true;
            }
        }
        false
    }

    /// Post-process the wiki structure
    async fn post_process_wiki(
        &self,
        wiki_structure: &mut WikiStructure,
        _config: &WikiConfig,
    ) -> WikifyResult<()> {
        info!("Post-processing wiki structure");

        // Update metadata
        wiki_structure.metadata.stats.total_pages = wiki_structure.pages.len();
        wiki_structure.metadata.stats.total_tokens_used = self.generation_stats.tokens_used;

        // Sort pages by importance
        wiki_structure
            .pages
            .sort_by(|a, b| b.importance.cmp(&a.importance));

        Ok(())
    }
}

/// Statistics for content generation
#[derive(Debug, Clone)]
struct GenerationStats {
    pages_generated: usize,
    tokens_used: usize,
}

impl GenerationStats {
    fn new() -> Self {
        Self {
            pages_generated: 0,
            tokens_used: 0,
        }
    }
}

impl Default for ContentGenerationStrategy {
    fn default() -> Self {
        Self::new()
    }
}
