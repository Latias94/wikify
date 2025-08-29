//! Wikify Core - Core data structures and trait definitions
//!
//! This module defines the core abstractions and data structures for the entire wikify system

pub mod async_utils;
pub mod config;
pub mod error;
pub mod logging;
pub mod traits;
pub mod types;

pub use async_utils::*;
pub use error::*;
pub use logging::*;
pub use traits::*;
pub use types::*;

// Re-export commonly used external types
pub use async_trait::async_trait;
pub use chrono::{DateTime, Utc};
pub use tokio;
pub use tracing;
pub use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wikify_error_creation() {
        let error = WikifyError::Repository {
            message: "test error".to_string(),
            source: None,
            context: crate::error::ErrorContext::new("test"),
        };
        assert!(matches!(error, WikifyError::Repository { .. }));
        assert!(error.to_string().contains("test error"));
    }

    #[test]
    fn test_wikify_result_ok() {
        let result: WikifyResult<i32> = Ok(42);
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value, 42);
        }
    }

    #[test]
    fn test_wikify_result_err() {
        let result: WikifyResult<i32> = Err(Box::new(WikifyError::Repository {
            message: "test".to_string(),
            source: None,
            context: crate::error::ErrorContext::new("test"),
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_repo_stats_creation() {
        let stats = RepoStats {
            total_files: 100,
            code_files: 80,
            doc_files: 20,
            total_lines: 5000,
            languages: vec!["Rust".to_string(), "Python".to_string()],
        };

        assert_eq!(stats.total_files, 100);
        assert_eq!(stats.code_files, 80);
        assert_eq!(stats.doc_files, 20);
        assert_eq!(stats.total_lines, 5000);
        assert_eq!(stats.languages.len(), 2);
    }

    #[test]
    fn test_repo_info_creation() {
        let repo_info = RepoInfo {
            owner: "test-owner".to_string(),
            name: "test-repo".to_string(),
            repo_type: RepoType::GitHub,
            url: "https://github.com/test/repo".to_string(),
            access_token: None,
            local_path: Some("/tmp/test".to_string()),
            access_mode: RepoAccessMode::GitClone,
        };

        assert_eq!(repo_info.name, "test-repo");
        assert_eq!(repo_info.owner, "test-owner");
        assert_eq!(repo_info.repo_type, RepoType::GitHub);
        assert!(repo_info.local_path.is_some());
    }

    #[test]
    fn test_repo_type_serialization() {
        let repo_type = RepoType::GitHub;
        let serialized = serde_json::to_string(&repo_type).unwrap();
        let deserialized: RepoType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(repo_type, deserialized);
    }

    #[test]
    fn test_wikify_config_default() {
        let config = WikifyConfig::default();
        assert_eq!(config.llm.provider, "openai");
        assert_eq!(config.llm.model, "gpt-4");
        assert_eq!(config.embedding.provider, "openai");
        assert_eq!(config.embedding.model, "text-embedding-3-small");
        assert_eq!(config.repository.max_size_mb, 50000);
        assert!(!config.storage.use_database);
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.include_location);
        assert!(config.include_timestamp);
    }
}
