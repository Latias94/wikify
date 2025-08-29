//! Tests for API clients

#[cfg(test)]
mod tests {
    use super::super::*;
    use wikify_core::{RepoAccessMode, RepoInfo, RepoType};

    #[test]
    fn test_api_client_config_creation() {
        // Test GitHub config
        let github_config = ApiClientConfig::github(Some("test_token".to_string()));
        assert_eq!(github_config.base_url, "https://api.github.com");
        assert_eq!(github_config.access_token, Some("test_token".to_string()));

        // Test GitLab config
        let gitlab_config = ApiClientConfig::gitlab(None, Some("test_token".to_string()));
        assert_eq!(gitlab_config.base_url, "https://gitlab.com/api/v4");
        assert_eq!(gitlab_config.access_token, Some("test_token".to_string()));

        // Test custom GitLab config
        let custom_gitlab_config = ApiClientConfig::gitlab(
            Some("https://gitlab.example.com/api/v4".to_string()),
            Some("test_token".to_string()),
        );
        assert_eq!(
            custom_gitlab_config.base_url,
            "https://gitlab.example.com/api/v4"
        );

        // Test Bitbucket config
        let bitbucket_config = ApiClientConfig::bitbucket(Some("test_token".to_string()));
        assert_eq!(bitbucket_config.base_url, "https://api.bitbucket.org/2.0");
        assert_eq!(
            bitbucket_config.access_token,
            Some("test_token".to_string())
        );

        // Test Gitea config
        let gitea_config = ApiClientConfig::gitea(
            "https://gitea.example.com".to_string(),
            Some("test_token".to_string()),
        );
        assert_eq!(gitea_config.base_url, "https://gitea.example.com/api/v1");
        assert_eq!(gitea_config.access_token, Some("test_token".to_string()));
    }

    #[test]
    fn test_api_client_factory() {
        let config = ApiClientConfig::github(None);

        // Test successful creation
        let github_client = ApiClientFactory::create_client("github", config.clone());
        assert!(github_client.is_ok());

        let gitlab_client = ApiClientFactory::create_client("gitlab", config.clone());
        assert!(gitlab_client.is_ok());

        let bitbucket_client = ApiClientFactory::create_client("bitbucket", config.clone());
        assert!(bitbucket_client.is_ok());

        let gitea_client = ApiClientFactory::create_client("gitea", config.clone());
        assert!(gitea_client.is_ok());

        // Test unsupported type
        let unsupported_client = ApiClientFactory::create_client("unsupported", config);
        assert!(unsupported_client.is_err());
    }

    #[test]
    fn test_repository_file_creation() {
        let repo_file = RepositoryFile {
            path: "src/main.rs".to_string(),
            file_type: "blob".to_string(),
            size: Some(1024),
            sha: Some("abc123".to_string()),
        };

        assert_eq!(repo_file.path, "src/main.rs");
        assert_eq!(repo_file.file_type, "blob");
        assert_eq!(repo_file.size, Some(1024));
        assert_eq!(repo_file.sha, Some("abc123".to_string()));
    }

    #[test]
    fn test_repository_metadata_creation() {
        let metadata = RepositoryMetadata {
            name: "test-repo".to_string(),
            description: Some("A test repository".to_string()),
            default_branch: "main".to_string(),
            language: Some("Rust".to_string()),
            topics: vec!["rust".to_string(), "test".to_string()],
            size: Some(2048),
            private: false,
        };

        assert_eq!(metadata.name, "test-repo");
        assert_eq!(metadata.description, Some("A test repository".to_string()));
        assert_eq!(metadata.default_branch, "main");
        assert_eq!(metadata.language, Some("Rust".to_string()));
        assert_eq!(metadata.topics.len(), 2);
        assert_eq!(metadata.size, Some(2048));
        assert!(!metadata.private);
    }

    #[test]
    fn test_repo_access_mode() {
        // Test RepoAccessMode enum
        assert_eq!(RepoAccessMode::GitClone, RepoAccessMode::GitClone);
        assert_eq!(RepoAccessMode::Api, RepoAccessMode::Api);
        assert_ne!(RepoAccessMode::GitClone, RepoAccessMode::Api);
    }

    #[test]
    fn test_repo_info_with_access_mode() {
        let repo_info = RepoInfo {
            owner: "test-owner".to_string(),
            name: "test-repo".to_string(),
            repo_type: RepoType::GitHub,
            url: "https://github.com/test-owner/test-repo".to_string(),
            access_token: Some("test_token".to_string()),
            local_path: None,
            access_mode: RepoAccessMode::Api,
        };

        assert_eq!(repo_info.access_mode, RepoAccessMode::Api);
        assert_eq!(repo_info.repo_type, RepoType::GitHub);
        assert_eq!(repo_info.access_token, Some("test_token".to_string()));
    }

    // Integration tests would require actual API access, so we'll skip them for now
    // These would test actual API calls to GitHub, GitLab, and Bitbucket

    #[tokio::test]
    async fn test_http_client_creation() {
        let config = ApiClientConfig::github(None);
        let client = create_http_client(&config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_config_with_headers() {
        let config = ApiClientConfig::github(None)
            .with_header("X-Custom-Header".to_string(), "test-value".to_string())
            .with_timeout(60);

        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(
            config.headers.get("X-Custom-Header"),
            Some(&"test-value".to_string())
        );
    }
}

// Mock tests for API clients (without actual network calls)
#[cfg(test)]
mod mock_tests {
    use super::super::*;

    // These tests would use a mock HTTP server to test API client behavior
    // without making actual network requests. For now, we'll keep them as placeholders.

    #[tokio::test]
    #[ignore] // Ignore until we implement mock server
    async fn test_github_client_get_repository_metadata() {
        // TODO: Implement with mock server
        // This would test the GitHub client's ability to parse repository metadata
    }

    #[tokio::test]
    #[ignore] // Ignore until we implement mock server
    async fn test_gitlab_client_get_file_tree() {
        // TODO: Implement with mock server
        // This would test the GitLab client's ability to fetch and parse file trees
    }

    #[tokio::test]
    #[ignore] // Ignore until we implement mock server
    async fn test_bitbucket_client_get_file_content() {
        // TODO: Implement with mock server
        // This would test the Bitbucket client's ability to fetch file content
    }

    #[tokio::test]
    #[ignore] // Ignore until we implement mock server
    async fn test_api_error_handling() {
        // TODO: Implement with mock server
        // This would test how clients handle various HTTP error responses
    }
}
