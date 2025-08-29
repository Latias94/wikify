//! Bitbucket API client implementation

use async_trait::async_trait;
use log::{debug, info};
use serde::Deserialize;
use wikify_core::{WikifyError, WikifyResult};

use super::{
    create_http_client, handle_response_error, ApiClientConfig, RepositoryApiClient,
    RepositoryFile, RepositoryMetadata,
};

/// Bitbucket API client
pub struct BitbucketApiClient {
    client: reqwest::Client,
    config: ApiClientConfig,
}

/// Bitbucket repository response
#[derive(Debug, Deserialize)]
struct BitbucketRepository {
    name: String,
    description: Option<String>,
    mainbranch: Option<BitbucketBranch>,
    language: Option<String>,
    size: Option<u64>,
    is_private: bool,
}

/// Bitbucket branch info
#[derive(Debug, Deserialize)]
struct BitbucketBranch {
    name: String,
}

/// Bitbucket tree response
#[derive(Debug, Deserialize)]
struct BitbucketTreeResponse {
    values: Vec<BitbucketTreeItem>,
    next: Option<String>,
}

/// Bitbucket tree item
#[derive(Debug, Deserialize)]
struct BitbucketTreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
    size: Option<u64>,
    commit: Option<BitbucketCommit>,
}

/// Bitbucket commit info
#[derive(Debug, Deserialize)]
struct BitbucketCommit {
    hash: String,
}

impl BitbucketApiClient {
    /// Create a new Bitbucket API client
    pub fn new(config: ApiClientConfig) -> WikifyResult<Self> {
        let client = create_http_client(&config)?;

        info!("Created Bitbucket API client for {}", config.base_url);

        Ok(Self { client, config })
    }

    /// Create authorization headers
    fn create_auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        if let Some(ref token) = self.config.access_token {
            if let Ok(auth_value) =
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
            {
                headers.insert(reqwest::header::AUTHORIZATION, auth_value);
            }
        }

        headers
    }

    /// Make a GET request to Bitbucket API
    async fn get_request(&self, endpoint: &str) -> WikifyResult<reqwest::Response> {
        let url = format!(
            "{}/{}",
            self.config.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        debug!("Making Bitbucket API request to: {}", url);

        let response = self
            .client
            .get(&url)
            .headers(self.create_auth_headers())
            .send()
            .await
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to make request to Bitbucket API: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("bitbucket_api_client")
                    .with_operation("get_request"),
            })?;

        if !response.status().is_success() {
            return Err(handle_response_error(response, "bitbucket_api_request").await);
        }

        Ok(response)
    }

    /// Get all tree items with pagination
    async fn get_all_tree_items(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
    ) -> WikifyResult<Vec<BitbucketTreeItem>> {
        let mut all_items = Vec::new();
        let branch = branch.unwrap_or("HEAD");

        let mut endpoint = format!("repositories/{}/{}/src/{}", owner, repo, branch);

        loop {
            let response = self.get_request(&endpoint).await?;

            let tree_response: BitbucketTreeResponse =
                response.json().await.map_err(|e| WikifyError::Repository {
                    message: format!("Failed to parse Bitbucket tree response: {}", e),
                    source: Some(Box::new(e)),
                    context: wikify_core::ErrorContext::new("bitbucket_api_client")
                        .with_operation("get_all_tree_items"),
                })?;

            all_items.extend(tree_response.values);

            // Check if there are more pages
            if let Some(next_url) = tree_response.next {
                // Extract the endpoint from the full URL
                if let Some(api_part) = next_url.strip_prefix(&self.config.base_url) {
                    endpoint = api_part.trim_start_matches('/').to_string();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(all_items)
    }
}

#[async_trait]
impl RepositoryApiClient for BitbucketApiClient {
    async fn get_repository_metadata(
        &self,
        owner: &str,
        repo: &str,
    ) -> WikifyResult<RepositoryMetadata> {
        info!(
            "Fetching Bitbucket repository metadata for {}/{}",
            owner, repo
        );

        let endpoint = format!("repositories/{}/{}", owner, repo);
        let response = self.get_request(&endpoint).await?;

        let bitbucket_repo: BitbucketRepository =
            response.json().await.map_err(|e| WikifyError::Repository {
                message: format!("Failed to parse repository metadata: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("bitbucket_api_client")
                    .with_operation("get_repository_metadata"),
            })?;

        let default_branch = bitbucket_repo
            .mainbranch
            .map(|b| b.name)
            .unwrap_or_else(|| "main".to_string());

        Ok(RepositoryMetadata {
            name: bitbucket_repo.name,
            description: bitbucket_repo.description,
            default_branch,
            language: bitbucket_repo.language,
            topics: Vec::new(), // Bitbucket doesn't have topics in the same way
            size: bitbucket_repo.size,
            private: bitbucket_repo.is_private,
        })
    }

    async fn get_file_tree(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
    ) -> WikifyResult<Vec<RepositoryFile>> {
        info!("Fetching Bitbucket file tree for {}/{}", owner, repo);

        let tree_items = self.get_all_tree_items(owner, repo, branch).await?;

        let files: Vec<RepositoryFile> = tree_items
            .into_iter()
            .filter(|item| item.item_type == "commit_file") // Only include files
            .map(|item| RepositoryFile {
                path: item.path,
                file_type: item.item_type,
                size: item.size,
                sha: item.commit.map(|c| c.hash),
            })
            .collect();

        info!(
            "Retrieved {} files from Bitbucket repository {}/{}",
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
            "Fetching Bitbucket file content for {}/{}/{}",
            owner, repo, path
        );

        let branch = branch.unwrap_or("HEAD");
        let endpoint = format!("repositories/{}/{}/src/{}/{}", owner, repo, branch, path);

        let response = self.get_request(&endpoint).await?;

        // Bitbucket returns raw file content, not base64 encoded
        let content = response.text().await.map_err(|e| WikifyError::Repository {
            message: format!("Failed to read file content: {}", e),
            source: Some(Box::new(e)),
            context: wikify_core::ErrorContext::new("bitbucket_api_client")
                .with_operation("get_file_content"),
        })?;

        Ok(content)
    }

    async fn get_readme(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
    ) -> WikifyResult<Option<String>> {
        debug!("Fetching Bitbucket README for {}/{}", owner, repo);

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

        debug!(
            "README not found for Bitbucket repository {}/{}",
            owner, repo
        );
        Ok(None)
    }

    async fn repository_exists(&self, owner: &str, repo: &str) -> WikifyResult<bool> {
        debug!("Checking if Bitbucket repository {}/{} exists", owner, repo);

        let endpoint = format!("repositories/{}/{}", owner, repo);
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
