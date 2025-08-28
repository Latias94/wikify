//! Wiki export functionality
//!
//! This module handles exporting wiki structures to various formats.

use crate::types::{WikiPage, WikiStructure};
use serde_json;
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};
use wikify_core::{ErrorContext, WikifyError, WikifyResult};

/// Export formats supported by the wiki exporter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Export as individual Markdown files
    Markdown,
    /// Export as a single JSON file
    Json,
    /// Export as HTML files
    Html,
    /// Export as a single PDF file
    Pdf,
}

/// Wiki exporter that handles different output formats
pub struct WikiExporter {
    // Future: could add template engines, styling options, etc.
}

impl WikiExporter {
    /// Create a new WikiExporter instance
    pub fn new() -> WikifyResult<Self> {
        Ok(Self {})
    }

    /// Export a wiki structure to the specified format
    pub async fn export(
        &self,
        wiki: &WikiStructure,
        format: ExportFormat,
        output_path: &str,
    ) -> WikifyResult<()> {
        let output_path = Path::new(output_path);

        match format {
            ExportFormat::Markdown => self.export_markdown(wiki, output_path).await,
            ExportFormat::Json => self.export_json(wiki, output_path).await,
            ExportFormat::Html => self.export_html(wiki, output_path).await,
            ExportFormat::Pdf => self.export_pdf(wiki, output_path).await,
        }
    }

    /// Export wiki as Markdown files
    async fn export_markdown(&self, wiki: &WikiStructure, output_path: &Path) -> WikifyResult<()> {
        info!("Exporting wiki as Markdown to: {:?}", output_path);

        // Create output directory
        fs::create_dir_all(output_path).await?;

        // Create index file
        let index_content = self.generate_markdown_index(wiki);
        let index_path = output_path.join("README.md");
        fs::write(&index_path, index_content).await?;

        // Export each page as a separate Markdown file
        for page in &wiki.pages {
            let filename = self.sanitize_filename(&format!("{}.md", page.title));
            let page_path = output_path.join(&filename);

            let page_content = self.generate_markdown_page(page, wiki);
            fs::write(&page_path, page_content).await?;

            debug!("Exported page: {} -> {:?}", page.title, page_path);
        }

        // Create a table of contents file
        let toc_content = self.generate_table_of_contents(wiki);
        let toc_path = output_path.join("TABLE_OF_CONTENTS.md");
        fs::write(&toc_path, toc_content).await?;

        info!(
            "Successfully exported {} pages as Markdown",
            wiki.pages.len()
        );
        Ok(())
    }

    /// Export wiki as JSON
    async fn export_json(&self, wiki: &WikiStructure, output_path: &Path) -> WikifyResult<()> {
        info!("Exporting wiki as JSON to: {:?}", output_path);

        let json_content = serde_json::to_string_pretty(wiki).map_err(|e| WikifyError::Config {
            message: format!("Failed to serialize wiki to JSON: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("wiki_export"),
        })?;

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(output_path, json_content).await?;

        info!("Successfully exported wiki as JSON");
        Ok(())
    }

    /// Export wiki as HTML files
    async fn export_html(&self, wiki: &WikiStructure, output_path: &Path) -> WikifyResult<()> {
        info!("Exporting wiki as HTML to: {:?}", output_path);

        // Create output directory
        fs::create_dir_all(output_path).await?;

        // Create CSS file
        let css_content = self.generate_css();
        let css_path = output_path.join("style.css");
        fs::write(&css_path, css_content).await?;

        // Create index HTML
        let index_html = self.generate_html_index(wiki);
        let index_path = output_path.join("index.html");
        fs::write(&index_path, index_html).await?;

        // Export each page as HTML
        for page in &wiki.pages {
            let filename = self.sanitize_filename(&format!("{}.html", page.title));
            let page_path = output_path.join(&filename);

            let page_html = self.generate_html_page(page, wiki);
            fs::write(&page_path, page_html).await?;

            debug!("Exported HTML page: {} -> {:?}", page.title, page_path);
        }

        info!("Successfully exported {} pages as HTML", wiki.pages.len());
        Ok(())
    }

    /// Export wiki as PDF (placeholder implementation)
    async fn export_pdf(&self, _wiki: &WikiStructure, _output_path: &Path) -> WikifyResult<()> {
        // TODO: Implement PDF export using a library like wkhtmltopdf or headless Chrome
        Err(WikifyError::Config {
            message: "PDF export is not yet implemented".to_string(),
            source: None,
            context: ErrorContext::new("wiki_export")
                .with_suggestion("Use Markdown or HTML export instead"),
        })
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
    fn generate_markdown_page(&self, page: &WikiPage, _wiki: &WikiStructure) -> String {
        let mut content = page.content.clone();

        // Add navigation links at the top
        content = format!("[← Back to Index](README.md)\n\n{}", content);

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

    /// Generate HTML index page
    fn generate_html_index(&self, wiki: &WikiStructure) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <div class="container">
        <header>
            <h1>{}</h1>
            <p class="description">{}</p>
        </header>
        
        <main>
            <h2>Pages</h2>
            <div class="page-grid">
                {}
            </div>
        </main>
        
        <footer>
            <p>Generated by Wikify on {}</p>
        </footer>
    </div>
</body>
</html>"#,
            wiki.title,
            wiki.title,
            wiki.description,
            wiki.pages
                .iter()
                .map(|page| {
                    let filename = self.sanitize_filename(&format!("{}.html", page.title));
                    format!(
                        r#"<div class="page-card">
                            <h3><a href="{}">{}</a></h3>
                            <p>{}</p>
                            <span class="importance {:?}">{:?}</span>
                        </div>"#,
                        filename, page.title, page.description, page.importance, page.importance
                    )
                })
                .collect::<Vec<_>>()
                .join("\n                "),
            wiki.metadata.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    /// Generate HTML content for a single page
    fn generate_html_page(&self, page: &WikiPage, wiki: &WikiStructure) -> String {
        // Convert Markdown to HTML (simplified)
        let html_content = self.markdown_to_html(&page.content);

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - {}</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <div class="container">
        <nav>
            <a href="index.html">← Back to Index</a>
        </nav>
        
        <main>
            {}
        </main>
        
        <footer>
            <p>Generated by Wikify on {}</p>
        </footer>
    </div>
</body>
</html>"#,
            page.title,
            wiki.title,
            html_content,
            wiki.metadata.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    /// Generate CSS for HTML export
    fn generate_css(&self) -> String {
        r#"
body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    line-height: 1.6;
    color: #333;
    margin: 0;
    padding: 0;
    background-color: #f5f5f5;
}

.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
    background-color: white;
    box-shadow: 0 0 10px rgba(0,0,0,0.1);
    min-height: 100vh;
}

header {
    text-align: center;
    margin-bottom: 40px;
    padding-bottom: 20px;
    border-bottom: 2px solid #eee;
}

h1 {
    color: #2c3e50;
    margin-bottom: 10px;
}

.description {
    font-size: 1.2em;
    color: #666;
    margin: 0;
}

.page-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 20px;
    margin-top: 20px;
}

.page-card {
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 20px;
    background-color: #fafafa;
    transition: transform 0.2s;
}

.page-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0,0,0,0.1);
}

.page-card h3 {
    margin-top: 0;
    margin-bottom: 10px;
}

.page-card a {
    text-decoration: none;
    color: #3498db;
}

.page-card a:hover {
    text-decoration: underline;
}

.importance {
    display: inline-block;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 0.8em;
    font-weight: bold;
    text-transform: uppercase;
}

.importance.High {
    background-color: #e74c3c;
    color: white;
}

.importance.Medium {
    background-color: #f39c12;
    color: white;
}

.importance.Low {
    background-color: #95a5a6;
    color: white;
}

nav {
    margin-bottom: 20px;
}

nav a {
    text-decoration: none;
    color: #3498db;
    font-weight: bold;
}

nav a:hover {
    text-decoration: underline;
}

footer {
    margin-top: 40px;
    padding-top: 20px;
    border-top: 1px solid #eee;
    text-align: center;
    color: #666;
    font-size: 0.9em;
}

pre {
    background-color: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 15px;
    overflow-x: auto;
}

code {
    background-color: #f8f8f8;
    padding: 2px 4px;
    border-radius: 3px;
    font-family: 'Monaco', 'Consolas', monospace;
}

blockquote {
    border-left: 4px solid #3498db;
    margin: 0;
    padding-left: 20px;
    color: #666;
}
"#
        .to_string()
    }

    /// Simple Markdown to HTML conversion (placeholder)
    fn markdown_to_html(&self, markdown: &str) -> String {
        // This is a very basic implementation
        // In a real implementation, you'd use a proper Markdown parser like pulldown-cmark
        markdown
            .lines()
            .map(|line| {
                if line.starts_with("# ") {
                    format!("<h1>{}</h1>", &line[2..])
                } else if line.starts_with("## ") {
                    format!("<h2>{}</h2>", &line[3..])
                } else if line.starts_with("### ") {
                    format!("<h3>{}</h3>", &line[4..])
                } else if line.trim().is_empty() {
                    "<br>".to_string()
                } else {
                    format!("<p>{}</p>", line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
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
}

impl Default for WikiExporter {
    fn default() -> Self {
        Self::new().expect("Failed to create WikiExporter")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ImportanceLevel, WikiPage, WikiStructure};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_markdown_export() {
        let exporter = WikiExporter::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let mut wiki = WikiStructure::new(
            "Test Wiki".to_string(),
            "A test wiki".to_string(),
            "/test/repo".to_string(),
        );

        let page = WikiPage {
            id: "test-page".to_string(),
            title: "Test Page".to_string(),
            content: "# Test Page\n\nThis is a test page.".to_string(),
            description: "A test page".to_string(),
            importance: ImportanceLevel::High,
            file_paths: vec![],
            related_pages: vec![],
            parent_section: None,
            tags: vec![],
            reading_time: 1,
            generated_at: chrono::Utc::now(),
            source_documents: vec![],
        };

        wiki.pages.push(page);

        let result = exporter
            .export(
                &wiki,
                ExportFormat::Markdown,
                temp_dir.path().to_str().unwrap(),
            )
            .await;
        assert!(result.is_ok());

        // Check that files were created
        assert!(temp_dir.path().join("README.md").exists());
        assert!(temp_dir.path().join("Test Page.md").exists());
    }

    #[test]
    fn test_filename_sanitization() {
        let exporter = WikiExporter::new().unwrap();

        assert_eq!(
            exporter.sanitize_filename("normal-file.md"),
            "normal-file.md"
        );
        assert_eq!(
            exporter.sanitize_filename("file/with\\bad:chars*.md"),
            "file_with_bad_chars_.md"
        );
        assert_eq!(
            exporter.sanitize_filename("file<with>more|bad\"chars?.md"),
            "file_with_more_bad_chars_.md"
        );
    }
}
