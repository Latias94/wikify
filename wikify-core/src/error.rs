//! Unified error handling system
//!
//! Provides structured error types with context, recovery suggestions, and proper error chaining

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, warn};

pub type WikifyResult<T> = Result<T, WikifyError>;

/// Error context providing additional information for debugging and recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Unique error ID for tracking
    pub error_id: String,
    /// Timestamp when error occurred
    pub timestamp: DateTime<Utc>,
    /// Component where error originated
    pub component: String,
    /// Operation being performed when error occurred
    pub operation: Option<String>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
    /// Recovery suggestions
    pub recovery_suggestions: Vec<String>,
}

impl ErrorContext {
    pub fn new(component: &str) -> Self {
        Self {
            error_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            component: component.to_string(),
            operation: None,
            metadata: std::collections::HashMap::new(),
            recovery_suggestions: Vec::new(),
        }
    }

    pub fn with_operation(mut self, operation: &str) -> Self {
        self.operation = Some(operation.to_string());
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.recovery_suggestions.push(suggestion.to_string());
        self
    }
}

/// Main error type for the Wikify system
#[derive(Error, Debug)]
pub enum WikifyError {
    #[error("Repository error: {message}")]
    Repository {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Indexing error: {message}")]
    Indexing {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("RAG system error: {message}")]
    Rag {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Wiki generation error: {message}")]
    WikiGeneration {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Storage error: {message}")]
    Storage {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Configuration error: {message}")]
    Config {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Network error: {message}")]
    Network {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },

    #[error("Authentication error: {message}")]
    Authentication {
        message: String,
        context: ErrorContext,
    },

    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
        context: ErrorContext,
    },

    #[error("Resource not found: {resource}")]
    NotFound {
        resource: String,
        context: ErrorContext,
    },

    #[error("Operation timeout: {operation}")]
    Timeout {
        operation: String,
        duration_ms: u64,
        context: ErrorContext,
    },

    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after_ms: Option<u64>,
        context: ErrorContext,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Git error: {message}")]
    Git {
        message: String,
        context: ErrorContext,
    },

    #[error("LLM error: {message}")]
    Llm {
        message: String,
        provider: Option<String>,
        model: Option<String>,
        context: ErrorContext,
    },

    #[error("Embedding error: {message}")]
    Embedding {
        message: String,
        provider: Option<String>,
        context: ErrorContext,
    },

    #[error("Internal error: {message}")]
    Internal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: ErrorContext,
    },
}

impl WikifyError {
    /// Get the error context
    pub fn context(&self) -> Option<&ErrorContext> {
        match self {
            WikifyError::Repository { context, .. } => Some(context),
            WikifyError::Indexing { context, .. } => Some(context),
            WikifyError::Rag { context, .. } => Some(context),
            WikifyError::WikiGeneration { context, .. } => Some(context),
            WikifyError::Storage { context, .. } => Some(context),
            WikifyError::Config { context, .. } => Some(context),
            WikifyError::Network { context, .. } => Some(context),
            WikifyError::Authentication { context, .. } => Some(context),
            WikifyError::Validation { context, .. } => Some(context),
            WikifyError::NotFound { context, .. } => Some(context),
            WikifyError::Timeout { context, .. } => Some(context),
            WikifyError::RateLimit { context, .. } => Some(context),
            WikifyError::Git { context, .. } => Some(context),
            WikifyError::Llm { context, .. } => Some(context),
            WikifyError::Embedding { context, .. } => Some(context),
            WikifyError::Internal { context, .. } => Some(context),
            _ => None,
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            WikifyError::Network { .. } => true,
            WikifyError::Timeout { .. } => true,
            WikifyError::RateLimit { .. } => true,
            WikifyError::Authentication { .. } => false,
            WikifyError::Config { .. } => false,
            WikifyError::Validation { .. } => false,
            WikifyError::NotFound { .. } => false,
            _ => false,
        }
    }

    /// Get retry delay in milliseconds for recoverable errors
    pub fn retry_delay_ms(&self) -> Option<u64> {
        match self {
            WikifyError::Network { .. } => Some(1000),
            WikifyError::Timeout { .. } => Some(2000),
            WikifyError::RateLimit { retry_after_ms, .. } => *retry_after_ms,
            _ => None,
        }
    }

    /// Log the error with appropriate level
    pub fn log(&self) {
        match self {
            WikifyError::Internal { .. } => {
                error!(
                    error_id = ?self.context().map(|c| &c.error_id),
                    error = %self,
                    "Internal error occurred"
                );
            }
            WikifyError::Config { .. } | WikifyError::Validation { .. } => {
                error!(
                    error_id = ?self.context().map(|c| &c.error_id),
                    error = %self,
                    "Configuration or validation error"
                );
            }
            WikifyError::Network { .. } | WikifyError::Timeout { .. } => {
                warn!(
                    error_id = ?self.context().map(|c| &c.error_id),
                    error = %self,
                    "Network or timeout error (may be recoverable)"
                );
            }
            _ => {
                error!(
                    error_id = ?self.context().map(|c| &c.error_id),
                    error = %self,
                    "Error occurred"
                );
            }
        }
    }
}

/// Convenience macros for creating errors with context
#[macro_export]
macro_rules! repository_error {
    ($msg:expr, $component:expr) => {
        WikifyError::Repository {
            message: $msg.to_string(),
            source: None,
            context: ErrorContext::new($component),
        }
    };
    ($msg:expr, $component:expr, $source:expr) => {
        WikifyError::Repository {
            message: $msg.to_string(),
            source: Some(Box::new($source)),
            context: ErrorContext::new($component),
        }
    };
}

#[macro_export]
macro_rules! config_error {
    ($msg:expr, $component:expr) => {
        WikifyError::Config {
            message: $msg.to_string(),
            source: None,
            context: ErrorContext::new($component)
                .with_suggestion("Check your configuration file")
                .with_suggestion("Run 'wikify config --init' to create default config"),
        }
    };
}

#[macro_export]
macro_rules! validation_error {
    ($msg:expr, $field:expr, $component:expr) => {
        WikifyError::Validation {
            message: $msg.to_string(),
            field: Some($field.to_string()),
            context: ErrorContext::new($component)
                .with_suggestion("Check the field value and format"),
        }
    };
}

#[macro_export]
macro_rules! not_found_error {
    ($resource:expr, $component:expr) => {
        WikifyError::NotFound {
            resource: $resource.to_string(),
            context: ErrorContext::new($component)
                .with_suggestion("Verify the resource path or URL")
                .with_suggestion("Check if the resource exists and is accessible"),
        }
    };
}
