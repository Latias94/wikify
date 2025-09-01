//! Document processor for converting repository files to cheungfun Documents
//!
//! This module handles the conversion of repository files into cheungfun Document
//! objects with appropriate metadata and content processing.

use cheungfun_core::Document;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;
use wikify_core::{ErrorContext, WikifyError, WikifyResult};

/// Document processor that converts repository files to cheungfun Documents
pub struct DocumentProcessor {
    /// Base path for the repository
    base_path: PathBuf,
    /// File filters
    included_extensions: Vec<String>,
    excluded_dirs: Vec<String>,
    excluded_files: Vec<String>,
}

impl DocumentProcessor {
    /// Create a new document processor
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            included_extensions: vec![
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "java".to_string(),
                "cpp".to_string(),
                "c".to_string(),
                "h".to_string(),
                "hpp".to_string(),
                "cs".to_string(),
                "go".to_string(),
                "php".to_string(),
                "rb".to_string(),
                "swift".to_string(),
                "kt".to_string(),
                "scala".to_string(),
                "md".to_string(),
                "txt".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
                "toml".to_string(),
                "xml".to_string(),
            ],
            excluded_dirs: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
                "__pycache__".to_string(),
                ".pytest_cache".to_string(),
                "coverage".to_string(),
                ".coverage".to_string(),
                "htmlcov".to_string(),
            ],
            excluded_files: vec![
                "*.lock".to_string(),
                "*.log".to_string(),
                "*.tmp".to_string(),
                "*.cache".to_string(),
                "*.pyc".to_string(),
                "*.pyo".to_string(),
                "*.pyd".to_string(),
                "*.so".to_string(),
                "*.dll".to_string(),
                "*.dylib".to_string(),
            ],
        }
    }

    /// Configure included file extensions
    pub fn with_included_extensions(mut self, extensions: Vec<String>) -> Self {
        self.included_extensions = extensions;
        self
    }

    /// Configure excluded directories
    pub fn with_excluded_dirs(mut self, dirs: Vec<String>) -> Self {
        self.excluded_dirs = dirs;
        self
    }

    /// Configure excluded files
    pub fn with_excluded_files(mut self, files: Vec<String>) -> Self {
        self.excluded_files = files;
        self
    }

    /// Process all documents in the repository
    pub async fn process_repository(&self) -> WikifyResult<Vec<Document>> {
        info!("Processing repository at {:?}", self.base_path);

        // Collect all eligible files
        let files = self.collect_files().await?;
        info!("Found {} eligible files", files.len());

        // Process files in batches to avoid overwhelming the system
        let mut documents = Vec::new();
        let batch_size = 50; // Process 50 files at a time

        for batch in files.chunks(batch_size) {
            let batch_docs = self.process_file_batch(batch).await?;
            documents.extend(batch_docs);
        }

        info!("Successfully processed {} documents", documents.len());
        Ok(documents)
    }

    /// Collect all eligible files from the repository
    async fn collect_files(&self) -> WikifyResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.base_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| self.should_traverse_entry(e))
        {
            let entry = entry.map_err(|e| WikifyError::Repository {
                message: format!("Failed to read directory entry: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("document_processor").with_operation("collect_files"),
            })?;

            if entry.file_type().is_file() && self.should_include_file(entry.path()) {
                files.push(entry.path().to_path_buf());
            }
        }

        Ok(files)
    }

    /// Process a batch of files
    async fn process_file_batch(&self, files: &[PathBuf]) -> WikifyResult<Vec<Document>> {
        let mut documents = Vec::new();

        for file_path in files {
            match self.process_single_file(file_path).await {
                Ok(Some(doc)) => documents.push(doc),
                Ok(None) => {
                    debug!("Skipped file: {:?}", file_path);
                }
                Err(e) => {
                    warn!("Failed to process file {:?}: {}", file_path, e);
                    // Continue processing other files instead of failing completely
                }
            }
        }

        Ok(documents)
    }

    /// Process a single file into a Document
    async fn process_single_file(&self, file_path: &Path) -> WikifyResult<Option<Document>> {
        debug!("Processing file: {:?}", file_path);

        // Read file content
        let content =
            tokio::fs::read_to_string(file_path)
                .await
                .map_err(|e| WikifyError::Repository {
                    message: format!("Failed to read file {:?}: {}", file_path, e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("document_processor")
                        .with_operation("read_file")
                        .with_metadata("file_path", &file_path.to_string_lossy()),
                })?;

        // Skip empty files
        if content.trim().is_empty() {
            return Ok(None);
        }

        // Create document with metadata
        let relative_path = file_path
            .strip_prefix(&self.base_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        let mut metadata = std::collections::HashMap::new();

        // Add file metadata
        metadata.insert(
            "file_path".to_string(),
            serde_json::Value::String(relative_path.clone()),
        );
        metadata.insert(
            "file_name".to_string(),
            serde_json::Value::String(
                file_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            ),
        );

        // Detect file type and language
        if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
            metadata.insert(
                "file_extension".to_string(),
                serde_json::Value::String(extension.to_string()),
            );

            let language = self.detect_language(extension);
            metadata.insert("language".to_string(), serde_json::Value::String(language));

            let file_type = self.classify_file_type(extension);
            metadata.insert(
                "file_type".to_string(),
                serde_json::Value::String(file_type),
            );
        }

        // Add file size
        if let Ok(file_metadata) = tokio::fs::metadata(file_path).await {
            metadata.insert(
                "file_size".to_string(),
                serde_json::Value::String(file_metadata.len().to_string()),
            );
        }

        // Create document with proper structure
        let document = Document {
            id: Uuid::new_v4(),
            content,
            metadata,
            embedding: None,
        };

        Ok(Some(document))
    }

    /// Check if we should traverse into a directory entry
    fn should_traverse_entry(&self, entry: &walkdir::DirEntry) -> bool {
        if entry.file_type().is_dir() {
            let dir_name = entry.file_name().to_string_lossy();
            !self
                .excluded_dirs
                .iter()
                .any(|excluded| *excluded == dir_name)
        } else {
            true
        }
    }

    /// Check if a file should be included in processing
    fn should_include_file(&self, file_path: &Path) -> bool {
        // Check file extension
        if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
            if !self.included_extensions.contains(&extension.to_string()) {
                return false;
            }
        } else {
            // Skip files without extensions unless they're known text files
            let file_name = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            let known_text_files = ["README", "LICENSE", "CHANGELOG", "Dockerfile", "Makefile"];
            if !known_text_files
                .iter()
                .any(|&name| file_name.starts_with(name))
            {
                return false;
            }
        }

        // Check excluded file patterns
        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        for pattern in &self.excluded_files {
            if let Some(suffix) = pattern.strip_prefix('*') {
                if file_name.ends_with(suffix) {
                    return false;
                }
            } else if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                if file_name.starts_with(prefix) {
                    return false;
                }
            } else if file_name == pattern {
                return false;
            }
        }

        true
    }

    /// Detect programming language from file extension
    fn detect_language(&self, extension: &str) -> String {
        match extension.to_lowercase().as_str() {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "java" => "java",
            "cpp" | "cc" | "cxx" => "cpp",
            "c" => "c",
            "h" | "hpp" => "c_header",
            "cs" => "csharp",
            "go" => "go",
            "php" => "php",
            "rb" => "ruby",
            "swift" => "swift",
            "kt" => "kotlin",
            "scala" => "scala",
            "md" => "markdown",
            "txt" => "text",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "xml" => "xml",
            _ => "unknown",
        }
        .to_string()
    }

    /// Classify file type based on extension
    fn classify_file_type(&self, extension: &str) -> String {
        match extension.to_lowercase().as_str() {
            "rs" | "py" | "js" | "ts" | "java" | "cpp" | "c" | "cs" | "go" | "php" | "rb"
            | "swift" | "kt" | "scala" => "code",
            "h" | "hpp" => "header",
            "md" | "txt" => "documentation",
            "json" | "yaml" | "yml" | "toml" | "xml" => "configuration",
            _ => "other",
        }
        .to_string()
    }
}
