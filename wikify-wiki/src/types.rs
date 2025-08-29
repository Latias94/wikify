//! Wiki type definitions
//!
//! This module defines the core data structures used for wiki generation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wikify_core::DocumentInfo;

/// Configuration for wiki generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiConfig {
    /// Whether to force regeneration even if cache exists
    pub force_regenerate: bool,
    /// Language for content generation
    pub language: String,
    /// Maximum number of pages to generate
    pub max_pages: Option<usize>,
    /// Include diagrams and visualizations
    pub include_diagrams: bool,
    /// Template style to use
    pub template_style: TemplateStyle,
    /// Directories to exclude from analysis
    pub excluded_dirs: Vec<String>,
    /// File patterns to exclude
    pub excluded_files: Vec<String>,
    /// Directories to include exclusively
    pub included_dirs: Option<Vec<String>>,
    /// File patterns to include exclusively
    pub included_files: Option<Vec<String>>,
    /// Minimum importance level for pages
    pub min_importance: ImportanceLevel,
    /// Whether to generate comprehensive view
    pub comprehensive_view: bool,
}

/// Template styles for wiki generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateStyle {
    /// Clean, minimal documentation style
    Minimal,
    /// Comprehensive technical documentation
    Technical,
    /// Tutorial-focused style
    Tutorial,
    /// API reference style
    ApiReference,
    /// Custom template path
    Custom(String),
}

/// Importance levels for wiki pages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportanceLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Complete wiki structure for a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiStructure {
    /// Unique identifier for the wiki
    pub id: String,
    /// Title of the wiki (usually repository name)
    pub title: String,
    /// Brief description of the repository
    pub description: String,
    /// All wiki pages
    pub pages: Vec<WikiPage>,
    /// Hierarchical sections for organization
    pub sections: Vec<WikiSection>,
    /// Root-level sections (not nested under others)
    pub root_sections: Vec<String>,
    /// Metadata about the wiki
    pub metadata: WikiMetadata,
}

/// Individual wiki page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    /// Unique identifier for the page
    pub id: String,
    /// Page title
    pub title: String,
    /// Generated markdown content
    pub content: String,
    /// Brief description of the page
    pub description: String,
    /// Importance level of this page
    pub importance: ImportanceLevel,
    /// File paths relevant to this page
    pub file_paths: Vec<String>,
    /// IDs of related pages
    pub related_pages: Vec<String>,
    /// Parent section ID (if any)
    pub parent_section: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Estimated reading time in minutes
    pub reading_time: u32,
    /// When this page was generated
    pub generated_at: DateTime<Utc>,
    /// Source documents used for generation
    pub source_documents: Vec<DocumentInfo>,
}

/// Hierarchical section for organizing pages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiSection {
    /// Unique identifier for the section
    pub id: String,
    /// Section title
    pub title: String,
    /// Brief description of the section
    pub description: String,
    /// Page IDs in this section
    pub pages: Vec<String>,
    /// Subsection IDs
    pub subsections: Vec<String>,
    /// Parent section ID (if any)
    pub parent_section: Option<String>,
    /// Order within parent section
    pub order: u32,
}

/// Metadata about the wiki
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiMetadata {
    /// Repository URL or path
    pub repository: String,
    /// When the wiki was generated
    pub generated_at: DateTime<Utc>,
    /// Configuration used for generation
    pub config: WikiConfig,
    /// Statistics about the generation process
    pub stats: WikiStats,
    /// Version of wikify used
    pub wikify_version: String,
}

/// Statistics about wiki generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiStats {
    /// Total number of pages generated
    pub total_pages: usize,
    /// Total number of sections
    pub total_sections: usize,
    /// Total number of source files analyzed
    pub total_files: usize,
    /// Total generation time in seconds
    pub generation_time_seconds: f64,
    /// Total tokens used for generation
    pub total_tokens_used: usize,
    /// Estimated cost in USD
    pub estimated_cost: f64,
}

/// Diagram or visualization in a wiki page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiDiagram {
    /// Unique identifier
    pub id: String,
    /// Diagram title
    pub title: String,
    /// Diagram type (mermaid, plantuml, etc.)
    pub diagram_type: DiagramType,
    /// Diagram source code
    pub source: String,
    /// Description of what the diagram shows
    pub description: String,
}

/// Types of diagrams supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagramType {
    /// Mermaid diagrams
    Mermaid,
    /// PlantUML diagrams
    PlantUml,
    /// Graphviz DOT diagrams
    Graphviz,
    /// ASCII art diagrams
    Ascii,
}

/// Progress information for wiki generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiProgress {
    /// Current step being executed
    pub current_step: String,
    /// Number of completed steps
    pub completed_steps: usize,
    /// Total number of steps
    pub total_steps: usize,
    /// Current page being processed (if applicable)
    pub current_page: Option<String>,
    /// Number of completed pages
    pub completed_pages: usize,
    /// Total number of pages to generate
    pub total_pages: usize,
    /// Estimated time remaining in seconds
    pub estimated_remaining_seconds: Option<f64>,
}

impl Default for WikiConfig {
    fn default() -> Self {
        Self {
            force_regenerate: false,
            language: "en".to_string(),
            max_pages: Some(50),
            include_diagrams: true,
            template_style: TemplateStyle::Technical,
            excluded_dirs: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".vscode".to_string(),
                ".idea".to_string(),
            ],
            excluded_files: vec![
                "*.log".to_string(),
                "*.tmp".to_string(),
                "*.cache".to_string(),
            ],
            included_dirs: None,
            included_files: None,
            min_importance: ImportanceLevel::Low,
            comprehensive_view: false,
        }
    }
}

impl WikiStructure {
    /// Create a new empty wiki structure
    pub fn new(title: String, description: String, repository: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            description,
            pages: Vec::new(),
            sections: Vec::new(),
            root_sections: Vec::new(),
            metadata: WikiMetadata {
                repository,
                generated_at: Utc::now(),
                config: WikiConfig::default(),
                stats: WikiStats {
                    total_pages: 0,
                    total_sections: 0,
                    total_files: 0,
                    generation_time_seconds: 0.0,
                    total_tokens_used: 0,
                    estimated_cost: 0.0,
                },
                wikify_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    /// Get pages by importance level
    pub fn pages_by_importance(&self, importance: ImportanceLevel) -> Vec<&WikiPage> {
        self.pages
            .iter()
            .filter(|page| page.importance >= importance)
            .collect()
    }

    /// Get pages in a specific section
    pub fn pages_in_section(&self, section_id: &str) -> Vec<&WikiPage> {
        if let Some(section) = self.sections.iter().find(|s| s.id == section_id) {
            section
                .pages
                .iter()
                .filter_map(|page_id| self.pages.iter().find(|p| p.id == *page_id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get total estimated reading time
    pub fn total_reading_time(&self) -> u32 {
        self.pages.iter().map(|page| page.reading_time).sum()
    }
}

impl WikiPage {
    /// Create a new wiki page
    pub fn new(id: String, title: String, description: String) -> Self {
        Self {
            id,
            title,
            description,
            content: String::new(),
            importance: ImportanceLevel::Medium,
            file_paths: Vec::new(),
            related_pages: Vec::new(),
            parent_section: None,
            tags: Vec::new(),
            reading_time: 0,
            generated_at: Utc::now(),
            source_documents: Vec::new(),
        }
    }

    /// Estimate reading time based on content length
    pub fn estimate_reading_time(&mut self) {
        // Average reading speed: 200 words per minute
        let word_count = self.content.split_whitespace().count();
        self.reading_time = ((word_count as f64 / 200.0).ceil() as u32).max(1);
    }

    /// Check if this page has content
    pub fn has_content(&self) -> bool {
        !self.content.trim().is_empty()
    }

    /// Get content length in words
    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }

    /// Get content summary (first paragraph or first 200 characters)
    pub fn get_summary(&self) -> String {
        if self.content.is_empty() {
            return "No content available".to_string();
        }

        // Try to get first paragraph
        if let Some(first_para) = self.content.split("\n\n").next() {
            if first_para.len() <= 200 {
                return first_para.trim().to_string();
            }
        }

        // Fallback to first 200 characters
        let summary = self.content.chars().take(200).collect::<String>();
        if summary.len() < self.content.len() {
            format!("{}...", summary.trim())
        } else {
            summary.trim().to_string()
        }
    }
}

/// Repository information gathered during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    pub title: String,
    pub description: String,
    pub languages: Vec<String>,
    pub main_files: Vec<String>,
    pub config_files: Vec<String>,
    pub setup_files: Vec<String>,
    pub api_files: Vec<String>,
    pub total_files: usize,
    pub has_api: bool,
    pub readme_content: Option<String>,
}
