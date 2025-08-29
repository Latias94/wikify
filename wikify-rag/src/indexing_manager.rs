//! Indexing Manager - Controls concurrent indexing operations
//!
//! This module provides centralized management of indexing operations with
//! concurrency control to prevent system resource exhaustion.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Manages concurrent indexing operations with resource control
pub struct IndexingManager {
    /// Controls the maximum number of concurrent indexing operations
    indexing_semaphore: Arc<Semaphore>,
    /// Tracks currently active indexing tasks by session_id
    active_indexing: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    /// Tracks repository paths being indexed (normalized paths)
    active_repositories: Arc<RwLock<HashMap<PathBuf, String>>>, // path -> session_id
    /// Maximum concurrent indexing operations allowed
    max_concurrent: usize,
}

impl IndexingManager {
    /// Create a new IndexingManager with specified concurrency limit
    pub fn new(max_concurrent_indexing: usize) -> Self {
        info!(
            "Creating IndexingManager with max_concurrent: {}",
            max_concurrent_indexing
        );

        Self {
            indexing_semaphore: Arc::new(Semaphore::new(max_concurrent_indexing)),
            active_indexing: Arc::new(RwLock::new(HashMap::new())),
            active_repositories: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent: max_concurrent_indexing,
        }
    }

    /// Normalize repository path for consistent comparison
    fn normalize_repo_path(repo_path: &str) -> PathBuf {
        // Convert to absolute path and canonicalize if possible
        let path = Path::new(repo_path);

        // Try to canonicalize (resolve symlinks, etc.)
        if let Ok(canonical) = path.canonicalize() {
            canonical
        } else {
            // If canonicalize fails (e.g., path doesn't exist yet),
            // just convert to absolute path
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("/"))
                    .join(path)
            }
        }
    }

    /// Check if a new indexing task can be started for the given session
    pub async fn can_start_indexing(&self, session_id: &str) -> bool {
        let active = self.active_indexing.read().await;
        let can_start = !active.contains_key(session_id);

        debug!(
            "Can start indexing for session {}: {} (active tasks: {})",
            session_id,
            can_start,
            active.len()
        );

        can_start
    }

    /// Check if a session is currently being indexed
    pub async fn is_indexing(&self, session_id: &str) -> bool {
        let active = self.active_indexing.read().await;
        active.contains_key(session_id)
    }

    /// Check if a repository path is currently being indexed
    pub async fn is_repository_indexing(&self, repo_path: &str) -> Option<String> {
        let normalized_path = Self::normalize_repo_path(repo_path);
        let active_repos = self.active_repositories.read().await;
        active_repos.get(&normalized_path).cloned()
    }

    /// Check if indexing can start for both session and repository
    pub async fn can_start_indexing_with_repo_check(
        &self,
        session_id: &str,
        repo_path: &str,
    ) -> Result<(), String> {
        // Check if session is already indexing
        if self.is_indexing(session_id).await {
            return Err(format!("Session {} is already being indexed", session_id));
        }

        // Check if repository is already being indexed by another session
        if let Some(existing_session) = self.is_repository_indexing(repo_path).await {
            if existing_session != session_id {
                return Err(format!(
                    "Repository {} is already being indexed by session {}",
                    repo_path, existing_session
                ));
            }
        }

        Ok(())
    }

    /// Start an indexing task with repository path checking and concurrency control
    pub async fn start_indexing_with_repo_check<F, Fut>(
        &self,
        session_id: String,
        repo_path: String,
        indexing_task: F,
    ) -> Result<(), String>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        // Check both session and repository conflicts
        self.can_start_indexing_with_repo_check(&session_id, &repo_path)
            .await?;

        let normalized_path = Self::normalize_repo_path(&repo_path);
        let semaphore = self.indexing_semaphore.clone();
        let active_indexing = self.active_indexing.clone();
        let active_repositories = self.active_repositories.clone();
        let session_id_clone = session_id.clone();
        let normalized_path_clone = normalized_path.clone();

        info!(
            "Starting indexing task for session {} with repository {} (available permits: {})",
            session_id,
            repo_path,
            semaphore.available_permits()
        );

        // Spawn background task with concurrency control
        let handle = tokio::spawn(async move {
            // Acquire semaphore permit to control concurrency
            let _permit = match semaphore.acquire().await {
                Ok(permit) => {
                    debug!("Acquired indexing permit for session {}", session_id_clone);
                    permit
                }
                Err(e) => {
                    warn!(
                        "Failed to acquire indexing permit for session {}: {}",
                        session_id_clone, e
                    );
                    return;
                }
            };

            // Execute the indexing task
            debug!("Executing indexing task for session {}", session_id_clone);
            indexing_task().await;
            debug!("Completed indexing task for session {}", session_id_clone);

            // Task completed, remove from both active lists
            {
                let mut active = active_indexing.write().await;
                active.remove(&session_id_clone);
                debug!(
                    "Removed session {} from active indexing list",
                    session_id_clone
                );
            }
            {
                let mut active_repos = active_repositories.write().await;
                active_repos.remove(&normalized_path_clone);
                debug!(
                    "Removed repository {:?} from active repositories list",
                    normalized_path_clone
                );
            }
        });

        // Record the active indexing task and repository
        {
            let mut active = self.active_indexing.write().await;
            active.insert(session_id.clone(), handle);
            info!(
                "Added session {} to active indexing list (total active: {})",
                session_id,
                active.len()
            );
        }
        {
            let mut active_repos = self.active_repositories.write().await;
            active_repos.insert(normalized_path, session_id.clone());
            debug!("Added repository {} to active repositories list", repo_path);
        }

        Ok(())
    }

    /// Start an indexing task with concurrency control
    pub async fn start_indexing<F, Fut>(
        &self,
        session_id: String,
        indexing_task: F,
    ) -> Result<(), String>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        // Check if already indexing (basic session check)
        if !self.can_start_indexing(&session_id).await {
            warn!(
                "Attempted to start indexing for session {} that is already being indexed",
                session_id
            );
            return Err("Session is already being indexed".to_string());
        }

        let semaphore = self.indexing_semaphore.clone();
        let active_indexing = self.active_indexing.clone();
        let session_id_clone = session_id.clone();

        info!(
            "Starting indexing task for session {} (available permits: {})",
            session_id,
            semaphore.available_permits()
        );

        // Spawn background task with concurrency control
        let handle = tokio::spawn(async move {
            // Acquire semaphore permit to control concurrency
            let _permit = match semaphore.acquire().await {
                Ok(permit) => {
                    debug!("Acquired indexing permit for session {}", session_id_clone);
                    permit
                }
                Err(e) => {
                    warn!(
                        "Failed to acquire indexing permit for session {}: {}",
                        session_id_clone, e
                    );
                    return;
                }
            };

            // Execute the indexing task
            debug!("Executing indexing task for session {}", session_id_clone);
            indexing_task().await;
            debug!("Completed indexing task for session {}", session_id_clone);

            // Task completed, remove from active list
            {
                let mut active = active_indexing.write().await;
                active.remove(&session_id_clone);
                debug!(
                    "Removed session {} from active indexing list",
                    session_id_clone
                );
            }
        });

        // Record the active indexing task
        {
            let mut active = self.active_indexing.write().await;
            active.insert(session_id.clone(), handle);
            info!(
                "Added session {} to active indexing list (total active: {})",
                session_id,
                active.len()
            );
        }

        Ok(())
    }

    /// Cancel an active indexing task
    pub async fn cancel_indexing(&self, session_id: &str) -> bool {
        let mut active = self.active_indexing.write().await;

        if let Some(handle) = active.remove(session_id) {
            handle.abort();
            info!("Cancelled indexing task for session {}", session_id);

            // Also remove from active repositories
            {
                let mut active_repos = self.active_repositories.write().await;
                // Find and remove the repository entry for this session
                active_repos.retain(|_path, session| session != session_id);
                debug!(
                    "Removed repository entries for cancelled session {}",
                    session_id
                );
            }

            true
        } else {
            debug!("No active indexing task found for session {}", session_id);
            false
        }
    }

    /// Get the number of currently active indexing tasks
    pub async fn active_count(&self) -> usize {
        self.active_indexing.read().await.len()
    }

    /// Get the maximum number of concurrent indexing operations allowed
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Get the number of available indexing permits
    pub fn available_permits(&self) -> usize {
        self.indexing_semaphore.available_permits()
    }

    /// Get a list of currently active session IDs
    pub async fn active_sessions(&self) -> Vec<String> {
        let active = self.active_indexing.read().await;
        active.keys().cloned().collect()
    }

    /// Clean up completed tasks (removes finished JoinHandles)
    pub async fn cleanup_completed_tasks(&self) {
        let mut active = self.active_indexing.write().await;
        let mut completed_sessions = Vec::new();

        for (session_id, handle) in active.iter() {
            if handle.is_finished() {
                completed_sessions.push(session_id.clone());
            }
        }

        for session_id in &completed_sessions {
            active.remove(session_id);
            debug!(
                "Cleaned up completed indexing task for session {}",
                session_id
            );
        }

        // Also clean up completed repositories
        if !completed_sessions.is_empty() {
            let mut active_repos = self.active_repositories.write().await;
            for session_id in completed_sessions {
                active_repos.retain(|_path, session| session != &session_id);
            }
        }
    }
}

impl Default for IndexingManager {
    fn default() -> Self {
        Self::new(2) // Default to 2 concurrent indexing operations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_indexing_manager_basic() {
        let manager = IndexingManager::new(2);

        assert_eq!(manager.max_concurrent(), 2);
        assert_eq!(manager.active_count().await, 0);
        assert!(manager.can_start_indexing("session1").await);
    }

    #[tokio::test]
    async fn test_concurrency_control() {
        let manager = Arc::new(IndexingManager::new(1));

        // Start first task
        let manager1 = manager.clone();
        let result1 = manager1
            .start_indexing("session1".to_string(), || async {
                sleep(Duration::from_millis(100)).await;
            })
            .await;
        assert!(result1.is_ok());

        // Try to start second task for same session (should fail)
        let result2 = manager
            .start_indexing("session1".to_string(), || async {
                sleep(Duration::from_millis(50)).await;
            })
            .await;
        assert!(result2.is_err());

        // Wait for first task to complete
        sleep(Duration::from_millis(150)).await;
        manager.cleanup_completed_tasks().await;

        // Now should be able to start again
        assert!(manager.can_start_indexing("session1").await);
    }

    #[tokio::test]
    async fn test_cancel_indexing() {
        let manager = IndexingManager::new(2);

        // Start a long-running task
        let result = manager
            .start_indexing("session1".to_string(), || async {
                sleep(Duration::from_secs(10)).await; // Long task
            })
            .await;
        assert!(result.is_ok());

        // Verify it's active
        assert!(manager.is_indexing("session1").await);

        // Cancel it
        assert!(manager.cancel_indexing("session1").await);

        // Verify it's no longer active
        sleep(Duration::from_millis(10)).await; // Give time for cleanup
        assert!(!manager.is_indexing("session1").await);
    }

    #[tokio::test]
    async fn test_repository_path_checking() {
        let manager = IndexingManager::new(2);

        // Start indexing for a repository
        let result = manager
            .start_indexing_with_repo_check(
                "session1".to_string(),
                "/path/to/repo".to_string(),
                || async {
                    sleep(Duration::from_millis(100)).await;
                },
            )
            .await;
        assert!(result.is_ok());

        // Try to start indexing the same repository with different session (should fail)
        let result2 = manager
            .start_indexing_with_repo_check(
                "session2".to_string(),
                "/path/to/repo".to_string(),
                || async {
                    sleep(Duration::from_millis(50)).await;
                },
            )
            .await;
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("already being indexed"));

        // Wait for first task to complete
        sleep(Duration::from_millis(150)).await;
        manager.cleanup_completed_tasks().await;

        // Now should be able to start indexing the same repository again
        let result3 = manager
            .start_indexing_with_repo_check(
                "session3".to_string(),
                "/path/to/repo".to_string(),
                || async {
                    sleep(Duration::from_millis(50)).await;
                },
            )
            .await;
        assert!(result3.is_ok());
    }

    #[tokio::test]
    async fn test_path_normalization() {
        let manager = IndexingManager::new(2);

        // Test that different representations of the same path are detected
        let result1 = manager
            .start_indexing_with_repo_check(
                "session1".to_string(),
                "./test_repo".to_string(),
                || async {
                    sleep(Duration::from_millis(100)).await;
                },
            )
            .await;
        assert!(result1.is_ok());

        // Try with a different representation of the same path
        let current_dir = std::env::current_dir().unwrap();
        let absolute_path = current_dir.join("test_repo");
        let result2 = manager
            .start_indexing_with_repo_check(
                "session2".to_string(),
                absolute_path.to_string_lossy().to_string(),
                || async {
                    sleep(Duration::from_millis(50)).await;
                },
            )
            .await;

        // This should fail because it's the same repository
        assert!(result2.is_err());
    }
}
