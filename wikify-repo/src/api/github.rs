//! GitHub API client implementation

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use log::{debug, info, warn};
use serde::Deserialize;
use wikify_core::{WikifyError, WikifyResult};

use super::{
    create_http_client, handle_response_error, ApiClientConfig, RepositoryApiClient,
    RepositoryFile, RepositoryMetadata,
};

/// GitHub API client
pub struct GitHubApiClient {
    client: reqwest::Client,
    config: ApiClientConfig,
}

/// GitHub repository response
#[derive(Debug, Deserialize)]
struct GitHubRepository {
    name: String,
    description: Option<String>,
    default_branch: String,
    language: Option<String>,
    topics: Option<Vec<String>>,
    size: Option<u64>,
    private: bool,
}

/// GitHub tree response
#[derive(Debug, Deserialize)]
struct GitHubTreeResponse {
    tree: Vec<GitHubTreeItem>,
    truncated: Option<bool>,
}

/// GitHub tree item
#[derive(Debug, Deserialize)]
struct GitHubTreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
    size: Option<u64>,
    sha: String,
}

/// GitHub content response
#[derive(Debug, Deserialize)]
struct GitHubContentResponse {
    content: String,
    encoding: String,
    size: u64,
    sha: String,
}

/// GitHub README response
#[derive(Debug, Deserialize)]
struct GitHubReadmeResponse {
    content: String,
    encoding: String,
}

impl GitHubApiClient {
    /// Create a new GitHub API client
    pub fn new(config: ApiClientConfig) -> WikifyResult<Self> {
        let client = create_http_client(&config)?;

        info!("Created GitHub API client for {}", config.base_url);

        Ok(Self { client, config })
    }

    /// Create authorization headers
    fn create_auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        if let Some(ref token) = self.config.access_token {
            if let Ok(auth_value) =
                reqwest::header::HeaderValue::from_str(&format!("token {}", token))
            {
                headers.insert(reqwest::header::AUTHORIZATION, auth_value);
            }
        }

        // GitHub API version
        if let Ok(accept_value) =
            reqwest::header::HeaderValue::from_str("application/vnd.github.v3+json")
        {
            headers.insert(reqwest::header::ACCEPT, accept_value);
        }

        headers
    }

    /// Make a GET request to GitHub API
    async fn get_request(&self, endpoint: &str) -> WikifyResult<reqwest::Response> {
        let url = format!(
            "{}/{}",
            self.config.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        debug!("Making GitHub API request to: {}", url);

        let response = self
            .client
            .get(&url)
            .headers(self.create_auth_headers())
            .send()
            .await
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to make request to GitHub API: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("github_api_client")
                    .with_operation("get_request"),
            })?;

        if !response.status().is_success() {
            return Err(Box::new(
                handle_response_error(response, "github_api_request").await,
            ));
        }

        Ok(response)
    }

    /// Decode base64 content from GitHub API
    fn decode_base64_content(&self, content: &str) -> WikifyResult<String> {
        // Remove newlines and whitespace
        let cleaned_content = content.replace(['\n', '\r', ' '], "");

        let decoded_bytes =
            BASE64
                .decode(&cleaned_content)
                .map_err(|e| WikifyError::Repository {
                    message: format!("Failed to decode base64 content: {}", e),
                    source: Some(Box::new(e)),
                    context: wikify_core::ErrorContext::new("github_api_client")
                        .with_operation("decode_base64_content"),
                })?;

        String::from_utf8(decoded_bytes).map_err(|e| {
            Box::new(WikifyError::Repository {
                message: format!("Content is not valid UTF-8: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("github_api_client")
                    .with_operation("decode_base64_content"),
            })
        })
    }
}

#[async_trait]
impl RepositoryApiClient for GitHubApiClient {
    async fn get_repository_metadata(
        &self,
        owner: &str,
        repo: &str,
    ) -> WikifyResult<RepositoryMetadata> {
        info!("Fetching GitHub repository metadata for {}/{}", owner, repo);

        let endpoint = format!("repos/{}/{}", owner, repo);
        let response = self.get_request(&endpoint).await?;

        let github_repo: GitHubRepository =
            response.json().await.map_err(|e| WikifyError::Repository {
                message: format!("Failed to parse repository metadata: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("github_api_client")
                    .with_operation("get_repository_metadata"),
            })?;

        Ok(RepositoryMetadata {
            name: github_repo.name,
            description: github_repo.description,
            default_branch: github_repo.default_branch,
            language: github_repo.language,
            topics: github_repo.topics.unwrap_or_default(),
            size: github_repo.size,
            private: github_repo.private,
        })
    }

    async fn get_file_tree(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
    ) -> WikifyResult<Vec<RepositoryFile>> {
        let branch = branch.unwrap_or("HEAD");
        info!(
            "Fetching GitHub file tree for {}/{} (branch: {})",
            owner, repo, branch
        );

        let endpoint = format!("repos/{}/{}/git/trees/{}?recursive=1", owner, repo, branch);
        let response = self.get_request(&endpoint).await?;

        let tree_response: GitHubTreeResponse =
            response.json().await.map_err(|e| WikifyError::Repository {
                message: format!("Failed to parse file tree: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("github_api_client")
                    .with_operation("get_file_tree"),
            })?;

        if tree_response.truncated.unwrap_or(false) {
            warn!("GitHub file tree was truncated for {}/{}", owner, repo);
        }

        let files: Vec<RepositoryFile> = tree_response
            .tree
            .into_iter()
            .filter(|item| item.item_type == "blob") // Only include files, not directories
            .map(|item| RepositoryFile {
                path: item.path,
                file_type: item.item_type,
                size: item.size,
                sha: Some(item.sha),
            })
            .collect();

        info!(
            "Retrieved {} files from GitHub repository {}/{}",
            files.len(),
            owner,
            repo
        );
        Ok(files)
    }

    async fn get_file_content(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        branch: Option<&str>,
    ) -> WikifyResult<String> {
        debug!(
            "Fetching GitHub file content for {}/{}/{}",
            owner, repo, path
        );

        let mut endpoint = format!("repos/{}/{}/contents/{}", owner, repo, path);
        if let Some(branch) = branch {
            endpoint.push_str(&format!("?ref={}", branch));
        }

        let response = self.get_request(&endpoint).await?;

        let content_response: GitHubContentResponse =
            response.json().await.map_err(|e| WikifyError::Repository {
                message: format!("Failed to parse file content response: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("github_api_client")
                    .with_operation("get_file_content"),
            })?;

        if content_response.encoding != "base64" {
            return Err(Box::new(WikifyError::Repository {
                message: format!("Unexpected encoding: {}", content_response.encoding),
                source: None,
                context: wikify_core::ErrorContext::new("github_api_client")
                    .with_operation("get_file_content")
                    .with_suggestion("Expected base64 encoding from GitHub API"),
            }));
        }

        self.decode_base64_content(&content_response.content)
    }

    async fn get_readme(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
    ) -> WikifyResult<Option<String>> {
        debug!("Fetching GitHub README for {}/{}", owner, repo);

        let mut endpoint = format!("repos/{}/{}/readme", owner, repo);
        if let Some(branch) = branch {
            endpoint.push_str(&format!("?ref={}", branch));
        }

        let response = match self.get_request(&endpoint).await {
            Ok(response) => response,
            Err(_) => {
                // README not found is not an error
                debug!("README not found for {}/{}", owner, repo);
                return Ok(None);
            }
        };

        let readme_response: GitHubReadmeResponse =
            response.json().await.map_err(|e| WikifyError::Repository {
                message: format!("Failed to parse README response: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("github_api_client")
                    .with_operation("get_readme"),
            })?;

        if readme_response.encoding != "base64" {
            return Err(Box::new(WikifyError::Repository {
                message: format!("Unexpected README encoding: {}", readme_response.encoding),
                source: None,
                context: wikify_core::ErrorContext::new("github_api_client")
                    .with_operation("get_readme"),
            }));
        }

        let content = self.decode_base64_content(&readme_response.content)?;
        Ok(Some(content))
    }

    async fn repository_exists(&self, owner: &str, repo: &str) -> WikifyResult<bool> {
        debug!("Checking if GitHub repository {}/{} exists", owner, repo);

        let endpoint = format!("repos/{}/{}", owner, repo);
        match self.get_request(&endpoint).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn get_default_branch(&self, owner: &str, repo: &str) -> WikifyResult<String> {
        let metadata = self.get_repository_metadata(owner, repo).await?;
        Ok(metadata.default_branch)
    }
}
