//! Wiki generator implementation
//! 
//! This module contains the core logic for generating wiki structures and content.

use crate::types::*;
use wikify_core::{WikifyResult, WikifyError, ErrorContext, DocumentInfo};
use wikify_rag::{RagPipeline, RagConfig, RagQuery};
use wikify_indexing::pipeline::IndexingPipeline;
use serde_json::Value;

use chrono::Utc;
use tracing::{info, debug, warn};

/// Main wiki generator that orchestrates the wiki creation process
pub struct WikiGenerator {
    rag_pipeline: Option<RagPipeline>,
    indexing_pipeline: IndexingPipeline,
}

impl WikiGenerator {
    /// Create a new WikiGenerator instance
    pub fn new() -> WikifyResult<Self> {
        let indexing_pipeline = IndexingPipeline::new()?;
        
        Ok(Self {
            rag_pipeline: None,
            indexing_pipeline,
        })
    }

    /// Initialize the RAG pipeline for content generation
    pub async fn initialize_rag(&mut self, config: &WikiConfig) -> WikifyResult<()> {
        info!("Initializing RAG pipeline for wiki generation");
        
        let mut rag_config = RagConfig::default();
        
        // Configure RAG based on wiki config
        rag_config.retrieval.similarity_threshold = 0.4; // Lower threshold for broader context
        rag_config.retrieval.top_k = 10; // More documents for comprehensive content
        rag_config.retrieval.max_context_length = 16000; // Larger context for detailed pages
        
        // Auto-detect LLM provider
        if std::env::var("OPENAI_API_KEY").is_ok() {
            rag_config.llm = wikify_rag::llm_client::configs::openai_gpt4o_mini();
            rag_config.embeddings.provider = "openai".to_string();
            rag_config.embeddings.model = "text-embedding-3-small".to_string();
        } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            rag_config.llm = wikify_rag::llm_client::configs::anthropic_claude_haiku();
        } else {
            rag_config.llm = wikify_rag::llm_client::configs::ollama_llama3(None);
        }

        let mut rag_pipeline = RagPipeline::new(rag_config);
        rag_pipeline.initialize().await?;
        
        self.rag_pipeline = Some(rag_pipeline);
        Ok(())
    }

    /// Generate the overall wiki structure by analyzing the repository
    pub async fn generate_structure(
        &mut self,
        repo_path: &str,
        config: &WikiConfig,
    ) -> WikifyResult<WikiStructure> {
        info!("Generating wiki structure for repository: {}", repo_path);

        // Initialize RAG if not already done
        if self.rag_pipeline.is_none() {
            self.initialize_rag(config).await?;
        }

        // Index the repository
        info!("Indexing repository for wiki generation");
        self.indexing_pipeline.index_repository(repo_path).await?;

        // Get repository information
        let repo_info = self.analyze_repository_structure(repo_path, config).await?;
        
        // Generate wiki structure using LLM
        let wiki_structure = self.generate_wiki_structure_with_llm(&repo_info, config).await?;
        
        info!("Generated wiki structure with {} pages and {} sections", 
              wiki_structure.pages.len(), wiki_structure.sections.len());
        
        Ok(wiki_structure)
    }

    /// Generate content for a specific wiki page
    pub async fn generate_page_content(
        &mut self,
        page: &WikiPage,
        repo_path: &str,
        config: &WikiConfig,
    ) -> WikifyResult<WikiPage> {
        info!("Generating content for page: {}", page.title);

        let rag_pipeline = self.rag_pipeline.as_mut()
            .ok_or_else(|| WikifyError::Config {
                message: "RAG pipeline not initialized".to_string(),
                source: None,
                context: ErrorContext::new("wiki_generator"),
            })?;

        // Create a comprehensive prompt for this page
        let prompt = self.create_page_generation_prompt(page, config);
        
        // Query RAG system for relevant information
        let query = RagQuery {
            question: prompt,
            context: None,
            filters: None,
            retrieval_config: None,
        };

        let rag_response = rag_pipeline.query(query).await?;
        
        // Process the response to create structured content
        let mut generated_page = page.clone();
        generated_page.content = self.process_generated_content(&rag_response.answer, page, config)?;
        generated_page.generated_at = Utc::now();
        generated_page.source_documents = rag_response.sources;
        generated_page.estimate_reading_time();

        info!("Generated content for page '{}' ({} words, {} min read)", 
              page.title, 
              generated_page.content.split_whitespace().count(),
              generated_page.reading_time);

        Ok(generated_page)
    }

    /// Analyze repository structure to understand the codebase
    async fn analyze_repository_structure(
        &self,
        repo_path: &str,
        config: &WikiConfig,
    ) -> WikifyResult<RepositoryInfo> {
        debug!("Analyzing repository structure: {}", repo_path);

        let mut repo_info = RepositoryInfo {
            path: repo_path.to_string(),
            name: std::path::Path::new(repo_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string(),
            description: String::new(),
            languages: Vec::new(),
            main_files: Vec::new(),
            directory_structure: Vec::new(),
            readme_content: None,
        };

        // Read README if exists
        let readme_paths = ["README.md", "README.rst", "README.txt", "readme.md"];
        for readme_path in &readme_paths {
            let full_path = std::path::Path::new(repo_path).join(readme_path);
            if full_path.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&full_path).await {
                    repo_info.readme_content = Some(content);
                    break;
                }
            }
        }

        // Analyze directory structure
        repo_info.directory_structure = self.get_directory_structure(repo_path, config).await?;
        
        // Detect programming languages
        repo_info.languages = self.detect_languages(&repo_info.directory_structure);
        
        // Identify main files
        repo_info.main_files = self.identify_main_files(&repo_info.directory_structure);

        Ok(repo_info)
    }

    /// Generate wiki structure using LLM analysis
    async fn generate_wiki_structure_with_llm(
        &mut self,
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
    ) -> WikifyResult<WikiStructure> {
        let rag_pipeline = self.rag_pipeline.as_mut()
            .ok_or_else(|| WikifyError::Config {
                message: "RAG pipeline not initialized".to_string(),
                source: None,
                context: ErrorContext::new("wiki_generator"),
            })?;

        // Create structure analysis prompt
        let prompt = self.create_structure_analysis_prompt(repo_info, config);
        
        let query = RagQuery {
            question: prompt,
            context: None,
            filters: None,
            retrieval_config: None,
        };

        let response = rag_pipeline.query(query).await?;
        
        // Parse the structured response
        let wiki_structure = self.parse_wiki_structure_response(&response.answer, repo_info, config)?;
        
        Ok(wiki_structure)
    }

    /// Create a prompt for analyzing repository structure
    fn create_structure_analysis_prompt(&self, repo_info: &RepositoryInfo, config: &WikiConfig) -> String {
        let readme_section = if let Some(readme) = &repo_info.readme_content {
            format!("README Content:\n{}\n\n", readme)
        } else {
            String::new()
        };

        let languages_section = if !repo_info.languages.is_empty() {
            format!("Programming Languages: {}\n\n", repo_info.languages.join(", "))
        } else {
            String::new()
        };

        let files_section = if !repo_info.main_files.is_empty() {
            format!("Key Files:\n{}\n\n", repo_info.main_files.join("\n"))
        } else {
            String::new()
        };

        format!(
            r#"You are an expert technical writer and software architect. Analyze this repository and create a comprehensive wiki structure.

Repository: {}
{}{}{}

Please analyze this codebase and create a wiki structure with the following requirements:

1. Create 5-15 wiki pages covering the most important aspects
2. Each page should focus on a specific feature, module, or concept
3. Organize pages by importance (high/medium/low)
4. Include relevant file paths for each page
5. Create logical sections to group related pages

Return your analysis in the following JSON format:

{{
  "title": "Repository Wiki Title",
  "description": "Brief description of the repository",
  "pages": [
    {{
      "id": "page-1",
      "title": "Page Title",
      "description": "What this page covers",
      "importance": "high|medium|low",
      "file_paths": ["path/to/relevant/file.rs"],
      "tags": ["tag1", "tag2"]
    }}
  ],
  "sections": [
    {{
      "id": "section-1",
      "title": "Section Title",
      "description": "What this section covers",
      "pages": ["page-1", "page-2"],
      "order": 1
    }}
  ]
}}

Focus on creating practical, useful documentation that would help developers understand and work with this codebase."#,
            repo_info.name,
            readme_section,
            languages_section,
            files_section
        )
    }

    /// Create a prompt for generating specific page content
    fn create_page_generation_prompt(&self, page: &WikiPage, config: &WikiConfig) -> String {
        let files_context = if !page.file_paths.is_empty() {
            format!("Focus on these files: {}\n\n", page.file_paths.join(", "))
        } else {
            String::new()
        };

        format!(
            r#"Generate comprehensive technical documentation for: {}

Description: {}
{}
Requirements:
1. Write in {} language
2. Use clear, professional technical writing
3. Include code examples where relevant
4. Explain concepts thoroughly but concisely
5. Use proper Markdown formatting
6. Include relevant diagrams if helpful (using Mermaid syntax)

Structure the content with:
- Overview/Introduction
- Key Concepts
- Implementation Details
- Code Examples
- Best Practices
- Related Information

Generate detailed, accurate documentation that would be valuable for developers working with this codebase."#,
            page.title,
            page.description,
            files_context,
            config.language
        )
    }

    /// Parse the LLM response into a WikiStructure
    fn parse_wiki_structure_response(
        &self,
        response: &str,
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
    ) -> WikifyResult<WikiStructure> {
        // Try to extract JSON from the response
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        let parsed: Value = serde_json::from_str(json_str)
            .map_err(|e| WikifyError::Config {
                message: format!("Failed to parse wiki structure JSON: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("wiki_generator")
                    .with_suggestion("Check LLM response format"),
            })?;

        // Convert parsed JSON to WikiStructure
        let mut wiki_structure = WikiStructure::new(
            parsed["title"].as_str().unwrap_or(&repo_info.name).to_string(),
            parsed["description"].as_str().unwrap_or("Generated wiki").to_string(),
            repo_info.path.clone(),
        );

        // Parse pages
        if let Some(pages_array) = parsed["pages"].as_array() {
            for (index, page_obj) in pages_array.iter().enumerate() {
                let page_id = page_obj["id"].as_str()
                    .unwrap_or(&format!("page-{}", index + 1))
                    .to_string();
                
                let mut page = WikiPage::new(
                    page_id,
                    page_obj["title"].as_str().unwrap_or("Untitled").to_string(),
                    page_obj["description"].as_str().unwrap_or("").to_string(),
                );

                // Set importance
                if let Some(importance_str) = page_obj["importance"].as_str() {
                    page.importance = match importance_str {
                        "high" => ImportanceLevel::High,
                        "medium" => ImportanceLevel::Medium,
                        "low" => ImportanceLevel::Low,
                        "critical" => ImportanceLevel::Critical,
                        _ => ImportanceLevel::Medium,
                    };
                }

                // Set file paths
                if let Some(files_array) = page_obj["file_paths"].as_array() {
                    page.file_paths = files_array
                        .iter()
                        .filter_map(|f| f.as_str())
                        .map(|s| s.to_string())
                        .collect();
                }

                // Set tags
                if let Some(tags_array) = page_obj["tags"].as_array() {
                    page.tags = tags_array
                        .iter()
                        .filter_map(|t| t.as_str())
                        .map(|s| s.to_string())
                        .collect();
                }

                wiki_structure.pages.push(page);
            }
        }

        // Parse sections
        if let Some(sections_array) = parsed["sections"].as_array() {
            for (index, section_obj) in sections_array.iter().enumerate() {
                let section_id = section_obj["id"].as_str()
                    .unwrap_or(&format!("section-{}", index + 1))
                    .to_string();

                let mut section = WikiSection {
                    id: section_id.clone(),
                    title: section_obj["title"].as_str().unwrap_or("Untitled Section").to_string(),
                    description: section_obj["description"].as_str().unwrap_or("").to_string(),
                    pages: Vec::new(),
                    subsections: Vec::new(),
                    parent_section: None,
                    order: section_obj["order"].as_u64().unwrap_or(index as u64 + 1) as u32,
                };

                // Set pages in section
                if let Some(pages_array) = section_obj["pages"].as_array() {
                    section.pages = pages_array
                        .iter()
                        .filter_map(|p| p.as_str())
                        .map(|s| s.to_string())
                        .collect();
                }

                wiki_structure.sections.push(section);
                wiki_structure.root_sections.push(section_id);
            }
        }

        // Update metadata
        wiki_structure.metadata.config = config.clone();
        wiki_structure.metadata.stats.total_pages = wiki_structure.pages.len();
        wiki_structure.metadata.stats.total_sections = wiki_structure.sections.len();

        Ok(wiki_structure)
    }

    /// Process generated content to ensure proper formatting
    fn process_generated_content(
        &self,
        raw_content: &str,
        page: &WikiPage,
        _config: &WikiConfig,
    ) -> WikifyResult<String> {
        let mut processed_content = raw_content.to_string();

        // Ensure the content starts with the page title as H1
        if !processed_content.starts_with(&format!("# {}", page.title)) {
            processed_content = format!("# {}\n\n{}", page.title, processed_content);
        }

        // Add metadata section at the end
        processed_content.push_str(&format!(
            r#"

---

## Metadata

- **Generated**: {}
- **Importance**: {:?}
- **Reading Time**: {} minutes
- **Source Files**: {}
- **Tags**: {}
"#,
            page.generated_at.format("%Y-%m-%d %H:%M:%S UTC"),
            page.importance,
            page.reading_time,
            if page.file_paths.is_empty() { "None".to_string() } else { page.file_paths.join(", ") },
            if page.tags.is_empty() { "None".to_string() } else { page.tags.join(", ") }
        ));

        Ok(processed_content)
    }

    // Helper methods for repository analysis
    async fn get_directory_structure(&self, repo_path: &str, config: &WikiConfig) -> WikifyResult<Vec<String>> {
        let mut files = Vec::new();
        let walker = walkdir::WalkDir::new(repo_path)
            .max_depth(5)
            .into_iter()
            .filter_entry(|e| {
                let path = e.path();
                let path_str = path.to_string_lossy();
                
                // Skip excluded directories
                !config.excluded_dirs.iter().any(|excluded| path_str.contains(excluded))
            });

        for entry in walker {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    if let Some(path_str) = entry.path().strip_prefix(repo_path).ok() {
                        files.push(path_str.to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(files)
    }

    fn detect_languages(&self, files: &[String]) -> Vec<String> {
        let mut languages = std::collections::HashSet::new();
        
        for file in files {
            if let Some(extension) = std::path::Path::new(file).extension() {
                if let Some(ext_str) = extension.to_str() {
                    let language = match ext_str {
                        "rs" => "Rust",
                        "py" => "Python",
                        "js" | "jsx" => "JavaScript",
                        "ts" | "tsx" => "TypeScript",
                        "java" => "Java",
                        "go" => "Go",
                        "cpp" | "cc" | "cxx" => "C++",
                        "c" => "C",
                        "cs" => "C#",
                        "php" => "PHP",
                        "rb" => "Ruby",
                        _ => continue,
                    };
                    languages.insert(language.to_string());
                }
            }
        }
        
        languages.into_iter().collect()
    }

    fn identify_main_files(&self, files: &[String]) -> Vec<String> {
        let important_patterns = [
            "main.", "index.", "app.", "server.", "client.",
            "Cargo.toml", "package.json", "requirements.txt",
            "Dockerfile", "docker-compose.yml",
            "README", "LICENSE", "CHANGELOG"
        ];

        files.iter()
            .filter(|file| {
                important_patterns.iter().any(|pattern| {
                    file.to_lowercase().contains(&pattern.to_lowercase())
                })
            })
            .cloned()
            .collect()
    }
}

/// Repository information gathered during analysis
#[derive(Debug, Clone)]
struct RepositoryInfo {
    path: String,
    name: String,
    description: String,
    languages: Vec<String>,
    main_files: Vec<String>,
    directory_structure: Vec<String>,
    readme_content: Option<String>,
}
