//! Structured wiki generator inspired by DeepWiki's approach
//!
//! This module implements a more sophisticated wiki generation strategy
//! that creates structured content with proper hierarchy and importance levels.

use crate::enhanced_prompts::MarkdownPrompts;
use crate::markdown_organizer::{MarkdownOptions, MarkdownOrganizer};
use crate::types::*;
use std::collections::HashMap;
use wikify_core::{ErrorContext, WikifyError, WikifyResult};
use wikify_rag::RagPipeline;

use tracing::{error, info};

/// Structured wiki generator that creates hierarchical content
pub struct StructuredWikiGenerator {
    rag_pipeline: Option<RagPipeline>,
    markdown_organizer: MarkdownOrganizer,
}

impl Default for StructuredWikiGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl StructuredWikiGenerator {
    /// Create a new structured wiki generator
    pub fn new() -> Self {
        let markdown_options = MarkdownOptions::default();
        Self {
            rag_pipeline: None,
            markdown_organizer: MarkdownOrganizer::new(markdown_options),
        }
    }

    /// Set the RAG pipeline for content generation
    pub fn set_rag_pipeline(&mut self, rag_pipeline: RagPipeline) {
        self.rag_pipeline = Some(rag_pipeline);
    }

    /// Generate wiki structure using DeepWiki-inspired approach
    pub async fn generate_structured_wiki(
        &mut self,
        repo_path: &str,
        config: &WikiConfig,
    ) -> WikifyResult<WikiStructure> {
        info!("Starting structured wiki generation for: {}", repo_path);

        // Step 1: Analyze repository structure
        let repo_info = self.analyze_repository_structure(repo_path).await?;
        info!(
            "Repository analysis complete: {} files found",
            repo_info.total_files
        );

        // Step 2: Generate structured wiki outline using XML-based approach
        let wiki_structure = self.generate_wiki_structure(&repo_info, config).await?;
        info!(
            "Generated wiki structure with {} pages and {} sections",
            wiki_structure.pages.len(),
            wiki_structure.sections.len()
        );

        // Step 3: Generate content for pages based on importance
        let completed_wiki = self
            .generate_content_by_priority(wiki_structure, &repo_info, config)
            .await?;

        info!("✅ Structured wiki generation complete!");
        Ok(completed_wiki)
    }

    /// Analyze repository structure to understand the codebase
    async fn analyze_repository_structure(&self, repo_path: &str) -> WikifyResult<RepositoryInfo> {
        info!("Analyzing repository structure...");

        // This would typically scan the repository and categorize files
        // For now, we'll create a basic implementation
        let repo_info = RepositoryInfo {
            title: repo_path
                .split('/')
                .next_back()
                .unwrap_or("Repository")
                .to_string(),
            description: "Repository documentation".to_string(),
            languages: vec!["Rust".to_string()], // TODO: Auto-detect
            main_files: vec![],                  // TODO: Detect main entry points
            config_files: vec![],                // TODO: Detect config files
            setup_files: vec![],                 // TODO: Detect setup files
            api_files: vec![],                   // TODO: Detect API files
            total_files: 0,                      // TODO: Count files
            has_api: false,                      // TODO: Detect API presence
            readme_content: None,                // TODO: Extract README
        };

        Ok(repo_info)
    }

    /// Generate wiki structure using structured prompting
    async fn generate_wiki_structure(
        &self,
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
    ) -> WikifyResult<WikiStructure> {
        info!("Generating structured wiki outline...");

        let rag_pipeline =
            self.rag_pipeline
                .as_ref()
                .ok_or_else(|| WikifyError::WikiGeneration {
                    message: "RAG pipeline not initialized".to_string(),
                    source: None,
                    context: ErrorContext::new("structured_generator"),
                })?;

        // Create enhanced structured prompt for wiki generation
        let file_tree = ""; // TODO: Generate actual file tree
        let structure_prompt =
            MarkdownPrompts::create_enhanced_structure_prompt(repo_info, config, file_tree);

        let query = wikify_rag::create_simple_query(&structure_prompt);

        let response = rag_pipeline
            .ask(query)
            .await
            .map_err(|e| WikifyError::WikiGeneration {
                message: format!("Failed to generate wiki structure: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("structured_generator"),
            })?;

        // Parse the structured response
        self.parse_wiki_structure_response(&response.answer, repo_info)
    }

    /// Parse the XML-structured wiki response
    fn parse_wiki_structure_response(
        &self,
        _response: &str,
        repo_info: &RepositoryInfo,
    ) -> WikifyResult<WikiStructure> {
        info!("Parsing structured wiki response...");

        // For now, create a basic structure
        // TODO: Implement proper XML parsing
        let mut wiki_structure = WikiStructure::new(
            repo_info.title.clone(),
            repo_info.description.clone(),
            repo_info.title.clone(),
        );

        // Create sample pages based on common patterns
        let sample_pages = vec![
            (
                "overview",
                "Overview",
                "High-level overview of the project",
                ImportanceLevel::High,
            ),
            (
                "getting-started",
                "Getting Started",
                "Quick start guide for new users",
                ImportanceLevel::High,
            ),
            (
                "architecture",
                "Architecture",
                "System architecture and design",
                ImportanceLevel::High,
            ),
            (
                "api-reference",
                "API Reference",
                "Complete API documentation",
                ImportanceLevel::Medium,
            ),
            (
                "configuration",
                "Configuration",
                "Configuration options and setup",
                ImportanceLevel::Medium,
            ),
            (
                "examples",
                "Examples",
                "Usage examples and tutorials",
                ImportanceLevel::Medium,
            ),
            (
                "deployment",
                "Deployment",
                "Deployment and installation guide",
                ImportanceLevel::Medium,
            ),
            (
                "troubleshooting",
                "Troubleshooting",
                "Common issues and solutions",
                ImportanceLevel::Low,
            ),
            (
                "faq",
                "FAQ",
                "Frequently asked questions",
                ImportanceLevel::Low,
            ),
        ];

        for (id, title, description, importance) in sample_pages {
            let mut page =
                WikiPage::new(id.to_string(), title.to_string(), description.to_string());
            page.importance = importance;
            wiki_structure.pages.push(page);
        }

        // Create sample sections
        let intro_section = WikiSection {
            id: "introduction".to_string(),
            title: "Introduction".to_string(),
            description: "Getting started with the project".to_string(),
            pages: vec!["overview".to_string(), "getting-started".to_string()],
            subsections: vec![],
            parent_section: None,
            order: 1,
        };

        let technical_section = WikiSection {
            id: "technical".to_string(),
            title: "Technical Documentation".to_string(),
            description: "Detailed technical information".to_string(),
            pages: vec![
                "architecture".to_string(),
                "api-reference".to_string(),
                "configuration".to_string(),
            ],
            subsections: vec![],
            parent_section: None,
            order: 2,
        };

        let guides_section = WikiSection {
            id: "guides".to_string(),
            title: "Guides & Examples".to_string(),
            description: "Practical guides and examples".to_string(),
            pages: vec!["examples".to_string(), "deployment".to_string()],
            subsections: vec![],
            parent_section: None,
            order: 3,
        };

        let support_section = WikiSection {
            id: "support".to_string(),
            title: "Support".to_string(),
            description: "Help and troubleshooting".to_string(),
            pages: vec!["troubleshooting".to_string(), "faq".to_string()],
            subsections: vec![],
            parent_section: None,
            order: 4,
        };

        wiki_structure.sections = vec![
            intro_section,
            technical_section,
            guides_section,
            support_section,
        ];
        wiki_structure.root_sections = vec![
            "introduction".to_string(),
            "technical".to_string(),
            "guides".to_string(),
            "support".to_string(),
        ];

        Ok(wiki_structure)
    }

    /// Generate content for pages based on importance priority
    async fn generate_content_by_priority(
        &mut self,
        mut wiki_structure: WikiStructure,
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
    ) -> WikifyResult<WikiStructure> {
        info!("Generating content by priority...");

        let rag_pipeline =
            self.rag_pipeline
                .as_ref()
                .ok_or_else(|| WikifyError::WikiGeneration {
                    message: "RAG pipeline not initialized".to_string(),
                    source: None,
                    context: ErrorContext::new("structured_generator"),
                })?;

        // Sort pages by importance (High -> Medium -> Low)
        let mut page_indices: Vec<_> = (0..wiki_structure.pages.len()).collect();
        page_indices.sort_by(|&a, &b| {
            wiki_structure.pages[b]
                .importance
                .cmp(&wiki_structure.pages[a].importance)
        });

        // Generate content for pages concurrently (grouped by importance for better resource management)

        // Group pages by importance to control concurrency
        let mut high_priority_tasks = Vec::new();
        let mut medium_priority_tasks = Vec::new();
        let mut low_priority_tasks = Vec::new();

        for index in page_indices {
            let page = wiki_structure.pages[index].clone();
            let task_data = (index, page);

            match task_data.1.importance {
                ImportanceLevel::Critical | ImportanceLevel::High => {
                    high_priority_tasks.push(task_data);
                }
                ImportanceLevel::Medium => {
                    medium_priority_tasks.push(task_data);
                }
                ImportanceLevel::Low => {
                    low_priority_tasks.push(task_data);
                }
            }
        }

        // Process high priority pages first (concurrently within group)
        if !high_priority_tasks.is_empty() {
            info!(
                "🚀 Generating {} high-priority pages concurrently...",
                high_priority_tasks.len()
            );
            let results = self
                .generate_pages_concurrently(high_priority_tasks, repo_info, config, rag_pipeline)
                .await?;
            for (index, content) in results {
                wiki_structure.pages[index].content = content;
                wiki_structure.pages[index].estimate_reading_time();
                wiki_structure.pages[index].generated_at = chrono::Utc::now();
            }
        }

        // Process medium priority pages
        if !medium_priority_tasks.is_empty() {
            info!(
                "🚀 Generating {} medium-priority pages concurrently...",
                medium_priority_tasks.len()
            );
            let results = self
                .generate_pages_concurrently(medium_priority_tasks, repo_info, config, rag_pipeline)
                .await?;
            for (index, content) in results {
                wiki_structure.pages[index].content = content;
                wiki_structure.pages[index].estimate_reading_time();
                wiki_structure.pages[index].generated_at = chrono::Utc::now();
            }
        }

        // Process low priority pages
        if !low_priority_tasks.is_empty() {
            info!(
                "🚀 Generating {} low-priority pages concurrently...",
                low_priority_tasks.len()
            );
            let results = self
                .generate_pages_concurrently(low_priority_tasks, repo_info, config, rag_pipeline)
                .await?;
            for (index, content) in results {
                wiki_structure.pages[index].content = content;
                wiki_structure.pages[index].estimate_reading_time();
                wiki_structure.pages[index].generated_at = chrono::Utc::now();
            }
        }

        Ok(wiki_structure)
    }

    /// Generate multiple pages concurrently
    async fn generate_pages_concurrently(
        &self,
        page_tasks: Vec<(usize, WikiPage)>,
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
        rag_pipeline: &RagPipeline,
    ) -> WikifyResult<Vec<(usize, String)>> {
        use futures::future::join_all;

        // Create concurrent tasks
        let tasks: Vec<_> = page_tasks
            .into_iter()
            .map(|(index, page)| {
                let page_title = page.title.clone();
                let page_importance = page.importance.as_str();

                async move {
                    info!(
                        "🔄 Generating content for {} page: {}",
                        page_importance, page_title
                    );

                    let content_result = self
                        .generate_page_content(&page, repo_info, config, rag_pipeline)
                        .await;

                    match content_result {
                        Ok(content) => {
                            let word_count = content.split_whitespace().count();
                            info!("✅ Generated {} words for page: {}", word_count, page_title);
                            Ok((index, content))
                        }
                        Err(e) => {
                            error!(
                                "❌ Failed to generate content for page {}: {}",
                                page_title, e
                            );
                            Err(e)
                        }
                    }
                }
            })
            .collect();

        // Execute all tasks concurrently
        let results = join_all(tasks).await;

        // Collect successful results and handle errors
        let mut successful_results = Vec::new();
        for result in results {
            match result {
                Ok((index, content)) => successful_results.push((index, content)),
                Err(e) => return Err(e), // Fail fast on any error
            }
        }

        Ok(successful_results)
    }

    /// Generate markdown content directly for a specific page
    async fn generate_page_content(
        &self,
        page: &WikiPage,
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
        rag_pipeline: &RagPipeline,
    ) -> WikifyResult<String> {
        // Use enhanced prompt for direct markdown generation
        let markdown_prompt = MarkdownPrompts::create_direct_markdown_prompt(
            page,
            &page.file_paths,
            repo_info,
            config,
        );

        let query = wikify_rag::create_simple_query(&markdown_prompt);

        let response = rag_pipeline
            .ask(query)
            .await
            .map_err(|e| WikifyError::WikiGeneration {
                message: format!(
                    "Failed to generate markdown content for page '{}': {}",
                    page.title, e
                ),
                source: Some(Box::new(e)),
                context: ErrorContext::new("structured_generator"),
            })?;

        // The response should already be in markdown format
        Ok(response.answer)
    }

    /// Organize wiki content into markdown files
    pub fn organize_markdown_files(&self, wiki: &WikiStructure) -> HashMap<String, String> {
        info!("Organizing markdown files for wiki structure");
        self.markdown_organizer.organize_wiki_files(wiki)
    }
}

impl ImportanceLevel {
    /// Convert to string for display
    pub fn as_str(&self) -> &'static str {
        match self {
            ImportanceLevel::Critical => "critical",
            ImportanceLevel::High => "high",
            ImportanceLevel::Medium => "medium",
            ImportanceLevel::Low => "low",
        }
    }
}
