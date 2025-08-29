//! Gitea API client implementation
//!
//! Gitea uses GitHub-compatible API, so this implementation is based on the GitHub client
//! with minor modifications for Gitea-specific endpoints and behavior.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use log::{debug, info, warn};
use serde::Deserialize;
use wikify_core::{WikifyError, WikifyResult};

use super::{
    create_http_client, handle_response_error, ApiClientConfig, RepositoryApiClient,
    RepositoryFile, RepositoryMetadata,
};

/// Gitea API client (GitHub-compatible)
pub struct GiteaApiClient {
    client: reqwest::Client,
    config: ApiClientConfig,
}

/// Gitea repository response (GitHub-compatible)
#[derive(Debug, Deserialize)]
struct GiteaRepository {
    name: String,
    description: Option<String>,
    default_branch: String,
    language: Option<String>,
    size: Option<u64>,
    private: bool,
}

/// Gitea tree response (GitHub-compatible)
#[derive(Debug, Deserialize)]
struct GiteaTreeResponse {
    tree: Vec<GiteaTreeItem>,
    truncated: Option<bool>,
}

/// Gitea tree item (GitHub-compatible)
#[derive(Debug, Deserialize)]
struct GiteaTreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
    size: Option<u64>,
    sha: String,
}

/// Gitea content response (GitHub-compatible)
#[derive(Debug, Deserialize)]
struct GiteaContentResponse {
    content: String,
    encoding: String,
}

impl GiteaApiClient {
    /// Create a new Gitea API client
    pub fn new(config: ApiClientConfig) -> WikifyResult<Self> {
        let client = create_http_client(&config)?;

        info!("Created Gitea API client for {}", config.base_url);

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

        headers
    }

    /// Make a GET request to Gitea API
    async fn get_request(&self, endpoint: &str) -> WikifyResult<reqwest::Response> {
        let url = format!(
            "{}/{}",
            self.config.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        debug!("Making Gitea API request to: {}", url);

        let response = self
            .client
            .get(&url)
            .headers(self.create_auth_headers())
            .send()
            .await
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to make request to Gitea API: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("gitea_api_client")
                    .with_operation("get_request"),
            })?;

        if !response.status().is_success() {
            return Err(Box::new(
                handle_response_error(response, "gitea_api_request").await,
            ));
        }

        Ok(response)
    }

    /// Decode base64 content from Gitea API
    fn decode_base64_content(&self, content: &str) -> WikifyResult<String> {
        // Remove newlines and whitespace
        let cleaned_content = content.replace(['\n', '\r', ' '], "");

        let decoded_bytes =
            BASE64
                .decode(&cleaned_content)
                .map_err(|e| WikifyError::Repository {
                    message: format!("Failed to decode base64 content: {}", e),
                    source: Some(Box::new(e)),
                    context: wikify_core::ErrorContext::new("gitea_api_client")
                        .with_operation("decode_base64_content"),
                })?;

        String::from_utf8(decoded_bytes).map_err(|e| {
            Box::new(WikifyError::Repository {
                message: format!("Content is not valid UTF-8: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("gitea_api_client")
                    .with_operation("decode_base64_content"),
            })
        })
    }
}

#[async_trait]
impl RepositoryApiClient for GiteaApiClient {
    async fn get_repository_metadata(
        &self,
        owner: &str,
        repo: &str,
    ) -> WikifyResult<RepositoryMetadata> {
        info!("Fetching Gitea repository metadata for {}/{}", owner, repo);

        let endpoint = format!("repos/{}/{}", owner, repo);
        let response = self.get_request(&endpoint).await?;

        let gitea_repo: GiteaRepository =
            response.json().await.map_err(|e| WikifyError::Repository {
                message: format!("Failed to parse repository metadata: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("gitea_api_client")
                    .with_operation("get_repository_metadata"),
            })?;

        Ok(RepositoryMetadata {
            name: gitea_repo.name,
            description: gitea_repo.description,
            default_branch: gitea_repo.default_branch,
            language: gitea_repo.language,
            topics: Vec::new(), // Gitea might not have topics in the same format
            size: gitea_repo.size,
            private: gitea_repo.private,
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
            "Fetching Gitea file tree for {}/{} (branch: {})",
            owner, repo, branch
        );

        let endpoint = format!("repos/{}/{}/git/trees/{}?recursive=1", owner, repo, branch);
        let response = self.get_request(&endpoint).await?;

        let tree_response: GiteaTreeResponse =
            response.json().await.map_err(|e| WikifyError::Repository {
                message: format!("Failed to parse file tree: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("gitea_api_client")
                    .with_operation("get_file_tree"),
            })?;

        if tree_response.truncated.unwrap_or(false) {
            warn!("Gitea file tree was truncated for {}/{}", owner, repo);
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
            "Retrieved {} files from Gitea repository {}/{}",
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
            "Fetching Gitea file content for {}/{}/{}",
            owner, repo, path
        );

        let mut endpoint = format!("repos/{}/{}/contents/{}", owner, repo, path);
        if let Some(branch) = branch {
            endpoint.push_str(&format!("?ref={}", branch));
        }

        let response = self.get_request(&endpoint).await?;

        let content_response: GiteaContentResponse =
            response.json().await.map_err(|e| WikifyError::Repository {
                message: format!("Failed to parse file content response: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("gitea_api_client")
                    .with_operation("get_file_content"),
            })?;

        if content_response.encoding != "base64" {
            return Err(Box::new(WikifyError::Repository {
                message: format!("Unexpected encoding: {}", content_response.encoding),
                source: None,
                context: wikify_core::ErrorContext::new("gitea_api_client")
                    .with_operation("get_file_content")
                    .with_suggestion("Expected base64 encoding from Gitea API"),
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
        debug!("Fetching Gitea README for {}/{}", owner, repo);

        // Try common README file names
        let readme_names = ["README.md", "README.rst", "README.txt", "README"];

        for readme_name in &readme_names {
            match self
                .get_file_content(owner, repo, readme_name, branch)
                .await
            {
                Ok(content) => return Ok(Some(content)),
                Err(_) => continue, // Try next README name
            }
        }

        debug!("README not found for Gitea repository {}/{}", owner, repo);
        Ok(None)
    }

    async fn repository_exists(&self, owner: &str, repo: &str) -> WikifyResult<bool> {
        debug!("Checking if Gitea repository {}/{} exists", owner, repo);

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
