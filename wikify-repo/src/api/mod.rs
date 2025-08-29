//! API clients for accessing remote repositories
//!
//! This module provides API clients for different Git hosting platforms,
//! allowing direct access to repository content without cloning.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wikify_core::{WikifyError, WikifyResult};

pub mod bitbucket;
pub mod gitea;
pub mod github;
pub mod gitlab;

#[cfg(test)]
mod tests;

pub use bitbucket::BitbucketApiClient;
pub use gitea::GiteaApiClient;
pub use github::GitHubApiClient;
pub use gitlab::GitLabApiClient;

/// Represents a file in the repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryFile {
    /// File path relative to repository root
    pub path: String,
    /// File type (blob, tree, etc.)
    pub file_type: String,
    /// File size in bytes (if available)
    pub size: Option<u64>,
    /// SHA hash of the file (if available)
    pub sha: Option<String>,
}

/// Repository metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryMetadata {
    /// Repository name
    pub name: String,
    /// Repository description
    pub description: Option<String>,
    /// Default branch name
    pub default_branch: String,
    /// Repository language
    pub language: Option<String>,
    /// Repository topics/tags
    pub topics: Vec<String>,
    /// Repository size in KB
    pub size: Option<u64>,
    /// Whether the repository is private
    pub private: bool,
}

/// Configuration for API clients
#[derive(Debug, Clone)]
pub struct ApiClientConfig {
    /// Base URL for the API
    pub base_url: String,
    /// Access token for authentication
    pub access_token: Option<String>,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// User agent string
    pub user_agent: String,
    /// Additional headers
    pub headers: HashMap<String, String>,
}

impl Default for ApiClientConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            access_token: None,
            timeout_seconds: 30,
            user_agent: "wikify/1.0".to_string(),
            headers: HashMap::new(),
        }
    }
}

impl ApiClientConfig {
    /// Create a new configuration for GitHub
    pub fn github(access_token: Option<String>) -> Self {
        Self {
            base_url: "https://api.github.com".to_string(),
            access_token,
            ..Default::default()
        }
    }

    /// Create a new configuration for GitLab
    pub fn gitlab(base_url: Option<String>, access_token: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "https://gitlab.com/api/v4".to_string()),
            access_token,
            ..Default::default()
        }
    }

    /// Create a new configuration for Bitbucket
    pub fn bitbucket(access_token: Option<String>) -> Self {
        Self {
            base_url: "https://api.bitbucket.org/2.0".to_string(),
            access_token,
            ..Default::default()
        }
    }

    /// Create a new configuration for Gitea
    pub fn gitea(base_url: String, access_token: Option<String>) -> Self {
        Self {
            base_url: format!("{}/api/v1", base_url.trim_end_matches('/')),
            access_token,
            ..Default::default()
        }
    }

    /// Set additional header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }
}

/// Trait for repository API clients
#[async_trait]
pub trait RepositoryApiClient: Send + Sync {
    /// Get repository metadata
    async fn get_repository_metadata(
        &self,
        owner: &str,
        repo: &str,
    ) -> WikifyResult<RepositoryMetadata>;

    /// Get the complete file tree of a repository
    async fn get_file_tree(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
    ) -> WikifyResult<Vec<RepositoryFile>>;

    /// Get the content of a specific file
    async fn get_file_content(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        branch: Option<&str>,
    ) -> WikifyResult<String>;

    /// Get README content (if available)
    async fn get_readme(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
    ) -> WikifyResult<Option<String>>;

    /// Check if the repository exists and is accessible
    async fn repository_exists(&self, owner: &str, repo: &str) -> WikifyResult<bool>;

    /// Get the default branch name
    async fn get_default_branch(&self, owner: &str, repo: &str) -> WikifyResult<String>;
}

/// Factory for creating API clients
pub struct ApiClientFactory;

impl ApiClientFactory {
    /// Create an API client based on the repository type
    pub fn create_client(
        repo_type: &str,
        config: ApiClientConfig,
    ) -> WikifyResult<Box<dyn RepositoryApiClient>> {
        match repo_type.to_lowercase().as_str() {
            "github" => Ok(Box::new(GitHubApiClient::new(config)?)),
            "gitlab" => Ok(Box::new(GitLabApiClient::new(config)?)),
            "bitbucket" => Ok(Box::new(BitbucketApiClient::new(config)?)),
            "gitea" => Ok(Box::new(GiteaApiClient::new(config)?)),
            _ => Err(Box::new(WikifyError::Repository {
                message: format!("Unsupported repository type: {}", repo_type),
                source: None,
                context: wikify_core::ErrorContext::new("api_client_factory")
                    .with_operation("create_client")
                    .with_suggestion("Supported types: github, gitlab, bitbucket, gitea"),
            })),
        }
    }
}

/// Helper function to create HTTP client with common configuration
pub(crate) fn create_http_client(config: &ApiClientConfig) -> WikifyResult<reqwest::Client> {
    let mut headers = reqwest::header::HeaderMap::new();

    // Add user agent
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_str(&config.user_agent).map_err(|e| {
            WikifyError::Repository {
                message: format!("Invalid user agent: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("http_client")
                    .with_operation("create_client"),
            }
        })?,
    );

    // Add custom headers
    for (key, value) in &config.headers {
        let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes()).map_err(|e| {
            WikifyError::Repository {
                message: format!("Invalid header name '{}': {}", key, e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("http_client")
                    .with_operation("create_client"),
            }
        })?;

        let header_value =
            reqwest::header::HeaderValue::from_str(value).map_err(|e| WikifyError::Repository {
                message: format!("Invalid header value for '{}': {}", key, e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("http_client")
                    .with_operation("create_client"),
            })?;

        headers.insert(header_name, header_value);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_seconds))
        .default_headers(headers)
        .build()
        .map_err(|e| WikifyError::Repository {
            message: format!("Failed to create HTTP client: {}", e),
            source: Some(Box::new(e)),
            context: wikify_core::ErrorContext::new("http_client").with_operation("create_client"),
        })?;

    Ok(client)
}

/// Helper function to handle HTTP response errors
pub(crate) async fn handle_response_error(
    response: reqwest::Response,
    context: &str,
) -> WikifyError {
    let status = response.status();
    let url = response.url().clone();

    let error_body = response.text().await.unwrap_or_default();

    WikifyError::Repository {
        message: format!(
            "HTTP {} error for {}: {}",
            status.as_u16(),
            url,
            if error_body.is_empty() {
                status.canonical_reason().unwrap_or("Unknown error")
            } else {
                &error_body
            }
        ),
        source: None,
        context: wikify_core::ErrorContext::new("api_client")
            .with_operation(context)
            .with_suggestion(match status.as_u16() {
                401 => "Check your access token",
                403 => "Check repository permissions or rate limits",
                404 => "Repository not found or not accessible",
                _ => "Check network connectivity and API status",
            }),
    }
}
