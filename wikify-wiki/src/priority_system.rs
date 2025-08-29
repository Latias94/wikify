//! Content priority system for intelligent wiki generation
//!
//! This module implements a sophisticated priority system inspired by DeepWiki
//! that determines the importance and generation order of wiki content.

use crate::types::{ImportanceLevel, RepositoryInfo};
use std::collections::HashMap;
use tracing::info;

/// Content priority analyzer that determines importance levels
pub struct ContentPriorityAnalyzer {
    /// File importance weights
    file_weights: HashMap<String, f64>,
    /// Directory importance weights  
    dir_weights: HashMap<String, f64>,
    /// Language-specific patterns
    language_patterns: HashMap<String, Vec<String>>,
}

impl ContentPriorityAnalyzer {
    /// Create a new priority analyzer
    pub fn new() -> Self {
        let mut analyzer = Self {
            file_weights: HashMap::new(),
            dir_weights: HashMap::new(),
            language_patterns: HashMap::new(),
        };

        analyzer.initialize_default_weights();
        analyzer
    }

    /// Initialize default importance weights
    fn initialize_default_weights(&mut self) {
        // Critical files (highest priority)
        self.file_weights.insert("README.md".to_string(), 1.0);
        self.file_weights.insert("README.rst".to_string(), 1.0);
        self.file_weights.insert("README.txt".to_string(), 0.9);
        self.file_weights.insert("CHANGELOG.md".to_string(), 0.8);
        self.file_weights.insert("CONTRIBUTING.md".to_string(), 0.7);

        // Configuration files (high priority)
        self.file_weights.insert("Cargo.toml".to_string(), 0.9);
        self.file_weights.insert("package.json".to_string(), 0.9);
        self.file_weights.insert("setup.py".to_string(), 0.9);
        self.file_weights
            .insert("requirements.txt".to_string(), 0.8);
        self.file_weights.insert("Dockerfile".to_string(), 0.8);
        self.file_weights
            .insert("docker-compose.yml".to_string(), 0.8);

        // Main entry points (high priority)
        self.file_weights.insert("main.rs".to_string(), 0.9);
        self.file_weights.insert("lib.rs".to_string(), 0.9);
        self.file_weights.insert("mod.rs".to_string(), 0.7);
        self.file_weights.insert("index.js".to_string(), 0.8);
        self.file_weights.insert("app.py".to_string(), 0.8);
        self.file_weights.insert("__init__.py".to_string(), 0.6);

        // Directory importance
        self.dir_weights.insert("src".to_string(), 0.9);
        self.dir_weights.insert("lib".to_string(), 0.8);
        self.dir_weights.insert("api".to_string(), 0.9);
        self.dir_weights.insert("core".to_string(), 0.9);
        self.dir_weights.insert("examples".to_string(), 0.7);
        self.dir_weights.insert("docs".to_string(), 0.8);
        self.dir_weights.insert("tests".to_string(), 0.5);
        self.dir_weights.insert("test".to_string(), 0.5);
        self.dir_weights.insert("target".to_string(), 0.1);
        self.dir_weights.insert("node_modules".to_string(), 0.1);
        self.dir_weights.insert(".git".to_string(), 0.1);

        // Language-specific patterns
        self.language_patterns.insert(
            "rust".to_string(),
            vec![
                "main.rs".to_string(),
                "lib.rs".to_string(),
                "mod.rs".to_string(),
                "Cargo.toml".to_string(),
            ],
        );

        self.language_patterns.insert(
            "python".to_string(),
            vec![
                "__init__.py".to_string(),
                "main.py".to_string(),
                "app.py".to_string(),
                "setup.py".to_string(),
                "requirements.txt".to_string(),
            ],
        );

        self.language_patterns.insert(
            "javascript".to_string(),
            vec![
                "index.js".to_string(),
                "app.js".to_string(),
                "package.json".to_string(),
                "webpack.config.js".to_string(),
            ],
        );
    }

    /// Analyze repository and determine content priorities
    pub fn analyze_priorities(
        &self,
        repo_info: &RepositoryInfo,
        file_list: &[String],
    ) -> ContentPriorityResult {
        info!("Analyzing content priorities for {} files", file_list.len());

        let mut priority_result = ContentPriorityResult::new();

        // Analyze file importance
        for file_path in file_list {
            let importance = self.calculate_file_importance(file_path, repo_info);
            priority_result
                .file_priorities
                .insert(file_path.clone(), importance);
        }

        // Determine critical pages
        priority_result.critical_pages = self.identify_critical_pages(repo_info, file_list);

        // Determine high priority pages
        priority_result.high_priority_pages =
            self.identify_high_priority_pages(repo_info, file_list);

        // Determine medium priority pages
        priority_result.medium_priority_pages =
            self.identify_medium_priority_pages(repo_info, file_list);

        info!(
            "Priority analysis complete: {} critical, {} high, {} medium priority pages",
            priority_result.critical_pages.len(),
            priority_result.high_priority_pages.len(),
            priority_result.medium_priority_pages.len()
        );

        priority_result
    }

    /// Calculate importance score for a specific file
    fn calculate_file_importance(&self, file_path: &str, repo_info: &RepositoryInfo) -> f64 {
        let mut score = 0.0;

        // Extract filename and directory
        let path_parts: Vec<&str> = file_path.split('/').collect();
        let filename = path_parts.last().map_or("", |v| v);

        // Check direct file importance
        if let Some(weight) = self.file_weights.get(filename) {
            score += weight;
        }

        // Check directory importance
        for part in &path_parts[..path_parts.len().saturating_sub(1)] {
            if let Some(weight) = self.dir_weights.get(*part) {
                score += weight * 0.5; // Directory weight is less than file weight
            }
        }

        // Language-specific bonuses
        for language in &repo_info.languages {
            if let Some(patterns) = self.language_patterns.get(&language.to_lowercase()) {
                for pattern in patterns {
                    if filename == pattern || file_path.contains(pattern) {
                        score += 0.3;
                    }
                }
            }
        }

        // API-related files get bonus
        if repo_info.has_api && (file_path.contains("api") || file_path.contains("endpoint")) {
            score += 0.4;
        }

        // Configuration files get bonus
        if filename.ends_with(".toml")
            || filename.ends_with(".json")
            || filename.ends_with(".yml")
            || filename.ends_with(".yaml")
        {
            score += 0.2;
        }

        // Main source files get bonus
        if file_path.starts_with("src/") || file_path.starts_with("lib/") {
            score += 0.3;
        }

        score.min(1.0) // Cap at 1.0
    }

    /// Identify critical pages that must be generated first
    fn identify_critical_pages(
        &self,
        _repo_info: &RepositoryInfo,
        _files: &[String],
    ) -> Vec<PageTemplate> {
        vec![
            PageTemplate {
                id: "overview".to_string(),
                title: "Project Overview".to_string(),
                description: "High-level introduction to the project".to_string(),
                importance: ImportanceLevel::Critical,
                estimated_tokens: 800,
            },
            PageTemplate {
                id: "getting-started".to_string(),
                title: "Getting Started".to_string(),
                description: "Quick start guide for new users".to_string(),
                importance: ImportanceLevel::Critical,
                estimated_tokens: 600,
            },
        ]
    }

    /// Identify high priority pages
    fn identify_high_priority_pages(
        &self,
        repo_info: &RepositoryInfo,
        _files: &[String],
    ) -> Vec<PageTemplate> {
        let mut pages = vec![
            PageTemplate {
                id: "installation".to_string(),
                title: "Installation".to_string(),
                description: "Installation and setup instructions".to_string(),
                importance: ImportanceLevel::High,
                estimated_tokens: 500,
            },
            PageTemplate {
                id: "architecture".to_string(),
                title: "Architecture".to_string(),
                description: "System architecture and design".to_string(),
                importance: ImportanceLevel::High,
                estimated_tokens: 1000,
            },
        ];

        // Add API reference if project has API
        if repo_info.has_api {
            pages.push(PageTemplate {
                id: "api-reference".to_string(),
                title: "API Reference".to_string(),
                description: "Complete API documentation".to_string(),
                importance: ImportanceLevel::High,
                estimated_tokens: 1200,
            });
        }

        pages
    }

    /// Identify medium priority pages
    fn identify_medium_priority_pages(
        &self,
        _repo_info: &RepositoryInfo,
        _files: &[String],
    ) -> Vec<PageTemplate> {
        vec![
            PageTemplate {
                id: "configuration".to_string(),
                title: "Configuration".to_string(),
                description: "Configuration options and settings".to_string(),
                importance: ImportanceLevel::Medium,
                estimated_tokens: 600,
            },
            PageTemplate {
                id: "examples".to_string(),
                title: "Examples".to_string(),
                description: "Usage examples and tutorials".to_string(),
                importance: ImportanceLevel::Medium,
                estimated_tokens: 800,
            },
            PageTemplate {
                id: "deployment".to_string(),
                title: "Deployment".to_string(),
                description: "Deployment and production setup".to_string(),
                importance: ImportanceLevel::Medium,
                estimated_tokens: 700,
            },
        ]
    }
}

/// Result of content priority analysis
#[derive(Debug, Clone)]
pub struct ContentPriorityResult {
    /// File importance scores (0.0 to 1.0)
    pub file_priorities: HashMap<String, f64>,
    /// Critical pages that must be generated first
    pub critical_pages: Vec<PageTemplate>,
    /// High priority pages
    pub high_priority_pages: Vec<PageTemplate>,
    /// Medium priority pages
    pub medium_priority_pages: Vec<PageTemplate>,
}

impl ContentPriorityResult {
    fn new() -> Self {
        Self {
            file_priorities: HashMap::new(),
            critical_pages: Vec::new(),
            high_priority_pages: Vec::new(),
            medium_priority_pages: Vec::new(),
        }
    }

    /// Get all pages sorted by priority
    pub fn get_pages_by_priority(&self) -> Vec<&PageTemplate> {
        let mut all_pages = Vec::new();
        all_pages.extend(&self.critical_pages);
        all_pages.extend(&self.high_priority_pages);
        all_pages.extend(&self.medium_priority_pages);
        all_pages
    }

    /// Get estimated total tokens needed
    pub fn estimate_total_tokens(&self) -> usize {
        self.get_pages_by_priority()
            .iter()
            .map(|page| page.estimated_tokens)
            .sum()
    }

    /// Get files above importance threshold
    pub fn get_important_files(&self, threshold: f64) -> Vec<String> {
        self.file_priorities
            .iter()
            .filter(|(_, &score)| score >= threshold)
            .map(|(path, _)| path.clone())
            .collect()
    }
}

/// Template for a wiki page to be generated
#[derive(Debug, Clone)]
pub struct PageTemplate {
    pub id: String,
    pub title: String,
    pub description: String,
    pub importance: ImportanceLevel,
    pub estimated_tokens: usize,
}

impl Default for ContentPriorityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
