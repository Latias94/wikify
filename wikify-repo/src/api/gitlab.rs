//! GitLab API client implementation

use async_trait::async_trait;
use log::{debug, info};
use serde::Deserialize;
use std::collections::HashMap;
use wikify_core::{WikifyError, WikifyResult};

use super::{
    create_http_client, handle_response_error, ApiClientConfig, RepositoryApiClient,
    RepositoryFile, RepositoryMetadata,
};

/// GitLab API client
pub struct GitLabApiClient {
    client: reqwest::Client,
    config: ApiClientConfig,
}

/// GitLab project response
#[derive(Debug, Deserialize)]
struct GitLabProject {
    name: String,
    description: Option<String>,
    default_branch: String,
    #[serde(rename = "repository_languages")]
    languages: Option<HashMap<String, f64>>,
    topics: Option<Vec<String>>,
    #[serde(rename = "repository_size")]
    size: Option<u64>,
    visibility: String,
}

/// GitLab tree item
#[derive(Debug, Deserialize)]
struct GitLabTreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
    id: String,
    name: String,
}

/// GitLab file response
#[derive(Debug, Deserialize)]
struct GitLabFileResponse {
    content: String,
    encoding: String,
    size: u64,
    blob_id: String,
}

impl GitLabApiClient {
    /// Create a new GitLab API client
    pub fn new(config: ApiClientConfig) -> WikifyResult<Self> {
        let client = create_http_client(&config)?;

        info!("Created GitLab API client for {}", config.base_url);

        Ok(Self { client, config })
    }

    /// Create authorization headers
    fn create_auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        if let Some(ref token) = self.config.access_token {
            if let Ok(auth_value) = reqwest::header::HeaderValue::from_str(token) {
                headers.insert("PRIVATE-TOKEN", auth_value);
            }
        }

        headers
    }

    /// Make a GET request to GitLab API
    async fn get_request(&self, endpoint: &str) -> WikifyResult<reqwest::Response> {
        let url = format!(
            "{}/{}",
            self.config.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        debug!("Making GitLab API request to: {}", url);

        let response = self
            .client
            .get(&url)
            .headers(self.create_auth_headers())
            .send()
            .await
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to make request to GitLab API: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("gitlab_api_client")
                    .with_operation("get_request"),
            })?;

        if !response.status().is_success() {
            return Err(handle_response_error(response, "gitlab_api_request").await);
        }

        Ok(response)
    }

    /// Encode project path for GitLab API
    fn encode_project_path(&self, owner: &str, repo: &str) -> String {
        let project_path = format!("{}/{}", owner, repo);
        urlencoding::encode(&project_path).to_string()
    }

    /// Get all tree items with pagination
    async fn get_all_tree_items(
        &self,
        project_id: &str,
        branch: Option<&str>,
    ) -> WikifyResult<Vec<GitLabTreeItem>> {
        let mut all_items = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let mut endpoint = format!(
                "projects/{}/repository/tree?recursive=true&per_page={}&page={}",
                project_id, per_page, page
            );

            if let Some(branch) = branch {
                endpoint.push_str(&format!("&ref={}", branch));
            }

            let response = self.get_request(&endpoint).await?;

            // Check if there are more pages
            let has_next_page = response
                .headers()
                .get("x-next-page")
                .and_then(|v| v.to_str().ok())
                .is_some();

            let items: Vec<GitLabTreeItem> =
                response.json().await.map_err(|e| WikifyError::Repository {
                    message: format!("Failed to parse GitLab tree response: {}", e),
                    source: Some(Box::new(e)),
                    context: wikify_core::ErrorContext::new("gitlab_api_client")
                        .with_operation("get_all_tree_items"),
                })?;

            all_items.extend(items);

            if !has_next_page {
                break;
            }

            page += 1;
        }

        Ok(all_items)
    }

    /// Decode base64 content from GitLab API
    fn decode_base64_content(&self, content: &str) -> WikifyResult<String> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

        let decoded_bytes = BASE64
            .decode(content)
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to decode base64 content: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("gitlab_api_client")
                    .with_operation("decode_base64_content"),
            })?;

        String::from_utf8(decoded_bytes).map_err(|e| WikifyError::Repository {
            message: format!("Content is not valid UTF-8: {}", e),
            source: Some(Box::new(e)),
            context: wikify_core::ErrorContext::new("gitlab_api_client")
                .with_operation("decode_base64_content"),
        })
    }
}

#[async_trait]
impl RepositoryApiClient for GitLabApiClient {
    async fn get_repository_metadata(
        &self,
        owner: &str,
        repo: &str,
    ) -> WikifyResult<RepositoryMetadata> {
        info!("Fetching GitLab repository metadata for {}/{}", owner, repo);

        let project_id = self.encode_project_path(owner, repo);
        let endpoint = format!("projects/{}", project_id);
        let response = self.get_request(&endpoint).await?;

        let gitlab_project: GitLabProject =
            response.json().await.map_err(|e| WikifyError::Repository {
                message: format!("Failed to parse repository metadata: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("gitlab_api_client")
                    .with_operation("get_repository_metadata"),
            })?;

        // Get the primary language
        let language = gitlab_project.languages.as_ref().and_then(|langs| {
            langs
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(lang, _)| lang.clone())
        });

        Ok(RepositoryMetadata {
            name: gitlab_project.name,
            description: gitlab_project.description,
            default_branch: gitlab_project.default_branch,
            language,
            topics: gitlab_project.topics.unwrap_or_default(),
            size: gitlab_project.size,
            private: gitlab_project.visibility != "public",
        })
    }

    async fn get_file_tree(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
    ) -> WikifyResult<Vec<RepositoryFile>> {
        info!("Fetching GitLab file tree for {}/{}", owner, repo);

        let project_id = self.encode_project_path(owner, repo);
        let tree_items = self.get_all_tree_items(&project_id, branch).await?;

        let files: Vec<RepositoryFile> = tree_items
            .into_iter()
            .filter(|item| item.item_type == "blob") // Only include files, not directories
            .map(|item| RepositoryFile {
                path: item.path,
                file_type: item.item_type,
                size: None, // GitLab tree API doesn't provide file size
                sha: Some(item.id),
            })
            .collect();

        info!(
            "Retrieved {} files from GitLab repository {}/{}",
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
            "Fetching GitLab file content for {}/{}/{}",
            owner, repo, path
        );

        let project_id = self.encode_project_path(owner, repo);
        let encoded_path = urlencoding::encode(path);

        let mut endpoint = format!("projects/{}/repository/files/{}", project_id, encoded_path);
        if let Some(branch) = branch {
            endpoint.push_str(&format!("?ref={}", branch));
        }

        let response = self.get_request(&endpoint).await?;

        let file_response: GitLabFileResponse =
            response.json().await.map_err(|e| WikifyError::Repository {
                message: format!("Failed to parse file content response: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("gitlab_api_client")
                    .with_operation("get_file_content"),
            })?;

        if file_response.encoding != "base64" {
            return Err(WikifyError::Repository {
                message: format!("Unexpected encoding: {}", file_response.encoding),
                source: None,
                context: wikify_core::ErrorContext::new("gitlab_api_client")
                    .with_operation("get_file_content")
                    .with_suggestion("Expected base64 encoding from GitLab API"),
            });
        }

        self.decode_base64_content(&file_response.content)
    }

    async fn get_readme(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
    ) -> WikifyResult<Option<String>> {
        debug!("Fetching GitLab README for {}/{}", owner, repo);

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

        debug!("README not found for GitLab repository {}/{}", owner, repo);
        Ok(None)
    }

    async fn repository_exists(&self, owner: &str, repo: &str) -> WikifyResult<bool> {
        debug!("Checking if GitLab repository {}/{} exists", owner, repo);

        let project_id = self.encode_project_path(owner, repo);
        let endpoint = format!("projects/{}", project_id);
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
