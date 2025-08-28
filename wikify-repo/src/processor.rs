//! Repository processor implementation
//!
//! Handles cloning, updating, and basic processing of repositories
//! Uses system git command like DeepWiki for maximum compatibility

use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::info;
use url::Url;
use wikify_core::{ErrorContext, RepoInfo, RepoType, WikifyError, WikifyResult};

/// Repository processor that handles cloning and basic operations
pub struct RepositoryProcessor {
    base_path: PathBuf,
}

impl RepositoryProcessor {
    /// Create a new repository processor
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Clone or update a repository using system git command (like DeepWiki)
    pub async fn clone_repository(&self, repo_info: &RepoInfo) -> WikifyResult<String> {
        let repo_path = self.get_repo_path(repo_info);

        // Create base directory if it doesn't exist
        if let Some(parent) = repo_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| WikifyError::Repository {
                    message: format!("Failed to create directory: {}", e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("repository_processor")
                        .with_operation("create_directory")
                        .with_suggestion("Check directory permissions"),
                })?;
        }

        // Check if repository already exists and is not empty
        if repo_path.exists() && self.is_non_empty_dir(&repo_path).await? {
            info!(
                "Repository already exists at {:?}, using existing",
                repo_path
            );
            return Ok(repo_path.to_string_lossy().to_string());
        }

        match repo_info.repo_type {
            RepoType::Local => {
                // For local repositories, just validate the path
                let source_path = Path::new(&repo_info.url);
                if !source_path.exists() {
                    return Err(WikifyError::Repository {
                        message: format!("Local repository path does not exist: {}", repo_info.url),
                        source: None,
                        context: ErrorContext::new("repository_processor")
                            .with_operation("validate_local_path")
                            .with_suggestion("Check if the path exists and is accessible"),
                    });
                }
                Ok(repo_info.url.clone())
            }
            _ => {
                // Clone remote repository using system git
                self.clone_remote_repo(repo_info, &repo_path).await
            }
        }
    }

    /// Clone remote repository using system git command
    async fn clone_remote_repo(
        &self,
        repo_info: &RepoInfo,
        repo_path: &Path,
    ) -> WikifyResult<String> {
        // First check if git is available
        self.check_git_available().await?;

        // Prepare clone URL with authentication
        let clone_url = self.prepare_clone_url(repo_info)?;

        info!(
            "Cloning repository from {} to {:?}",
            repo_info.url, repo_path
        );

        // Ensure the target directory exists
        tokio::fs::create_dir_all(&repo_path)
            .await
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to create target directory: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("repository_processor")
                    .with_operation("create_target_directory"),
            })?;

        // Execute git clone with shallow clone for efficiency
        let output = Command::new("git")
            .args(&[
                "clone",
                "--depth=1",
                "--single-branch",
                &clone_url,
                repo_path.to_str().unwrap(),
            ])
            .output()
            .await
            .map_err(|e| WikifyError::Git {
                message: format!("Failed to execute git command: {}", e),
                context: ErrorContext::new("repository_processor")
                    .with_operation("git_clone")
                    .with_suggestion("Ensure git is installed and accessible"),
            })?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            // Sanitize error message to remove any tokens (like DeepWiki does)
            let sanitized_error = if let Some(token) = &repo_info.access_token {
                error_msg.replace(token, "***TOKEN***")
            } else {
                error_msg.to_string()
            };

            return Err(WikifyError::Git {
                message: format!("Git clone failed: {}", sanitized_error),
                context: ErrorContext::new("repository_processor")
                    .with_operation("git_clone")
                    .with_suggestion("Check repository URL and access permissions")
                    .with_suggestion("Verify network connectivity"),
            });
        }

        info!("Repository cloned successfully");
        Ok(repo_path.to_string_lossy().to_string())
    }

    /// Check if git is available on the system
    async fn check_git_available(&self) -> WikifyResult<()> {
        let output = Command::new("git")
            .arg("--version")
            .output()
            .await
            .map_err(|e| WikifyError::Git {
                message: format!("Git command not found: {}", e),
                context: ErrorContext::new("repository_processor")
                    .with_operation("check_git")
                    .with_suggestion("Install git and ensure it's in your PATH"),
            })?;

        if !output.status.success() {
            return Err(WikifyError::Git {
                message: "Git command failed".to_string(),
                context: ErrorContext::new("repository_processor")
                    .with_operation("check_git")
                    .with_suggestion("Ensure git is properly installed"),
            });
        }

        Ok(())
    }

    /// Check if directory exists and is not empty
    async fn is_non_empty_dir(&self, path: &Path) -> WikifyResult<bool> {
        if !path.exists() {
            return Ok(false);
        }

        let mut entries = tokio::fs::read_dir(path)
            .await
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to read directory: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("repository_processor").with_operation("read_directory"),
            })?;

        // Check if directory has any entries
        match entries.next_entry().await {
            Ok(Some(_)) => Ok(true), // Directory is not empty
            Ok(None) => Ok(false),   // Directory is empty
            Err(e) => Err(WikifyError::Repository {
                message: format!("Failed to check directory contents: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("repository_processor")
                    .with_operation("check_directory_contents"),
            }),
        }
    }

    /// Prepare clone URL with authentication (following DeepWiki's approach)
    fn prepare_clone_url(&self, repo_info: &RepoInfo) -> WikifyResult<String> {
        if repo_info.access_token.is_none() {
            return Ok(repo_info.url.clone());
        }

        let token = repo_info.access_token.as_ref().unwrap();
        let mut url = Url::parse(&repo_info.url).map_err(|e| WikifyError::Repository {
            message: format!("Invalid URL: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("repository_processor").with_operation("parse_url"),
        })?;

        // Format URL with token based on repository type (like DeepWiki)
        match repo_info.repo_type {
            RepoType::GitHub => {
                // Format: https://{token}@github.com/owner/repo.git
                url.set_username(token)
                    .map_err(|_| WikifyError::Repository {
                        message: "Failed to set username in URL".to_string(),
                        source: None,
                        context: ErrorContext::new("repository_processor")
                            .with_operation("set_url_username"),
                    })?;
            }
            RepoType::GitLab => {
                // Format: https://oauth2:{token}@gitlab.com/owner/repo.git
                url.set_username("oauth2")
                    .map_err(|_| WikifyError::Repository {
                        message: "Failed to set username in URL".to_string(),
                        source: None,
                        context: ErrorContext::new("repository_processor")
                            .with_operation("set_url_username"),
                    })?;
                url.set_password(Some(token))
                    .map_err(|_| WikifyError::Repository {
                        message: "Failed to set password in URL".to_string(),
                        source: None,
                        context: ErrorContext::new("repository_processor")
                            .with_operation("set_url_password"),
                    })?;
            }
            RepoType::Bitbucket => {
                // Format: https://x-token-auth:{token}@bitbucket.org/owner/repo.git
                url.set_username("x-token-auth")
                    .map_err(|_| WikifyError::Repository {
                        message: "Failed to set username in URL".to_string(),
                        source: None,
                        context: ErrorContext::new("repository_processor")
                            .with_operation("set_url_username"),
                    })?;
                url.set_password(Some(token))
                    .map_err(|_| WikifyError::Repository {
                        message: "Failed to set password in URL".to_string(),
                        source: None,
                        context: ErrorContext::new("repository_processor")
                            .with_operation("set_url_password"),
                    })?;
            }
            RepoType::Local => {
                return Err(WikifyError::Repository {
                    message: "Local repositories don't need authentication".to_string(),
                    source: None,
                    context: ErrorContext::new("repository_processor")
                        .with_operation("prepare_clone_url"),
                });
            }
        }

        Ok(url.to_string())
    }

    /// Get the local path for a repository
    fn get_repo_path(&self, repo_info: &RepoInfo) -> PathBuf {
        match repo_info.repo_type {
            RepoType::Local => PathBuf::from(&repo_info.url),
            _ => {
                let repo_name = format!("{}_{}", repo_info.owner, repo_info.name);
                self.base_path.join("repos").join(repo_name)
            }
        }
    }

    /// Extract repository information from URL
    pub fn parse_repo_url(url: &str) -> WikifyResult<RepoInfo> {
        if !url.starts_with("http") {
            // Assume local path
            let path = Path::new(url);
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            return Ok(RepoInfo {
                owner: "local".to_string(),
                name,
                repo_type: RepoType::Local,
                url: url.to_string(),
                access_token: None,
                local_path: Some(url.to_string()),
            });
        }

        let parsed_url = Url::parse(url).map_err(|e| WikifyError::Repository {
            message: format!("Invalid URL: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("repository_processor").with_operation("parse_url"),
        })?;

        let host = parsed_url
            .host_str()
            .ok_or_else(|| WikifyError::Repository {
                message: "No host in URL".to_string(),
                source: None,
                context: ErrorContext::new("repository_processor").with_operation("extract_host"),
            })?;

        let repo_type = match host {
            "github.com" => RepoType::GitHub,
            "gitlab.com" => RepoType::GitLab,
            "bitbucket.org" => RepoType::Bitbucket,
            _ if host.contains("gitlab") => RepoType::GitLab,
            _ if host.contains("github") => RepoType::GitHub,
            _ => {
                return Err(WikifyError::Repository {
                    message: format!("Unsupported host: {}", host),
                    source: None,
                    context: ErrorContext::new("repository_processor")
                        .with_operation("determine_repo_type")
                        .with_suggestion("Supported hosts: github.com, gitlab.com, bitbucket.org"),
                })
            }
        };

        let path_segments: Vec<&str> = parsed_url
            .path()
            .trim_start_matches('/')
            .trim_end_matches(".git")
            .split('/')
            .collect();

        if path_segments.len() < 2 {
            return Err(WikifyError::Repository {
                message: "Invalid repository URL format".to_string(),
                source: None,
                context: ErrorContext::new("repository_processor")
                    .with_operation("parse_path_segments")
                    .with_suggestion("URL should be in format: https://host.com/owner/repo"),
            });
        }

        let owner = path_segments[path_segments.len() - 2].to_string();
        let name = path_segments[path_segments.len() - 1].to_string();

        Ok(RepoInfo {
            owner,
            name,
            repo_type,
            url: url.to_string(),
            access_token: None,
            local_path: None,
        })
    }
}
