//! Repository storage backends
//!
//! Provides different storage implementations for repository persistence,
//! including in-memory, SQLite, and other database backends.

use super::errors::{RepositoryError, RepositoryResult};
use super::types::{IndexingStatus, RepositoryIndex};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Repository storage trait for different persistence backends
#[async_trait]
pub trait RepositoryStorage: Send + Sync {
    /// Save or update a repository
    async fn save_repository(&self, repo: &RepositoryIndex) -> RepositoryResult<()>;

    /// Load a repository by ID
    async fn load_repository(&self, id: &str) -> RepositoryResult<Option<RepositoryIndex>>;

    /// List all repositories, optionally filtered by owner
    async fn list_repositories(
        &self,
        owner_id: Option<&str>,
    ) -> RepositoryResult<Vec<RepositoryIndex>>;

    /// Delete a repository
    async fn delete_repository(&self, id: &str) -> RepositoryResult<()>;

    /// Update repository status and progress
    async fn update_status(
        &self,
        id: &str,
        status: IndexingStatus,
        progress: f64,
    ) -> RepositoryResult<()>;

    /// Update repository metadata
    async fn update_metadata(
        &self,
        id: &str,
        metadata: HashMap<String, String>,
    ) -> RepositoryResult<()>;

    /// Get repositories by status
    async fn get_repositories_by_status(
        &self,
        status: IndexingStatus,
    ) -> RepositoryResult<Vec<RepositoryIndex>>;

    /// Get repository count by status
    async fn get_status_counts(&self) -> RepositoryResult<HashMap<IndexingStatus, u64>>;

    /// Health check for the storage backend
    async fn health_check(&self) -> RepositoryResult<()>;
}

/// In-memory repository storage (default implementation)
pub struct MemoryRepositoryStorage {
    repositories: Arc<RwLock<HashMap<String, RepositoryIndex>>>,
}

impl MemoryRepositoryStorage {
    pub fn new() -> Self {
        Self {
            repositories: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load repositories from another storage backend (for migration)
    pub async fn load_from<S: RepositoryStorage>(&self, source: &S) -> RepositoryResult<usize> {
        let repositories = source.list_repositories(None).await?;
        let count = repositories.len();

        let mut storage = self.repositories.write().await;
        for repo in repositories {
            storage.insert(repo.id.clone(), repo);
        }

        info!("Loaded {} repositories into memory storage", count);
        Ok(count)
    }
}

impl Default for MemoryRepositoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RepositoryStorage for MemoryRepositoryStorage {
    async fn save_repository(&self, repo: &RepositoryIndex) -> RepositoryResult<()> {
        let mut repositories = self.repositories.write().await;
        repositories.insert(repo.id.clone(), repo.clone());
        debug!("Saved repository {} to memory storage", repo.id);
        Ok(())
    }

    async fn load_repository(&self, id: &str) -> RepositoryResult<Option<RepositoryIndex>> {
        let repositories = self.repositories.read().await;
        Ok(repositories.get(id).cloned())
    }

    async fn list_repositories(
        &self,
        owner_id: Option<&str>,
    ) -> RepositoryResult<Vec<RepositoryIndex>> {
        let repositories = self.repositories.read().await;
        let repos: Vec<RepositoryIndex> = repositories
            .values()
            .filter(|repo| {
                owner_id.map_or(true, |owner| {
                    repo.owner_id
                        .as_ref()
                        .map_or(false, |repo_owner| repo_owner == owner)
                })
            })
            .cloned()
            .collect();
        Ok(repos)
    }

    async fn delete_repository(&self, id: &str) -> RepositoryResult<()> {
        let mut repositories = self.repositories.write().await;
        if repositories.remove(id).is_some() {
            debug!("Deleted repository {} from memory storage", id);
            Ok(())
        } else {
            Err(RepositoryError::NotFound {
                repository_id: id.to_string(),
            })
        }
    }

    async fn update_status(
        &self,
        id: &str,
        status: IndexingStatus,
        progress: f64,
    ) -> RepositoryResult<()> {
        let mut repositories = self.repositories.write().await;
        if let Some(repo) = repositories.get_mut(id) {
            repo.set_progress(progress, Some(status.clone()));
            debug!(
                "Updated repository {} status to {:?} with progress {}",
                id, status, progress
            );
            Ok(())
        } else {
            Err(RepositoryError::NotFound {
                repository_id: id.to_string(),
            })
        }
    }

    async fn update_metadata(
        &self,
        id: &str,
        metadata: HashMap<String, String>,
    ) -> RepositoryResult<()> {
        let mut repositories = self.repositories.write().await;
        if let Some(repo) = repositories.get_mut(id) {
            repo.metadata = metadata;
            repo.updated_at = Utc::now();
            debug!("Updated repository {} metadata", id);
            Ok(())
        } else {
            Err(RepositoryError::NotFound {
                repository_id: id.to_string(),
            })
        }
    }

    async fn get_repositories_by_status(
        &self,
        status: IndexingStatus,
    ) -> RepositoryResult<Vec<RepositoryIndex>> {
        let repositories = self.repositories.read().await;
        let repos: Vec<RepositoryIndex> = repositories
            .values()
            .filter(|repo| repo.status == status)
            .cloned()
            .collect();
        Ok(repos)
    }

    async fn get_status_counts(&self) -> RepositoryResult<HashMap<IndexingStatus, u64>> {
        let repositories = self.repositories.read().await;
        let mut counts = HashMap::new();

        for repo in repositories.values() {
            *counts.entry(repo.status.clone()).or_insert(0) += 1;
        }

        Ok(counts)
    }

    async fn health_check(&self) -> RepositoryResult<()> {
        // Memory storage is always healthy
        Ok(())
    }
}

/// SQLite repository storage implementation
#[cfg(feature = "sqlite")]
pub struct SqliteRepositoryStorage {
    pool: sqlx::SqlitePool,
}

#[cfg(feature = "sqlite")]
impl SqliteRepositoryStorage {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    /// Create from database URL
    pub async fn from_url(database_url: &str) -> RepositoryResult<Self> {
        let pool = sqlx::SqlitePool::connect(database_url).await.map_err(|e| {
            RepositoryError::Internal {
                message: format!("Failed to connect to SQLite database: {}", e),
                component: "sqlite_storage".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: true,
            }
        })?;

        Ok(Self::new(pool))
    }

    /// Run database migrations
    pub async fn migrate(&self) -> RepositoryResult<()> {
        sqlx::migrate!("../wikify-web/migrations")
            .run(&self.pool)
            .await
            .map_err(|e| RepositoryError::Internal {
                message: format!("Database migration failed: {}", e),
                component: "sqlite_storage".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: false,
            })?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    /// Convert database row to RepositoryIndex
    fn row_to_repository_index(row: &sqlx::sqlite::SqliteRow) -> RepositoryResult<RepositoryIndex> {
        use sqlx::Row;

        let metadata_json: String =
            row.try_get("metadata")
                .map_err(|e| RepositoryError::Internal {
                    message: format!("Failed to get metadata column: {}", e),
                    component: "sqlite_storage".to_string(),
                    error_id: uuid::Uuid::new_v4().to_string(),
                    recoverable: false,
                })?;

        let metadata: HashMap<String, String> =
            serde_json::from_str(&metadata_json).map_err(|e| RepositoryError::Internal {
                message: format!("Failed to parse metadata JSON: {}", e),
                component: "sqlite_storage".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: false,
            })?;

        let status_str: String = row
            .try_get("status")
            .map_err(|e| RepositoryError::Internal {
                message: format!("Failed to get status column: {}", e),
                component: "sqlite_storage".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: false,
            })?;

        let status = match status_str.as_str() {
            "created" => IndexingStatus::Pending,
            "indexing" => IndexingStatus::Indexing,
            "indexed" => IndexingStatus::Completed,
            "failed" => IndexingStatus::Failed,
            "archived" => IndexingStatus::Cancelled,
            _ => IndexingStatus::Pending,
        };

        Ok(RepositoryIndex {
            id: row.try_get("id").map_err(|e| RepositoryError::Internal {
                message: format!("Failed to get id column: {}", e),
                component: "sqlite_storage".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: false,
            })?,
            url: row
                .try_get("repo_path")
                .map_err(|e| RepositoryError::Internal {
                    message: format!("Failed to get repo_path column: {}", e),
                    component: "sqlite_storage".to_string(),
                    error_id: uuid::Uuid::new_v4().to_string(),
                    recoverable: false,
                })?,
            repo_type: row
                .try_get("repo_type")
                .map_err(|e| RepositoryError::Internal {
                    message: format!("Failed to get repo_type column: {}", e),
                    component: "sqlite_storage".to_string(),
                    error_id: uuid::Uuid::new_v4().to_string(),
                    recoverable: false,
                })?,
            status,
            progress: 0.0, // Not stored in current schema, will be calculated
            created_at: row
                .try_get("created_at")
                .map_err(|e| RepositoryError::Internal {
                    message: format!("Failed to get created_at column: {}", e),
                    component: "sqlite_storage".to_string(),
                    error_id: uuid::Uuid::new_v4().to_string(),
                    recoverable: false,
                })?,
            indexed_at: row.try_get("last_indexed_at").ok(),
            updated_at: Utc::now(), // Will be set to current time
            owner_id: Some("default".to_string()), // Default owner for now
            metadata,
        })
    }

    /// Convert IndexingStatus to database status string
    fn status_to_db_string(status: &IndexingStatus) -> &'static str {
        match status {
            IndexingStatus::Pending => "created",
            IndexingStatus::Indexing => "indexing",
            IndexingStatus::Completed => "indexed",
            IndexingStatus::Failed => "failed",
            IndexingStatus::Cancelled => "archived",
        }
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl RepositoryStorage for SqliteRepositoryStorage {
    async fn save_repository(&self, repo: &RepositoryIndex) -> RepositoryResult<()> {
        let metadata_json =
            serde_json::to_string(&repo.metadata).map_err(|e| RepositoryError::Internal {
                message: format!("Failed to serialize metadata: {}", e),
                component: "sqlite_storage".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: false,
            })?;

        let status_str = Self::status_to_db_string(&repo.status);

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO repositories
            (id, name, description, repo_path, repo_type, created_at, last_indexed_at, status, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&repo.id)
        .bind(&repo.url) // Using URL as name for now
        .bind("") // Empty description
        .bind(&repo.url)
        .bind(&repo.repo_type)
        .bind(repo.created_at)
        .bind(repo.indexed_at)
        .bind(status_str)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Internal {
            message: format!("Failed to save repository to database: {}", e),
            component: "sqlite_storage".to_string(),
            error_id: uuid::Uuid::new_v4().to_string(),
            recoverable: true,
        })?;

        debug!("Saved repository {} to SQLite storage", repo.id);
        Ok(())
    }

    async fn load_repository(&self, id: &str) -> RepositoryResult<Option<RepositoryIndex>> {
        let row = sqlx::query(
            "SELECT id, name, description, repo_path, repo_type, created_at, last_indexed_at, status, metadata FROM repositories WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Internal {
            message: format!("Failed to load repository from database: {}", e),
            component: "sqlite_storage".to_string(),
            error_id: uuid::Uuid::new_v4().to_string(),
            recoverable: true,
        })?;

        if let Some(row) = row {
            let repo = Self::row_to_repository_index(&row)?;
            debug!("Loaded repository {} from SQLite storage", id);
            Ok(Some(repo))
        } else {
            Ok(None)
        }
    }

    async fn list_repositories(
        &self,
        owner_id: Option<&str>,
    ) -> RepositoryResult<Vec<RepositoryIndex>> {
        let rows = sqlx::query(
            "SELECT id, name, description, repo_path, repo_type, created_at, last_indexed_at, status, metadata FROM repositories ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Internal {
            message: format!("Failed to list repositories from database: {}", e),
            component: "sqlite_storage".to_string(),
            error_id: uuid::Uuid::new_v4().to_string(),
            recoverable: true,
        })?;

        let mut repositories = Vec::new();
        for row in rows {
            match Self::row_to_repository_index(&row) {
                Ok(repo) => repositories.push(repo),
                Err(e) => {
                    warn!("Failed to parse repository row: {}", e);
                    continue;
                }
            }
        }

        debug!(
            "Listed {} repositories from SQLite storage",
            repositories.len()
        );
        Ok(repositories)
    }

    async fn delete_repository(&self, id: &str) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM repositories WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Internal {
                message: format!("Failed to delete repository from database: {}", e),
                component: "sqlite_storage".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: true,
            })?;

        if result.rows_affected() > 0 {
            debug!("Deleted repository {} from SQLite storage", id);
            Ok(())
        } else {
            Err(RepositoryError::NotFound {
                repository_id: id.to_string(),
            })
        }
    }

    async fn update_status(
        &self,
        id: &str,
        status: IndexingStatus,
        progress: f64,
    ) -> RepositoryResult<()> {
        let status_str = Self::status_to_db_string(&status);
        let indexed_at = if matches!(status, IndexingStatus::Completed) {
            Some(Utc::now())
        } else {
            None
        };

        let result =
            sqlx::query("UPDATE repositories SET status = ?, last_indexed_at = ? WHERE id = ?")
                .bind(status_str)
                .bind(indexed_at)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(|e| RepositoryError::Internal {
                    message: format!("Failed to update repository status in database: {}", e),
                    component: "sqlite_storage".to_string(),
                    error_id: uuid::Uuid::new_v4().to_string(),
                    recoverable: true,
                })?;

        if result.rows_affected() > 0 {
            debug!(
                "Updated repository {} status to {:?} with progress {} in SQLite storage",
                id, status, progress
            );
            Ok(())
        } else {
            Err(RepositoryError::NotFound {
                repository_id: id.to_string(),
            })
        }
    }

    async fn update_metadata(
        &self,
        id: &str,
        metadata: HashMap<String, String>,
    ) -> RepositoryResult<()> {
        let metadata_json =
            serde_json::to_string(&metadata).map_err(|e| RepositoryError::Internal {
                message: format!("Failed to serialize metadata: {}", e),
                component: "sqlite_storage".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: false,
            })?;

        let result = sqlx::query("UPDATE repositories SET metadata = ? WHERE id = ?")
            .bind(metadata_json)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Internal {
                message: format!("Failed to update repository metadata in database: {}", e),
                component: "sqlite_storage".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: true,
            })?;

        if result.rows_affected() > 0 {
            debug!("Updated repository {} metadata in SQLite storage", id);
            Ok(())
        } else {
            Err(RepositoryError::NotFound {
                repository_id: id.to_string(),
            })
        }
    }

    async fn get_repositories_by_status(
        &self,
        status: IndexingStatus,
    ) -> RepositoryResult<Vec<RepositoryIndex>> {
        let status_str = Self::status_to_db_string(&status);

        let rows = sqlx::query(
            "SELECT id, name, description, repo_path, repo_type, created_at, last_indexed_at, status, metadata FROM repositories WHERE status = ? ORDER BY created_at DESC"
        )
        .bind(status_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Internal {
            message: format!("Failed to get repositories by status from database: {}", e),
            component: "sqlite_storage".to_string(),
            error_id: uuid::Uuid::new_v4().to_string(),
            recoverable: true,
        })?;

        let mut repositories = Vec::new();
        for row in rows {
            match Self::row_to_repository_index(&row) {
                Ok(repo) => repositories.push(repo),
                Err(e) => {
                    warn!("Failed to parse repository row: {}", e);
                    continue;
                }
            }
        }

        debug!(
            "Found {} repositories with status {:?} in SQLite storage",
            repositories.len(),
            status
        );
        Ok(repositories)
    }

    async fn get_status_counts(&self) -> RepositoryResult<HashMap<IndexingStatus, u64>> {
        let rows =
            sqlx::query("SELECT status, COUNT(*) as count FROM repositories GROUP BY status")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| RepositoryError::Internal {
                    message: format!("Failed to get status counts from database: {}", e),
                    component: "sqlite_storage".to_string(),
                    error_id: uuid::Uuid::new_v4().to_string(),
                    recoverable: true,
                })?;

        let mut counts = HashMap::new();
        for row in rows {
            use sqlx::Row;
            let status_str: String = row.try_get("status").unwrap_or_default();
            let count: i64 = row.try_get("count").unwrap_or(0);

            let status = match status_str.as_str() {
                "created" => IndexingStatus::Pending,
                "indexing" => IndexingStatus::Indexing,
                "indexed" => IndexingStatus::Completed,
                "failed" => IndexingStatus::Failed,
                "archived" => IndexingStatus::Cancelled,
                _ => continue,
            };
            counts.insert(status, count as u64);
        }

        debug!("Retrieved status counts from SQLite storage: {:?}", counts);
        Ok(counts)
    }

    async fn health_check(&self) -> RepositoryResult<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::Internal {
                message: format!("SQLite health check failed: {}", e),
                component: "sqlite_storage".to_string(),
                error_id: uuid::Uuid::new_v4().to_string(),
                recoverable: true,
            })?;

        Ok(())
    }
}
