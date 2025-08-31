//! Repository manager configuration and metrics
//!
//! Provides configuration types and monitoring capabilities for the
//! repository management system.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

/// Configuration for the repository manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryManagerConfig {
    /// Maximum number of concurrent indexing operations
    pub max_concurrent_indexing: usize,

    /// Maximum number of concurrent query operations
    pub max_concurrent_queries: usize,

    /// Timeout for indexing operations
    pub indexing_timeout: Duration,

    /// Timeout for query operations
    pub query_timeout: Duration,

    /// Maximum number of retry attempts for failed operations
    pub retry_attempts: u32,

    /// Base delay for retry backoff
    pub retry_backoff: Duration,

    /// Maximum delay for retry backoff
    pub max_retry_delay: Duration,

    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,

    /// Enable query result caching
    pub enable_query_cache: bool,

    /// Query cache TTL
    pub query_cache_ttl: Duration,

    /// Maximum number of cached query results
    pub query_cache_size: usize,

    /// Health check interval
    pub health_check_interval: Duration,

    /// Maximum time before considering an indexing operation stuck
    pub stuck_indexing_threshold: Duration,

    /// Enable automatic recovery for stuck operations
    pub enable_auto_recovery: bool,

    /// Storage configuration
    pub storage: StorageConfig,
}

impl Default for RepositoryManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_indexing: 3,
            max_concurrent_queries: 10,
            indexing_timeout: Duration::from_secs(300), // 5 minutes
            query_timeout: Duration::from_secs(30),
            retry_attempts: 3,
            retry_backoff: Duration::from_secs(1),
            max_retry_delay: Duration::from_secs(300), // 5 minutes
            backoff_multiplier: 2.0,
            enable_query_cache: true,
            query_cache_ttl: Duration::from_secs(300), // 5 minutes
            query_cache_size: 1000,
            health_check_interval: Duration::from_secs(30),
            stuck_indexing_threshold: Duration::from_secs(600), // 10 minutes
            enable_auto_recovery: true,
            storage: StorageConfig::default(),
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend: StorageBackend,

    /// Connection string or file path
    pub connection: String,

    /// Maximum number of database connections
    pub max_connections: u32,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Enable automatic migrations
    pub auto_migrate: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Sqlite,
            connection: "wikify_repositories.db".to_string(),
            max_connections: 10,
            connection_timeout: Duration::from_secs(5),
            auto_migrate: true,
        }
    }
}

/// Storage backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    /// In-memory storage (for testing)
    Memory,
    /// SQLite file database
    Sqlite,
    /// PostgreSQL database
    PostgreSQL,
    /// File system storage
    FileSystem,
}

/// Repository manager metrics
#[derive(Debug, Default)]
pub struct RepositoryMetrics {
    // Repository counts
    pub total_repositories: AtomicU64,
    pub pending_repositories: AtomicU64,
    pub indexing_repositories: AtomicU64,
    pub completed_repositories: AtomicU64,
    pub failed_repositories: AtomicU64,

    // Operation counts
    pub total_indexing_operations: AtomicU64,
    pub successful_indexing_operations: AtomicU64,
    pub failed_indexing_operations: AtomicU64,
    pub total_queries: AtomicU64,
    pub successful_queries: AtomicU64,
    pub failed_queries: AtomicU64,
    pub cached_queries: AtomicU64,

    // Performance metrics (in milliseconds)
    pub avg_indexing_time: AtomicU64,
    pub max_indexing_time: AtomicU64,
    pub min_indexing_time: AtomicU64,
    pub avg_query_time: AtomicU64,
    pub max_query_time: AtomicU64,
    pub min_query_time: AtomicU64,

    // Resource metrics
    pub active_indexing_operations: AtomicU32,
    pub active_query_operations: AtomicU32,
    pub queue_size: AtomicU32,
    pub memory_usage_bytes: AtomicU64,
    pub cache_hit_rate: AtomicU64, // Percentage * 100

    // Error metrics
    pub timeout_errors: AtomicU64,
    pub rate_limit_errors: AtomicU64,
    pub resource_exhaustion_errors: AtomicU64,
    pub permission_errors: AtomicU64,
    pub validation_errors: AtomicU64,
    pub internal_errors: AtomicU64,
}

impl RepositoryMetrics {
    /// Record a successful indexing operation
    pub fn record_indexing_success(&self, duration: Duration) {
        self.successful_indexing_operations
            .fetch_add(1, Ordering::Relaxed);
        self.active_indexing_operations
            .fetch_sub(1, Ordering::Relaxed);

        let duration_ms = duration.as_millis() as u64;
        self.update_indexing_time_stats(duration_ms);
    }

    /// Record a failed indexing operation
    pub fn record_indexing_failure(&self) {
        self.failed_indexing_operations
            .fetch_add(1, Ordering::Relaxed);
        self.active_indexing_operations
            .fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a successful query operation
    pub fn record_query_success(&self, duration: Duration, from_cache: bool) {
        self.successful_queries.fetch_add(1, Ordering::Relaxed);
        self.active_query_operations.fetch_sub(1, Ordering::Relaxed);

        if from_cache {
            self.cached_queries.fetch_add(1, Ordering::Relaxed);
        }

        let duration_ms = duration.as_millis() as u64;
        self.update_query_time_stats(duration_ms);
    }

    /// Record a failed query operation
    pub fn record_query_failure(&self) {
        self.failed_queries.fetch_add(1, Ordering::Relaxed);
        self.active_query_operations.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record an error by category
    pub fn record_error(&self, category: &str) {
        match category {
            "timeout" => self.timeout_errors.fetch_add(1, Ordering::Relaxed),
            "rate_limit" => self.rate_limit_errors.fetch_add(1, Ordering::Relaxed),
            "resource" => self
                .resource_exhaustion_errors
                .fetch_add(1, Ordering::Relaxed),
            "permission" => self.permission_errors.fetch_add(1, Ordering::Relaxed),
            "validation" => self.validation_errors.fetch_add(1, Ordering::Relaxed),
            "internal" => self.internal_errors.fetch_add(1, Ordering::Relaxed),
            _ => self.internal_errors.fetch_add(1, Ordering::Relaxed),
        };
    }

    /// Start tracking an indexing operation
    pub fn start_indexing(&self) {
        self.total_indexing_operations
            .fetch_add(1, Ordering::Relaxed);
        self.active_indexing_operations
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Start tracking a query operation
    pub fn start_query(&self) {
        self.total_queries.fetch_add(1, Ordering::Relaxed);
        self.active_query_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Update repository status counts
    pub fn update_repository_status(
        &self,
        old_status: Option<super::types::IndexingStatus>,
        new_status: super::types::IndexingStatus,
    ) {
        use super::types::IndexingStatus;

        // Decrement old status count
        if let Some(old) = old_status {
            match old {
                IndexingStatus::Pending => {
                    self.pending_repositories.fetch_sub(1, Ordering::Relaxed)
                }
                IndexingStatus::Indexing => {
                    self.indexing_repositories.fetch_sub(1, Ordering::Relaxed)
                }
                IndexingStatus::Completed => {
                    self.completed_repositories.fetch_sub(1, Ordering::Relaxed)
                }
                IndexingStatus::Failed => self.failed_repositories.fetch_sub(1, Ordering::Relaxed),
                IndexingStatus::Cancelled => 0, // Don't track cancelled separately
            };
        }

        // Increment new status count
        match new_status {
            IndexingStatus::Pending => self.pending_repositories.fetch_add(1, Ordering::Relaxed),
            IndexingStatus::Indexing => self.indexing_repositories.fetch_add(1, Ordering::Relaxed),
            IndexingStatus::Completed => {
                self.completed_repositories.fetch_add(1, Ordering::Relaxed)
            }
            IndexingStatus::Failed => self.failed_repositories.fetch_add(1, Ordering::Relaxed),
            IndexingStatus::Cancelled => 0, // Don't track cancelled separately
        };
    }

    /// Get current health status
    pub fn get_health_status(&self) -> HealthStatus {
        let total_ops = self.total_indexing_operations.load(Ordering::Relaxed)
            + self.total_queries.load(Ordering::Relaxed);
        let failed_ops = self.failed_indexing_operations.load(Ordering::Relaxed)
            + self.failed_queries.load(Ordering::Relaxed);

        if total_ops == 0 {
            return HealthStatus::Healthy;
        }

        let failure_rate = failed_ops as f64 / total_ops as f64;
        let active_ops = self.active_indexing_operations.load(Ordering::Relaxed)
            + self.active_query_operations.load(Ordering::Relaxed);

        if failure_rate > 0.2 || active_ops > 50 {
            HealthStatus::Unhealthy
        } else if failure_rate > 0.1 || active_ops > 20 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Get cache hit rate as percentage
    pub fn get_cache_hit_rate(&self) -> f64 {
        let total_queries = self.total_queries.load(Ordering::Relaxed);
        let cached_queries = self.cached_queries.load(Ordering::Relaxed);

        if total_queries == 0 {
            0.0
        } else {
            (cached_queries as f64 / total_queries as f64) * 100.0
        }
    }

    /// Update indexing time statistics
    fn update_indexing_time_stats(&self, duration_ms: u64) {
        // Simple implementation - in production, consider using a more sophisticated approach
        self.avg_indexing_time.store(duration_ms, Ordering::Relaxed);

        // Update max
        let current_max = self.max_indexing_time.load(Ordering::Relaxed);
        if duration_ms > current_max {
            self.max_indexing_time.store(duration_ms, Ordering::Relaxed);
        }

        // Update min (if not set or new value is smaller)
        let current_min = self.min_indexing_time.load(Ordering::Relaxed);
        if current_min == 0 || duration_ms < current_min {
            self.min_indexing_time.store(duration_ms, Ordering::Relaxed);
        }
    }

    /// Update query time statistics
    fn update_query_time_stats(&self, duration_ms: u64) {
        // Simple implementation - in production, consider using a more sophisticated approach
        self.avg_query_time.store(duration_ms, Ordering::Relaxed);

        // Update max
        let current_max = self.max_query_time.load(Ordering::Relaxed);
        if duration_ms > current_max {
            self.max_query_time.store(duration_ms, Ordering::Relaxed);
        }

        // Update min (if not set or new value is smaller)
        let current_min = self.min_query_time.load(Ordering::Relaxed);
        if current_min == 0 || duration_ms < current_min {
            self.min_query_time.store(duration_ms, Ordering::Relaxed);
        }
    }
}

/// System health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// System is operating normally
    Healthy,
    /// System is experiencing some issues but still functional
    Degraded,
    /// System is experiencing significant issues
    Unhealthy,
}

impl HealthStatus {
    /// Check if the status indicates the system is operational
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }

    /// Get HTTP status code for health checks
    pub fn http_status(&self) -> u16 {
        match self {
            Self::Healthy => 200,
            Self::Degraded => 200,  // Still operational
            Self::Unhealthy => 503, // Service unavailable
        }
    }
}
