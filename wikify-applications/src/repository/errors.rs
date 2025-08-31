//! Repository-specific error types and handling
//!
//! Provides structured error types for repository operations with detailed
//! context, recovery suggestions, and proper error chaining.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Repository operation result type
pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Structured error types for repository operations
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[serde(tag = "error_type", content = "details")]
pub enum RepositoryError {
    /// Repository indexing failed
    #[error("Indexing failed for repository {repository_id}: {reason}")]
    IndexingFailed {
        repository_id: String,
        reason: String,
        retry_count: u32,
        last_attempt: DateTime<Utc>,
        recoverable: bool,
    },

    /// Repository query failed
    #[error("Query failed for repository {repository_id}: {reason}")]
    QueryFailed {
        repository_id: String,
        query: String,
        reason: String,
        suggestion: Option<String>,
    },

    /// Repository not found
    #[error("Repository not found: {repository_id}")]
    NotFound { repository_id: String },

    /// Repository not ready for operations
    #[error("Repository {repository_id} is not ready (status: {status:?})")]
    NotReady {
        repository_id: String,
        status: super::types::IndexingStatus,
        estimated_completion: Option<DateTime<Utc>>,
    },

    /// Rate limiting applied
    #[error("Rate limited for repository {repository_id}")]
    RateLimited {
        repository_id: String,
        retry_after: Duration,
        current_rate: f64,
        limit: f64,
    },

    /// Resource exhaustion
    #[error("Resource exhausted: {resource_type} (current: {current}, limit: {limit})")]
    ResourceExhausted {
        resource_type: String,
        current: u32,
        limit: u32,
        estimated_availability: Option<DateTime<Utc>>,
    },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        field: Option<String>,
        suggestion: String,
    },

    /// Permission denied
    #[error("Permission denied for repository {repository_id}: {reason}")]
    PermissionDenied {
        repository_id: String,
        reason: String,
        required_permission: String,
    },

    /// Timeout error
    #[error("Operation timed out for repository {repository_id} after {duration:?}")]
    Timeout {
        repository_id: String,
        operation: String,
        duration: Duration,
        suggestion: String,
    },

    /// Validation error
    #[error("Validation failed: {message}")]
    Validation {
        message: String,
        field: String,
        value: String,
        expected: String,
    },

    /// Internal system error
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        component: String,
        error_id: String,
        recoverable: bool,
    },
}

impl RepositoryError {
    /// Create an indexing failed error
    pub fn indexing_failed(
        repository_id: String,
        reason: String,
        retry_count: u32,
        recoverable: bool,
    ) -> Self {
        Self::IndexingFailed {
            repository_id,
            reason,
            retry_count,
            last_attempt: Utc::now(),
            recoverable,
        }
    }

    /// Create a query failed error with suggestion
    pub fn query_failed_with_suggestion(
        repository_id: String,
        query: String,
        reason: String,
        suggestion: String,
    ) -> Self {
        Self::QueryFailed {
            repository_id,
            query,
            reason,
            suggestion: Some(suggestion),
        }
    }

    /// Create a not ready error with estimated completion
    pub fn not_ready_with_eta(
        repository_id: String,
        status: super::types::IndexingStatus,
        eta: DateTime<Utc>,
    ) -> Self {
        Self::NotReady {
            repository_id,
            status,
            estimated_completion: Some(eta),
        }
    }

    /// Create a resource exhausted error with availability estimate
    pub fn resource_exhausted_with_eta(
        resource_type: String,
        current: u32,
        limit: u32,
        eta: DateTime<Utc>,
    ) -> Self {
        Self::ResourceExhausted {
            resource_type,
            current,
            limit,
            estimated_availability: Some(eta),
        }
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::IndexingFailed { recoverable, .. } => *recoverable,
            Self::Internal { recoverable, .. } => *recoverable,
            Self::QueryFailed { .. } => true,
            Self::RateLimited { .. } => true,
            Self::ResourceExhausted { .. } => true,
            Self::Timeout { .. } => true,
            Self::NotReady { .. } => true,
            Self::NotFound { .. } => false,
            Self::PermissionDenied { .. } => false,
            Self::Configuration { .. } => false,
            Self::Validation { .. } => false,
        }
    }

    /// Get retry delay for recoverable errors
    pub fn retry_delay(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after, .. } => Some(*retry_after),
            Self::ResourceExhausted { .. } => Some(Duration::from_secs(30)),
            Self::IndexingFailed { retry_count, .. } => {
                // Exponential backoff: 2^retry_count seconds, max 5 minutes
                let delay_secs = (2_u64.pow(*retry_count)).min(300);
                Some(Duration::from_secs(delay_secs))
            }
            Self::Timeout { .. } => Some(Duration::from_secs(10)),
            _ => None,
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            Self::IndexingFailed { repository_id, .. } => {
                format!(
                    "Failed to index repository {}. Please try again later.",
                    repository_id
                )
            }
            Self::QueryFailed {
                suggestion: Some(suggestion),
                ..
            } => {
                format!("Query failed. Suggestion: {}", suggestion)
            }
            Self::QueryFailed { .. } => {
                "Query failed. Please try rephrasing your question.".to_string()
            }
            Self::NotFound { repository_id } => {
                format!(
                    "Repository {} not found. Please check the repository ID.",
                    repository_id
                )
            }
            Self::NotReady {
                repository_id,
                estimated_completion: Some(eta),
                ..
            } => {
                format!(
                    "Repository {} is still being indexed. Expected completion: {}",
                    repository_id,
                    eta.format("%Y-%m-%d %H:%M:%S UTC")
                )
            }
            Self::NotReady { repository_id, .. } => {
                format!(
                    "Repository {} is still being indexed. Please wait.",
                    repository_id
                )
            }
            Self::RateLimited { retry_after, .. } => {
                format!(
                    "Rate limited. Please try again in {} seconds.",
                    retry_after.as_secs()
                )
            }
            Self::ResourceExhausted {
                resource_type,
                estimated_availability: Some(eta),
                ..
            } => {
                format!(
                    "System is currently at capacity ({}). Please try again after {}",
                    resource_type,
                    eta.format("%H:%M:%S UTC")
                )
            }
            Self::ResourceExhausted { resource_type, .. } => {
                format!(
                    "System is currently at capacity ({}). Please try again later.",
                    resource_type
                )
            }
            Self::PermissionDenied {
                required_permission,
                ..
            } => {
                format!(
                    "Permission denied. Required permission: {}",
                    required_permission
                )
            }
            Self::Timeout { operation, .. } => {
                format!("Operation '{}' timed out. Please try again.", operation)
            }
            Self::Validation { message, .. } => {
                format!("Invalid input: {}", message)
            }
            Self::Configuration { suggestion, .. } => {
                format!("Configuration error. {}", suggestion)
            }
            Self::Internal { .. } => {
                "An internal error occurred. Please contact support if this persists.".to_string()
            }
        }
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            Self::IndexingFailed { .. } => "indexing",
            Self::QueryFailed { .. } => "query",
            Self::NotFound { .. } => "not_found",
            Self::NotReady { .. } => "not_ready",
            Self::RateLimited { .. } => "rate_limit",
            Self::ResourceExhausted { .. } => "resource",
            Self::Configuration { .. } => "config",
            Self::PermissionDenied { .. } => "permission",
            Self::Timeout { .. } => "timeout",
            Self::Validation { .. } => "validation",
            Self::Internal { .. } => "internal",
        }
    }

    /// Convert to HTTP status code
    pub fn http_status(&self) -> u16 {
        match self {
            Self::NotFound { .. } => 404,
            Self::PermissionDenied { .. } => 403,
            Self::Validation { .. } => 400,
            Self::Configuration { .. } => 400,
            Self::RateLimited { .. } => 429,
            Self::ResourceExhausted { .. } => 503,
            Self::Timeout { .. } => 408,
            Self::NotReady { .. } => 202, // Accepted, still processing
            Self::IndexingFailed { .. } => 500,
            Self::QueryFailed { .. } => 500,
            Self::Internal { .. } => 500,
        }
    }
}

/// Convert from ApplicationError to RepositoryError
impl From<crate::ApplicationError> for RepositoryError {
    fn from(err: crate::ApplicationError) -> Self {
        match err {
            crate::ApplicationError::NotFound { message } => {
                // Try to extract repository_id from message
                let repository_id = message
                    .split_whitespace()
                    .find(|s| s.len() == 36 && s.contains('-'))
                    .unwrap_or("unknown")
                    .to_string();

                Self::NotFound { repository_id }
            }
            crate::ApplicationError::Permission { message } => Self::PermissionDenied {
                repository_id: "unknown".to_string(),
                reason: message,
                required_permission: "repository_access".to_string(),
            },
            crate::ApplicationError::Config { message } => Self::Configuration {
                message: message.clone(),
                field: None,
                suggestion: "Please check your configuration file".to_string(),
            },
            _ => Self::Internal {
                message: err.to_string(),
                component: "application".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        let recoverable_error = RepositoryError::indexing_failed(
            "test-repo".to_string(),
            "Network timeout".to_string(),
            1,
            true,
        );
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error = RepositoryError::NotFound {
            repository_id: "test-repo".to_string(),
        };
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_retry_delay() {
        let rate_limited = RepositoryError::RateLimited {
            repository_id: "test".to_string(),
            retry_after: Duration::from_secs(60),
            current_rate: 10.0,
            limit: 5.0,
        };
        assert_eq!(rate_limited.retry_delay(), Some(Duration::from_secs(60)));

        let not_found = RepositoryError::NotFound {
            repository_id: "test".to_string(),
        };
        assert_eq!(not_found.retry_delay(), None);
    }

    #[test]
    fn test_user_messages() {
        let error = RepositoryError::query_failed_with_suggestion(
            "test-repo".to_string(),
            "test query".to_string(),
            "No results".to_string(),
            "Try using different keywords".to_string(),
        );

        let message = error.user_message();
        assert!(message.contains("Try using different keywords"));
    }
}
