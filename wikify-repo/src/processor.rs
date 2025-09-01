//! Repository Processor - Clean, Modern Interface
//!
//! This module provides a interface for accessing repositories through three modes:
//! - API: Fast, minimal storage, requires network
//! - GitClone: Complete, offline capable, more storage  
//! - LocalDirectory: Direct local access, immediate

use crate::api::{ApiClientConfig, ApiClientFactory, RepositoryApiClient};
use glob::Pattern;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info, warn};
use url::Url;
use wikify_core::{
    ErrorContext, RepoAccessMode, RepoInfo, RepoType, RepositoryAccess, RepositoryAccessConfig,
    RepositoryFile, WikifyError, WikifyResult,
};

/// Simple file filter configuration
#[derive(Debug, Clone)]
pub struct FileFilterConfig {
    /// 是否使用 .gitignore 文件（默认 true）
    pub use_gitignore: bool,
    /// 额外的忽略模式（glob 格式）
    pub additional_patterns: Vec<String>,
    /// 编译后的 glob 模式（内部使用）
    compiled_patterns: Vec<Pattern>,
}

impl FileFilterConfig {
    /// 创建新的过滤配置
    pub fn new(use_gitignore: bool, additional_patterns: Vec<String>) -> WikifyResult<Self> {
        let mut compiled_patterns = Vec::new();

        for pattern in &additional_patterns {
            match Pattern::new(pattern) {
                Ok(compiled) => compiled_patterns.push(compiled),
                Err(e) => {
                    return Err(Box::new(WikifyError::Repository {
                        message: format!("Invalid glob pattern '{}': {}", pattern, e),
                        source: Some(Box::new(e)),
                        context: ErrorContext::new("file_filter_config")
                            .with_operation("compile_patterns"),
                    }));
                }
            }
        }

        Ok(Self {
            use_gitignore,
            additional_patterns,
            compiled_patterns,
        })
    }

    /// 添加新的 glob 模式
    pub fn add_pattern(&mut self, pattern: &str) -> WikifyResult<()> {
        match Pattern::new(pattern) {
            Ok(compiled) => {
                self.additional_patterns.push(pattern.to_string());
                self.compiled_patterns.push(compiled);
                Ok(())
            }
            Err(e) => Err(Box::new(WikifyError::Repository {
                message: format!("Invalid glob pattern '{}': {}", pattern, e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("file_filter_config").with_operation("add_pattern"),
            })),
        }
    }
}

impl Default for FileFilterConfig {
    fn default() -> Self {
        Self {
            use_gitignore: true,
            additional_patterns: vec![],
            compiled_patterns: vec![],
        }
    }
}

/// Repository processor - the main entry point for all repository operations
#[derive(Debug)]
pub struct RepositoryProcessor {
    /// Base directory for storing cloned repositories and temporary files
    base_path: PathBuf,
    /// File filtering configuration
    filter_config: FileFilterConfig,
}

impl RepositoryProcessor {
    /// Create a new repository processor with default filter config
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            filter_config: FileFilterConfig::default(),
        }
    }

    /// Create a new repository processor with custom filter config
    pub fn with_filter_config<P: AsRef<Path>>(
        base_path: P,
        filter_config: FileFilterConfig,
    ) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            filter_config,
        }
    }

    /// Create a new repository processor with glob patterns
    pub fn with_patterns<P: AsRef<Path>>(
        base_path: P,
        use_gitignore: bool,
        patterns: Vec<String>,
    ) -> WikifyResult<Self> {
        let filter_config = FileFilterConfig::new(use_gitignore, patterns)?;
        Ok(Self {
            base_path: base_path.as_ref().to_path_buf(),
            filter_config,
        })
    }

    /// Main entry point: Access a repository with automatic mode detection
    ///
    /// This is the primary method that handles all repository access logic:
    /// 1. Parse and validate the URL/path
    /// 2. Determine the optimal access mode
    /// 3. Set up the repository access
    /// 4. Return a ready-to-use RepositoryAccess handle
    pub async fn access_repository(
        &self,
        url_or_path: &str,
        config: Option<RepositoryAccessConfig>,
    ) -> WikifyResult<RepositoryAccess> {
        let config = config.unwrap_or_default();

        info!(
            url_or_path = %url_or_path,
            config = ?config,
            "🚀 Starting repository access"
        );

        // Step 1: Determine access mode
        let access_mode = self.determine_access_mode(url_or_path, &config);

        info!(
            url_or_path = %url_or_path,
            access_mode = ?access_mode,
            "🎯 Determined access mode"
        );

        // Step 2: Parse repository information
        let repo_info = self.parse_repository_info(url_or_path, access_mode.clone())?;

        // Step 3: Set up repository access based on mode
        let repository_access = match access_mode {
            RepoAccessMode::Api => self.setup_api_access(&repo_info, &config).await?,
            RepoAccessMode::GitClone => self.setup_git_clone_access(&repo_info, &config).await?,
            RepoAccessMode::LocalDirectory => self.setup_local_directory_access(&repo_info).await?,
        };

        info!(
            url_or_path = %url_or_path,
            access_mode = ?access_mode,
            is_ready = repository_access.is_ready,
            "✅ Repository access ready"
        );

        Ok(repository_access)
    }

    /// Determine the optimal access mode based on URL and configuration
    fn determine_access_mode(
        &self,
        url_or_path: &str,
        config: &RepositoryAccessConfig,
    ) -> RepoAccessMode {
        // 1. Check for forced mode
        if config.force_mode {
            if let Some(preferred) = &config.preferred_mode {
                debug!(
                    url_or_path = %url_or_path,
                    forced_mode = ?preferred,
                    "🔒 Using forced access mode"
                );
                return preferred.clone();
            }
        }

        // 2. Check for explicit preference
        if let Some(preferred) = &config.preferred_mode {
            debug!(
                url_or_path = %url_or_path,
                preferred_mode = ?preferred,
                "👍 Using preferred access mode"
            );
            return preferred.clone();
        }

        // 3. Auto-detect based on URL pattern
        if !url_or_path.starts_with("http") {
            debug!(
                url_or_path = %url_or_path,
                "📁 Local path detected, using LocalDirectory mode"
            );
            return RepoAccessMode::LocalDirectory;
        }

        // 4. For remote URLs, check API token availability
        let repo_type = self.detect_repo_type(url_or_path);
        let has_api_access = self.check_api_token_availability(&repo_type, config);

        if has_api_access {
            debug!(
                url_or_path = %url_or_path,
                repo_type = ?repo_type,
                "🔗 API token available, using API mode"
            );
            RepoAccessMode::Api
        } else {
            debug!(
                url_or_path = %url_or_path,
                repo_type = ?repo_type,
                "📥 No API token, using GitClone mode"
            );
            RepoAccessMode::GitClone
        }
    }

    /// Detect repository type from URL
    fn detect_repo_type(&self, url: &str) -> RepoType {
        if url.contains("github.com") {
            RepoType::GitHub
        } else if url.contains("gitlab.com") || url.contains("gitlab") {
            RepoType::GitLab
        } else if url.contains("bitbucket.org") {
            RepoType::Bitbucket
        } else if url.contains("gitea") {
            RepoType::Gitea
        } else {
            RepoType::Local
        }
    }

    /// Check if API token is available for the repository type
    fn check_api_token_availability(
        &self,
        repo_type: &RepoType,
        config: &RepositoryAccessConfig,
    ) -> bool {
        // Check config first
        if config.api_token.is_some() {
            return true;
        }

        // Check environment variables
        match repo_type {
            RepoType::GitHub => std::env::var("GITHUB_TOKEN").is_ok(),
            RepoType::GitLab => std::env::var("GITLAB_TOKEN").is_ok(),
            RepoType::Bitbucket => std::env::var("BITBUCKET_TOKEN").is_ok(),
            RepoType::Gitea => {
                std::env::var("GITEA_TOKEN").is_ok() && std::env::var("GITEA_BASE_URL").is_ok()
            }
            RepoType::Local => false,
        }
    }

    /// Parse repository information from URL or path
    fn parse_repository_info(
        &self,
        url_or_path: &str,
        access_mode: RepoAccessMode,
    ) -> WikifyResult<RepoInfo> {
        if access_mode == RepoAccessMode::LocalDirectory {
            // Handle local directory
            let path = Path::new(url_or_path);
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            return Ok(RepoInfo {
                owner: "local".to_string(),
                name,
                repo_type: RepoType::Local,
                url: url_or_path.to_string(),
                access_token: None,
                local_path: Some(url_or_path.to_string()),
                access_mode,
            });
        }

        // Parse remote URL
        let parsed_url = Url::parse(url_or_path).map_err(|e| WikifyError::Repository {
            message: format!("Invalid repository URL: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("repository_processor")
                .with_operation("parse_repository_info")
                .with_suggestion("Ensure the URL is valid and properly formatted"),
        })?;

        let path_segments: Vec<&str> = parsed_url
            .path_segments()
            .ok_or_else(|| WikifyError::Repository {
                message: "URL path cannot be parsed".to_string(),
                source: None,
                context: ErrorContext::new("repository_processor")
                    .with_operation("parse_repository_info"),
            })?
            .collect();

        if path_segments.len() < 2 {
            return Err(Box::new(WikifyError::Repository {
                message: "URL must contain owner and repository name".to_string(),
                source: None,
                context: ErrorContext::new("repository_processor")
                    .with_operation("parse_repository_info")
                    .with_suggestion("URL should be in format: https://platform.com/owner/repo"),
            }));
        }

        let owner = path_segments[0].to_string();
        let name = path_segments[1].trim_end_matches(".git").to_string();
        let repo_type = self.detect_repo_type(url_or_path);

        Ok(RepoInfo {
            owner,
            name,
            repo_type,
            url: url_or_path.to_string(),
            access_token: None,
            local_path: None,
            access_mode,
        })
    }

    /// Set up API access mode
    async fn setup_api_access(
        &self,
        repo_info: &RepoInfo,
        config: &RepositoryAccessConfig,
    ) -> WikifyResult<RepositoryAccess> {
        debug!(
            repo_url = %repo_info.url,
            "🔗 Setting up API access"
        );

        // API access doesn't require local setup, just validate credentials
        let has_token = config.api_token.is_some()
            || self.check_api_token_availability(&repo_info.repo_type, config);

        if !has_token {
            warn!(
                repo_url = %repo_info.url,
                repo_type = ?repo_info.repo_type,
                "⚠️ No API token available for API access mode"
            );
        }

        Ok(RepositoryAccess {
            repo_info: repo_info.clone(),
            access_mode: RepoAccessMode::Api,
            local_path: None,
            is_ready: true, // API access is always ready if we have tokens
        })
    }

    /// Set up Git clone access mode
    async fn setup_git_clone_access(
        &self,
        repo_info: &RepoInfo,
        config: &RepositoryAccessConfig,
    ) -> WikifyResult<RepositoryAccess> {
        debug!(
            repo_url = %repo_info.url,
            "📥 Setting up Git clone access"
        );

        // Determine local path (like DeepWiki: ~/.wikify/repos/{owner}_{repo_name})
        let local_path = if let Some(custom_path) = &config.custom_local_path {
            PathBuf::from(custom_path)
        } else {
            self.get_default_clone_path(repo_info)
        };

        // Check if repository already exists and is valid
        let is_ready = if local_path.exists() && self.is_valid_git_repository(&local_path).await? {
            info!(
                repo_url = %repo_info.url,
                local_path = %local_path.display(),
                "📁 Repository already cloned, using existing"
            );
            true
        } else {
            // Clone the repository
            self.clone_repository(repo_info, &local_path, config)
                .await?;
            true
        };

        Ok(RepositoryAccess {
            repo_info: repo_info.clone(),
            access_mode: RepoAccessMode::GitClone,
            local_path: Some(local_path),
            is_ready,
        })
    }

    /// Set up local directory access mode
    async fn setup_local_directory_access(
        &self,
        repo_info: &RepoInfo,
    ) -> WikifyResult<RepositoryAccess> {
        debug!(
            local_path = %repo_info.url,
            "📁 Setting up local directory access"
        );

        let local_path = PathBuf::from(&repo_info.url);

        if !local_path.exists() {
            return Err(Box::new(WikifyError::Repository {
                message: format!("Local directory does not exist: {}", repo_info.url),
                source: None,
                context: ErrorContext::new("repository_processor")
                    .with_operation("setup_local_directory_access")
                    .with_suggestion("Ensure the directory path exists and is accessible"),
            }));
        }

        Ok(RepositoryAccess {
            repo_info: repo_info.clone(),
            access_mode: RepoAccessMode::LocalDirectory,
            local_path: Some(local_path),
            is_ready: true,
        })
    }

    /// Get default clone path like DeepWiki: ~/.wikify/repos/{owner}_{repo_name}
    fn get_default_clone_path(&self, repo_info: &RepoInfo) -> PathBuf {
        let repo_name = format!("{}_{}", repo_info.owner, repo_info.name);
        self.base_path.join("repos").join(repo_name)
    }

    /// Check if a directory contains a valid git repository
    async fn is_valid_git_repository(&self, path: &Path) -> WikifyResult<bool> {
        if !path.exists() {
            return Ok(false);
        }

        // Check if .git directory exists
        let git_dir = path.join(".git");
        if !git_dir.exists() {
            return Ok(false);
        }

        // Check if directory has files (not empty)
        let mut entries = tokio::fs::read_dir(path)
            .await
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to read directory: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("repository_processor")
                    .with_operation("is_valid_git_repository"),
            })?;

        let has_files = entries
            .next_entry()
            .await
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to check directory contents: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("repository_processor")
                    .with_operation("is_valid_git_repository"),
            })?
            .is_some();

        Ok(has_files)
    }

    /// Clone repository using git command (inspired by DeepWiki)
    async fn clone_repository(
        &self,
        repo_info: &RepoInfo,
        target_path: &Path,
        config: &RepositoryAccessConfig,
    ) -> WikifyResult<()> {
        info!(
            repo_url = %repo_info.url,
            target_path = %target_path.display(),
            "🚀 Starting repository clone"
        );

        // Ensure parent directory exists
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| WikifyError::Repository {
                    message: format!("Failed to create parent directory: {}", e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("repository_processor")
                        .with_operation("clone_repository"),
                })?;
        }

        // Build git clone command
        let mut cmd = Command::new("git");
        cmd.arg("clone");

        // Add depth option for shallow clone (like DeepWiki)
        if let Some(depth) = config.clone_depth {
            cmd.arg("--depth").arg(depth.to_string());
            cmd.arg("--single-branch");
        }

        // Prepare clone URL with authentication if needed
        let clone_url = self.prepare_authenticated_url(repo_info, config)?;
        cmd.arg(&clone_url).arg(target_path);

        // Execute clone command
        let output = cmd.output().await.map_err(|e| WikifyError::Repository {
            message: format!("Failed to execute git clone: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("repository_processor")
                .with_operation("clone_repository")
                .with_suggestion("Ensure git is installed and accessible"),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Box::new(WikifyError::Repository {
                message: format!("Git clone failed: {}", stderr),
                source: None,
                context: ErrorContext::new("repository_processor")
                    .with_operation("clone_repository")
                    .with_suggestion("Check repository URL and access permissions"),
            }));
        }

        info!(
            repo_url = %repo_info.url,
            target_path = %target_path.display(),
            "✅ Repository cloned successfully"
        );

        Ok(())
    }

    /// Prepare authenticated URL for cloning (like DeepWiki)
    fn prepare_authenticated_url(
        &self,
        repo_info: &RepoInfo,
        config: &RepositoryAccessConfig,
    ) -> WikifyResult<String> {
        let env_token = self.get_env_token(&repo_info.repo_type);
        let token = config.api_token.as_ref().or_else(|| env_token.as_ref());

        if let Some(token) = token {
            let parsed_url = Url::parse(&repo_info.url).map_err(|e| WikifyError::Repository {
                message: format!("Invalid repository URL: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("repository_processor")
                    .with_operation("prepare_authenticated_url"),
            })?;

            // Format authentication based on platform (like DeepWiki)
            let auth_url = match repo_info.repo_type {
                RepoType::GitHub => {
                    format!(
                        "https://{}@{}{}",
                        token,
                        parsed_url.host_str().unwrap(),
                        parsed_url.path()
                    )
                }
                RepoType::GitLab => {
                    format!(
                        "https://oauth2:{}@{}{}",
                        token,
                        parsed_url.host_str().unwrap(),
                        parsed_url.path()
                    )
                }
                RepoType::Bitbucket => {
                    format!(
                        "https://x-token-auth:{}@{}{}",
                        token,
                        parsed_url.host_str().unwrap(),
                        parsed_url.path()
                    )
                }
                RepoType::Gitea => {
                    format!(
                        "https://{}@{}{}",
                        token,
                        parsed_url.host_str().unwrap(),
                        parsed_url.path()
                    )
                }
                RepoType::Local => repo_info.url.clone(),
            };

            Ok(auth_url)
        } else {
            // No token, use original URL (for public repositories)
            Ok(repo_info.url.clone())
        }
    }

    /// Get environment token for repository type
    fn get_env_token(&self, repo_type: &RepoType) -> Option<String> {
        match repo_type {
            RepoType::GitHub => std::env::var("GITHUB_TOKEN").ok(),
            RepoType::GitLab => std::env::var("GITLAB_TOKEN").ok(),
            RepoType::Bitbucket => std::env::var("BITBUCKET_TOKEN").ok(),
            RepoType::Gitea => std::env::var("GITEA_TOKEN").ok(),
            RepoType::Local => None,
        }
    }

    // ============================================================================
    // File Operations - Unified interface for all access modes
    // ============================================================================

    /// Get file tree - unified interface that works with all access modes
    pub async fn get_file_tree(
        &self,
        access: &RepositoryAccess,
        branch: Option<&str>,
    ) -> WikifyResult<Vec<RepositoryFile>> {
        if !access.is_ready {
            return Err(Box::new(WikifyError::Repository {
                message: "Repository access is not ready".to_string(),
                source: None,
                context: ErrorContext::new("repository_processor")
                    .with_operation("get_file_tree")
                    .with_suggestion("Ensure repository access is properly initialized"),
            }));
        }

        match access.access_mode {
            RepoAccessMode::Api => self.get_file_tree_api(access, branch).await,
            RepoAccessMode::GitClone | RepoAccessMode::LocalDirectory => {
                self.get_file_tree_local(access, branch).await
            }
        }
    }

    /// Get file content - unified interface that works with all access modes
    pub async fn get_file_content(
        &self,
        access: &RepositoryAccess,
        file_path: &str,
        branch: Option<&str>,
    ) -> WikifyResult<String> {
        if !access.is_ready {
            return Err(Box::new(WikifyError::Repository {
                message: "Repository access is not ready".to_string(),
                source: None,
                context: ErrorContext::new("repository_processor")
                    .with_operation("get_file_content")
                    .with_suggestion("Ensure repository access is properly initialized"),
            }));
        }

        match access.access_mode {
            RepoAccessMode::Api => self.get_file_content_api(access, file_path, branch).await,
            RepoAccessMode::GitClone | RepoAccessMode::LocalDirectory => {
                self.get_file_content_local(access, file_path, branch).await
            }
        }
    }

    /// Get README content - unified interface
    pub async fn get_readme(
        &self,
        access: &RepositoryAccess,
        branch: Option<&str>,
    ) -> WikifyResult<Option<String>> {
        // Try common README file names
        let readme_names = [
            "README.md",
            "README.rst",
            "README.txt",
            "README",
            "readme.md",
        ];

        for readme_name in &readme_names {
            match self.get_file_content(access, readme_name, branch).await {
                Ok(content) => return Ok(Some(content)),
                Err(_) => continue, // Try next README name
            }
        }

        Ok(None) // No README found
    }

    // ============================================================================
    // API Mode Implementation
    // ============================================================================

    /// Get file tree using API
    async fn get_file_tree_api(
        &self,
        access: &RepositoryAccess,
        branch: Option<&str>,
    ) -> WikifyResult<Vec<RepositoryFile>> {
        let api_client = self.create_api_client(&access.repo_info)?;
        let files = api_client
            .get_file_tree(&access.repo_info.owner, &access.repo_info.name, branch)
            .await?;

        // Convert API response to our unified format
        Ok(files
            .into_iter()
            .map(|f| RepositoryFile {
                path: f.path,
                file_type: f.file_type,
                size: f.size,
                sha: f.sha,
                last_modified: None, // API might not provide this
            })
            .collect())
    }

    /// Get file content using API
    async fn get_file_content_api(
        &self,
        access: &RepositoryAccess,
        file_path: &str,
        branch: Option<&str>,
    ) -> WikifyResult<String> {
        let api_client = self.create_api_client(&access.repo_info)?;
        api_client
            .get_file_content(
                &access.repo_info.owner,
                &access.repo_info.name,
                file_path,
                branch,
            )
            .await
    }

    /// Create API client for the repository
    fn create_api_client(
        &self,
        repo_info: &RepoInfo,
    ) -> WikifyResult<Box<dyn RepositoryApiClient>> {
        let repo_type_str = match repo_info.repo_type {
            RepoType::GitHub => "github",
            RepoType::GitLab => "gitlab",
            RepoType::Bitbucket => "bitbucket",
            RepoType::Gitea => "gitea",
            RepoType::Local => {
                return Err(Box::new(WikifyError::Repository {
                    message: "Cannot create API client for local repository".to_string(),
                    source: None,
                    context: ErrorContext::new("repository_processor")
                        .with_operation("create_api_client"),
                }))
            }
        };

        let config = ApiClientConfig::default();
        ApiClientFactory::create_client(repo_type_str, config)
    }

    // ============================================================================
    // Local Mode Implementation (GitClone + LocalDirectory)
    // ============================================================================

    /// Get file tree from local repository/directory
    async fn get_file_tree_local(
        &self,
        access: &RepositoryAccess,
        _branch: Option<&str>, // TODO: Implement branch switching for git repos
    ) -> WikifyResult<Vec<RepositoryFile>> {
        let local_path = access
            .local_path
            .as_ref()
            .ok_or_else(|| WikifyError::Repository {
                message: "Local path not available for local access".to_string(),
                source: None,
                context: ErrorContext::new("repository_processor")
                    .with_operation("get_file_tree_local"),
            })?;

        let mut files = Vec::new();
        self.collect_files_iterative(local_path, &mut files).await?;
        Ok(files)
    }

    /// Get file content from local repository/directory
    async fn get_file_content_local(
        &self,
        access: &RepositoryAccess,
        file_path: &str,
        _branch: Option<&str>, // TODO: Implement branch switching for git repos
    ) -> WikifyResult<String> {
        let local_path = access
            .local_path
            .as_ref()
            .ok_or_else(|| WikifyError::Repository {
                message: "Local path not available for local access".to_string(),
                source: None,
                context: ErrorContext::new("repository_processor")
                    .with_operation("get_file_content_local"),
            })?;

        let file_full_path = local_path.join(file_path);

        if !file_full_path.exists() {
            return Err(Box::new(WikifyError::Repository {
                message: format!("File not found: {}", file_path),
                source: None,
                context: ErrorContext::new("repository_processor")
                    .with_operation("get_file_content_local")
                    .with_suggestion("Check if the file path is correct"),
            }));
        }

        let content = tokio::fs::read_to_string(&file_full_path)
            .await
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to read file {}: {}", file_path, e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("repository_processor")
                    .with_operation("get_file_content_local"),
            })?;

        Ok(content)
    }

    /// Collect files using ignore library (respects .gitignore)
    async fn collect_files_iterative(
        &self,
        root_path: &Path,
        files: &mut Vec<RepositoryFile>,
    ) -> WikifyResult<()> {
        // Build walker with gitignore support
        let mut builder = WalkBuilder::new(root_path);

        // Configure gitignore usage
        builder.git_ignore(self.filter_config.use_gitignore);
        builder.git_exclude(self.filter_config.use_gitignore);
        builder.git_global(self.filter_config.use_gitignore);

        // Note: .git directory is automatically ignored by the ignore library

        let walker = builder.build();

        for result in walker {
            match result {
                Ok(entry) => {
                    let path = entry.path();

                    // Skip directories
                    if path.is_dir() {
                        continue;
                    }

                    // Apply additional custom patterns
                    if self.should_skip_by_additional_patterns(&path) {
                        continue;
                    }

                    // Get file metadata
                    let metadata = match tokio::fs::metadata(&path).await {
                        Ok(meta) => meta,
                        Err(_) => continue, // Skip files we can't read
                    };

                    // Get relative path from root
                    let relative_path = match path.strip_prefix(root_path) {
                        Ok(rel_path) => rel_path,
                        Err(_) => continue, // Skip if we can't get relative path
                    };

                    files.push(RepositoryFile {
                        path: relative_path.to_string_lossy().to_string(),
                        file_type: "blob".to_string(),
                        size: Some(metadata.len()),
                        sha: None,           // We don't calculate SHA for local files
                        last_modified: None, // TODO: Convert SystemTime to DateTime<Utc>
                    });
                }
                Err(_) => {
                    // Skip entries we can't process
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Check if file should be skipped by additional glob patterns
    fn should_skip_by_additional_patterns(&self, path: &Path) -> bool {
        if self.filter_config.compiled_patterns.is_empty() {
            return false;
        }

        // Convert path to string for pattern matching
        let path_str = path.to_string_lossy();

        // Also try just the filename for patterns like "*.tmp"
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        for pattern in &self.filter_config.compiled_patterns {
            // Check against full path
            if pattern.matches(&path_str) {
                return true;
            }

            // Check against filename only
            if pattern.matches(filename) {
                return true;
            }

            // For Unix-style paths, also check with forward slashes
            if cfg!(windows) {
                let unix_path = path_str.replace('\\', "/");
                if pattern.matches(&unix_path) {
                    return true;
                }
            }
        }

        false
    }
}
