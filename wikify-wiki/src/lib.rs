//! Wikify Wiki Generation Module
//!
//! This module provides functionality to generate comprehensive wiki documentation
//! from code repositories using RAG (Retrieval-Augmented Generation).

pub mod prompts;
pub mod types;

// Re-export main types and functions
pub use types::*;

/// Export formats supported by the wiki exporter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Export as individual Markdown files
    Markdown,
    /// Export as HTML files
    Html,
    /// Export as a single JSON file
    Json,
    /// Export as a single PDF file
    Pdf,
}

/// Main Wiki service that orchestrates wiki generation with RAG
pub struct WikiService;

impl WikiService {
    /// Create a new WikiService instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self)
    }

    /// Get cached wiki if available (placeholder)
    pub async fn get_cached_wiki(
        &self,
        _repo_path: &str,
    ) -> Result<Option<WikiStructure>, Box<dyn std::error::Error + Send + Sync>> {
        // Placeholder implementation
        Ok(None)
    }

    /// Generate a complete wiki for a repository
    pub async fn generate_wiki(
        &mut self,
        repo_path: &str,
        config: &WikiConfig,
    ) -> Result<WikiStructure, Box<dyn std::error::Error + Send + Sync>> {
        println!("🔍 Analyzing repository structure...");

        // Analyze repository
        let repo_info = self.analyze_repository(repo_path).await?;

        println!("📝 Generating wiki structure...");

        // Create wiki structure
        let mut wiki = WikiStructure::new(
            repo_info.title.clone(),
            repo_info.description.clone(),
            repo_path.to_string(),
        );

        // Generate pages based on repository analysis
        let mut pages = Vec::new();
        let mut sections = Vec::new();

        // 1. Overview page
        let overview_page = WikiPage {
            id: "overview".to_string(),
            title: "Overview".to_string(),
            description: "Project overview and introduction".to_string(),
            content: self.generate_overview_content(&repo_info),
            importance: ImportanceLevel::High,
            file_paths: repo_info.main_files.clone(),
            related_pages: vec![],
            parent_section: Some("getting-started".to_string()),
            tags: vec!["overview".to_string(), "introduction".to_string()],
            reading_time: 3,
            generated_at: chrono::Utc::now(),
            source_documents: vec![],
        };
        pages.push(overview_page);

        // 2. Getting Started page
        let getting_started_page = WikiPage {
            id: "getting-started".to_string(),
            title: "Getting Started".to_string(),
            description: "How to set up and run the project".to_string(),
            content: self.generate_getting_started_content(&repo_info),
            importance: ImportanceLevel::High,
            file_paths: repo_info.setup_files.clone(),
            related_pages: vec!["overview".to_string()],
            parent_section: Some("getting-started".to_string()),
            tags: vec!["setup".to_string(), "installation".to_string()],
            reading_time: 4,
            generated_at: chrono::Utc::now(),
            source_documents: vec![],
        };
        pages.push(getting_started_page);

        // Create sections
        let getting_started_section = WikiSection {
            id: "getting-started".to_string(),
            title: "Getting Started".to_string(),
            description: "Everything you need to get up and running".to_string(),
            pages: vec!["overview".to_string(), "getting-started".to_string()],
            subsections: vec![],
            parent_section: None,
            order: 1,
        };
        sections.push(getting_started_section);

        let technical_section = WikiSection {
            id: "technical".to_string(),
            title: "Technical Documentation".to_string(),
            description: "In-depth technical information".to_string(),
            pages: vec![],
            subsections: vec![],
            parent_section: None,
            order: 2,
        };
        sections.push(technical_section);

        // Update wiki structure
        wiki.pages = pages;
        wiki.sections = sections;
        wiki.root_sections = vec!["getting-started".to_string(), "technical".to_string()];

        // Update metadata
        wiki.metadata.config = config.clone();
        wiki.metadata.stats.total_pages = wiki.pages.len();
        wiki.metadata.stats.total_sections = wiki.sections.len();
        wiki.metadata.stats.total_files = repo_info.total_files;

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
        match format {
            ExportFormat::Markdown => self.export_markdown(wiki, output_path).await,
            ExportFormat::Html => {
                println!("HTML export functionality is not yet implemented");
                Ok(())
            }
            ExportFormat::Json => self.export_json(wiki, output_path).await,
            ExportFormat::Pdf => {
                println!("PDF export functionality is not yet implemented");
                Ok(())
            }
        }
    }

    /// Export wiki as Markdown files
    async fn export_markdown(
        &self,
        wiki: &WikiStructure,
        output_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::path::Path;
        use tokio::fs;

        println!("📝 Exporting wiki as Markdown to: {}", output_path);

        let output_dir = Path::new(output_path);

        // Create output directory
        fs::create_dir_all(output_dir).await?;

        // Create index file (README.md)
        let index_content = self.generate_markdown_index(wiki);
        let index_path = output_dir.join("README.md");
        fs::write(&index_path, index_content).await?;
        println!("  ✅ Created README.md");

        // Export each page as a separate Markdown file
        for page in &wiki.pages {
            let filename = self.sanitize_filename(&format!("{}.md", page.title));
            let page_path = output_dir.join(&filename);

            let page_content = self.generate_markdown_page(page);
            fs::write(&page_path, page_content).await?;
            println!("  ✅ Created {}", filename);
        }

        // Create table of contents
        let toc_content = self.generate_table_of_contents(wiki);
        let toc_path = output_dir.join("TABLE_OF_CONTENTS.md");
        fs::write(&toc_path, toc_content).await?;
        println!("  ✅ Created TABLE_OF_CONTENTS.md");

        println!(
            "✅ Successfully exported {} pages as Markdown",
            wiki.pages.len()
        );
        Ok(())
    }

    /// Export wiki as JSON
    async fn export_json(
        &self,
        wiki: &WikiStructure,
        output_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::path::Path;
        use tokio::fs;

        println!("📄 Exporting wiki as JSON to: {}", output_path);

        let json_content = serde_json::to_string_pretty(wiki)?;

        // Ensure parent directory exists
        if let Some(parent) = Path::new(output_path).parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(output_path, json_content).await?;
        println!("✅ Successfully exported wiki as JSON");
        Ok(())
    }

    /// Generate Markdown index content
    fn generate_markdown_index(&self, wiki: &WikiStructure) -> String {
        let mut content = format!("# {}\n\n{}\n\n", wiki.title, wiki.description);

        content.push_str("## Table of Contents\n\n");

        // Group pages by section
        for section in &wiki.sections {
            content.push_str(&format!("### {}\n\n", section.title));
            if !section.description.is_empty() {
                content.push_str(&format!("{}\n\n", section.description));
            }

            for page_id in &section.pages {
                if let Some(page) = wiki.pages.iter().find(|p| p.id == *page_id) {
                    let filename = self.sanitize_filename(&format!("{}.md", page.title));
                    content.push_str(&format!(
                        "- [{}]({}) - {}\n",
                        page.title, filename, page.description
                    ));
                }
            }
            content.push('\n');
        }

        // Add pages not in any section
        let pages_in_sections: std::collections::HashSet<_> =
            wiki.sections.iter().flat_map(|s| &s.pages).collect();

        let orphan_pages: Vec<_> = wiki
            .pages
            .iter()
            .filter(|p| !pages_in_sections.contains(&p.id))
            .collect();

        if !orphan_pages.is_empty() {
            content.push_str("### Other Pages\n\n");
            for page in orphan_pages {
                let filename = self.sanitize_filename(&format!("{}.md", page.title));
                content.push_str(&format!(
                    "- [{}]({}) - {}\n",
                    page.title, filename, page.description
                ));
            }
        }

        content.push_str(&format!(
            "\n---\n\n*Generated by Wikify on {}*\n",
            wiki.metadata.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        content
    }

    /// Generate Markdown content for a single page
    fn generate_markdown_page(&self, page: &WikiPage) -> String {
        let mut content = format!(
            "[← Back to Index](README.md)\n\n# {}\n\n{}\n\n",
            page.title, page.description
        );

        // Add the main content
        if !page.content.is_empty() {
            content.push_str(&page.content);
        } else {
            content.push_str("*Content will be generated here.*\n");
        }

        // Add metadata section
        content.push_str(&format!(
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
            if page.file_paths.is_empty() {
                "None".to_string()
            } else {
                page.file_paths.join(", ")
            },
            if page.tags.is_empty() {
                "None".to_string()
            } else {
                page.tags.join(", ")
            }
        ));

        content
    }

    /// Generate table of contents
    fn generate_table_of_contents(&self, wiki: &WikiStructure) -> String {
        let mut content = format!("# Table of Contents - {}\n\n", wiki.title);

        // Sort pages by importance and title
        let mut sorted_pages = wiki.pages.clone();
        sorted_pages.sort_by(|a, b| {
            b.importance
                .cmp(&a.importance)
                .then_with(|| a.title.cmp(&b.title))
        });

        for page in &sorted_pages {
            let filename = self.sanitize_filename(&format!("{}.md", page.title));
            content.push_str(&format!(
                "- [{}]({}) ({:?}) - {}\n",
                page.title, filename, page.importance, page.description
            ));
        }

        content
    }

    /// Sanitize filename for filesystem compatibility
    fn sanitize_filename(&self, filename: &str) -> String {
        filename
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Analyze repository structure
    async fn analyze_repository(
        &self,
        repo_path: &str,
    ) -> Result<RepositoryInfo, Box<dyn std::error::Error + Send + Sync>> {
        use std::path::Path;
        use tokio::fs;

        let repo_path = Path::new(repo_path);
        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown Project")
            .to_string();

        let mut repo_info = RepositoryInfo {
            title: format!("{} Documentation", repo_name),
            description: format!("Comprehensive documentation for the {} project", repo_name),
            languages: Vec::new(),
            main_files: Vec::new(),
            config_files: Vec::new(),
            setup_files: Vec::new(),
            api_files: Vec::new(),
            total_files: 0,
            has_api: false,
            readme_content: None,
        };

        // Read README if exists
        let readme_paths = ["README.md", "README.rst", "README.txt", "readme.md"];
        for readme_path in &readme_paths {
            let full_path = repo_path.join(readme_path);
            if full_path.exists() {
                if let Ok(content) = fs::read_to_string(&full_path).await {
                    repo_info.readme_content = Some(content);
                    // Extract title from README if possible
                    if let Some(first_line) =
                        repo_info.readme_content.as_ref().unwrap().lines().next()
                    {
                        if first_line.starts_with("# ") {
                            repo_info.title = first_line[2..].trim().to_string();
                        }
                    }
                    break;
                }
            }
        }

        // Analyze files
        if let Ok(mut entries) = fs::read_dir(repo_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                repo_info.total_files += 1;

                // Detect languages and categorize files
                if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                    let language = match extension {
                        "rs" => Some("Rust"),
                        "py" => Some("Python"),
                        "js" | "jsx" => Some("JavaScript"),
                        "ts" | "tsx" => Some("TypeScript"),
                        "java" => Some("Java"),
                        "go" => Some("Go"),
                        "cpp" | "cc" | "cxx" => Some("C++"),
                        "c" => Some("C"),
                        "cs" => Some("C#"),
                        _ => None,
                    };

                    if let Some(lang) = language {
                        if !repo_info.languages.contains(&lang.to_string()) {
                            repo_info.languages.push(lang.to_string());
                        }
                    }
                }

                // Categorize important files
                match file_name {
                    "main.rs" | "lib.rs" | "mod.rs" | "main.py" | "app.py" | "index.js"
                    | "main.js" => {
                        repo_info.main_files.push(file_name.to_string());
                    }
                    "Cargo.toml" | "package.json" | "requirements.txt" | "go.mod" | "pom.xml" => {
                        repo_info.config_files.push(file_name.to_string());
                    }
                    "Dockerfile" | "docker-compose.yml" | "Makefile" | "build.sh"
                    | "install.sh" => {
                        repo_info.setup_files.push(file_name.to_string());
                    }
                    name if name.contains("api")
                        || name.contains("server")
                        || name.contains("handler") =>
                    {
                        repo_info.api_files.push(file_name.to_string());
                        repo_info.has_api = true;
                    }
                    _ => {}
                }
            }
        }

        Ok(repo_info)
    }

    /// Generate overview content
    fn generate_overview_content(&self, repo_info: &RepositoryInfo) -> String {
        let mut content = String::new();

        // Add README content if available
        if let Some(readme) = &repo_info.readme_content {
            content.push_str("## Project Description\n\n");
            // Take first few paragraphs from README
            let lines: Vec<&str> = readme.lines().take(10).collect();
            content.push_str(&lines.join("\n"));
            content.push_str("\n\n");
        }

        // Add languages section
        if !repo_info.languages.is_empty() {
            content.push_str("## Technologies Used\n\n");
            for lang in &repo_info.languages {
                content.push_str(&format!("- {}\n", lang));
            }
            content.push_str("\n");
        }

        // Add key files section
        if !repo_info.main_files.is_empty() {
            content.push_str("## Key Files\n\n");
            for file in &repo_info.main_files {
                content.push_str(&format!("- `{}`\n", file));
            }
            content.push_str("\n");
        }

        if content.is_empty() {
            content = format!(
                "This is the main overview page for the {} project.\n\n\
                 This documentation provides comprehensive information about the project structure, \
                 setup instructions, and technical details.\n",
                repo_info.title
            );
        }

        content
    }

    /// Generate architecture content
    fn generate_architecture_content(&self, repo_info: &RepositoryInfo) -> String {
        let mut content = String::new();

        content.push_str("## System Architecture\n\n");
        content.push_str(
            "This section describes the overall architecture and design of the system.\n\n",
        );

        // Add languages and technologies
        if !repo_info.languages.is_empty() {
            content.push_str("### Technology Stack\n\n");
            for lang in &repo_info.languages {
                content.push_str(&format!("- **{}**: Core implementation language\n", lang));
            }
            content.push_str("\n");
        }

        // Add configuration files
        if !repo_info.config_files.is_empty() {
            content.push_str("### Configuration Files\n\n");
            for file in &repo_info.config_files {
                content.push_str(&format!("- `{}`: Project configuration\n", file));
            }
            content.push_str("\n");
        }

        // Add API information
        if repo_info.has_api {
            content.push_str("### API Architecture\n\n");
            content.push_str("This project includes API components:\n\n");
            for file in &repo_info.api_files {
                content.push_str(&format!("- `{}`\n", file));
            }
            content.push_str("\n");
        }

        content.push_str("### Project Structure\n\n");
        content.push_str("```\n");
        content.push_str("project/\n");
        content.push_str("├── src/          # Source code\n");
        content.push_str("├── docs/         # Documentation\n");
        content.push_str("├── tests/        # Test files\n");
        content.push_str("└── README.md     # Project overview\n");
        content.push_str("```\n\n");

        content
    }

    /// Generate getting started content
    fn generate_getting_started_content(&self, repo_info: &RepositoryInfo) -> String {
        let mut content = String::new();

        content.push_str("## Prerequisites\n\n");

        // Add language-specific prerequisites
        for lang in &repo_info.languages {
            match lang.as_str() {
                "Rust" => {
                    content.push_str("- [Rust](https://rustup.rs/) (latest stable version)\n");
                    content.push_str("- Cargo (comes with Rust)\n");
                }
                "Python" => {
                    content.push_str("- [Python](https://python.org/) 3.8 or higher\n");
                    content.push_str("- pip (Python package manager)\n");
                }
                "JavaScript" | "TypeScript" => {
                    content.push_str("- [Node.js](https://nodejs.org/) (LTS version)\n");
                    content.push_str("- npm or yarn\n");
                }
                "Go" => {
                    content.push_str("- [Go](https://golang.org/) 1.19 or higher\n");
                }
                _ => {}
            }
        }
        content.push_str("\n");

        content.push_str("## Installation\n\n");
        content.push_str("1. Clone the repository:\n");
        content.push_str("   ```bash\n");
        content.push_str("   git clone <repository-url>\n");
        content.push_str("   cd <project-directory>\n");
        content.push_str("   ```\n\n");

        // Add language-specific installation steps
        if repo_info.languages.contains(&"Rust".to_string()) {
            content.push_str("2. Build the project:\n");
            content.push_str("   ```bash\n");
            content.push_str("   cargo build\n");
            content.push_str("   ```\n\n");

            content.push_str("3. Run the project:\n");
            content.push_str("   ```bash\n");
            content.push_str("   cargo run\n");
            content.push_str("   ```\n\n");
        } else if repo_info.languages.contains(&"Python".to_string()) {
            content.push_str("2. Install dependencies:\n");
            content.push_str("   ```bash\n");
            content.push_str("   pip install -r requirements.txt\n");
            content.push_str("   ```\n\n");

            content.push_str("3. Run the project:\n");
            content.push_str("   ```bash\n");
            content.push_str("   python main.py\n");
            content.push_str("   ```\n\n");
        } else if repo_info.languages.contains(&"JavaScript".to_string())
            || repo_info.languages.contains(&"TypeScript".to_string())
        {
            content.push_str("2. Install dependencies:\n");
            content.push_str("   ```bash\n");
            content.push_str("   npm install\n");
            content.push_str("   ```\n\n");

            content.push_str("3. Run the project:\n");
            content.push_str("   ```bash\n");
            content.push_str("   npm start\n");
            content.push_str("   ```\n\n");
        }

        content.push_str("## Development\n\n");
        content.push_str(
            "For development setup and contribution guidelines, see the project README.\n\n",
        );

        content
    }

    /// Generate API content
    fn generate_api_content(&self, repo_info: &RepositoryInfo) -> String {
        let mut content = String::new();

        content.push_str("## API Documentation\n\n");
        content.push_str("This section provides detailed API documentation.\n\n");

        content.push_str("### API Files\n\n");
        for file in &repo_info.api_files {
            content.push_str(&format!("- `{}`\n", file));
        }
        content.push_str("\n");

        content.push_str("### Endpoints\n\n");
        content.push_str("*API endpoints will be documented here based on code analysis.*\n\n");

        content.push_str("### Authentication\n\n");
        content.push_str("*Authentication methods will be documented here.*\n\n");

        content.push_str("### Examples\n\n");
        content.push_str("*Usage examples will be provided here.*\n\n");

        content
    }
}
