//! Wiki cache management
//!
//! This module handles caching of generated wikis to avoid regeneration.

use crate::types::WikiStructure;
use chrono::{DateTime, Utc};
use serde_json;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};
use wikify_core::{ErrorContext, WikifyError, WikifyResult};

/// Cache manager for wiki structures
pub struct WikiCache {
    cache_dir: PathBuf,
}

impl WikiCache {
    /// Create a new WikiCache instance
    pub fn new() -> WikifyResult<Self> {
        let cache_dir = Self::get_cache_directory()?;
        Ok(Self { cache_dir })
    }

    /// Create a WikiCache with custom cache directory
    pub fn with_cache_dir<P: AsRef<Path>>(cache_dir: P) -> WikifyResult<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        Ok(Self { cache_dir })
    }

    /// Store a wiki structure in cache
    pub async fn store_wiki(&self, repo_path: &str, wiki: &WikiStructure) -> WikifyResult<()> {
        let cache_key = self.generate_cache_key(repo_path);
        let cache_file = self.cache_dir.join(format!("{}.json", cache_key));

        // Ensure cache directory exists
        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Serialize and write to file
        let json_content = serde_json::to_string_pretty(wiki).map_err(|e| WikifyError::Config {
            message: format!("Failed to serialize wiki structure: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("wiki_cache"),
        })?;

        fs::write(&cache_file, json_content).await?;

        info!(
            "Cached wiki for repository: {} -> {:?}",
            repo_path, cache_file
        );
        Ok(())
    }

    /// Retrieve a wiki structure from cache
    pub async fn get_wiki(&self, repo_path: &str) -> WikifyResult<Option<WikiStructure>> {
        let cache_key = self.generate_cache_key(repo_path);
        let cache_file = self.cache_dir.join(format!("{}.json", cache_key));

        if !cache_file.exists() {
            debug!("No cache found for repository: {}", repo_path);
            return Ok(None);
        }

        // Check if cache is still valid
        if !self.is_cache_valid(&cache_file, repo_path).await? {
            warn!("Cache is outdated for repository: {}", repo_path);
            // Optionally remove outdated cache
            let _ = fs::remove_file(&cache_file).await;
            return Ok(None);
        }

        // Read and deserialize cache file
        let json_content = fs::read_to_string(&cache_file).await?;

        let wiki_structure: WikiStructure =
            serde_json::from_str(&json_content).map_err(|e| WikifyError::Config {
                message: format!("Failed to deserialize wiki structure: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("wiki_cache"),
            })?;

        info!("Retrieved cached wiki for repository: {}", repo_path);
        Ok(Some(wiki_structure))
    }

    /// Clear cache for a specific repository
    pub async fn clear_wiki(&self, repo_path: &str) -> WikifyResult<()> {
        let cache_key = self.generate_cache_key(repo_path);
        let cache_file = self.cache_dir.join(format!("{}.json", cache_key));

        if cache_file.exists() {
            fs::remove_file(&cache_file).await?;
            info!("Cleared cache for repository: {}", repo_path);
        }

        Ok(())
    }

    /// List all cached wikis
    pub async fn list_wikis(&self) -> WikifyResult<Vec<String>> {
        if !self.cache_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&self.cache_dir).await?;

        let mut wikis = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Try to reverse the cache key to get original path
                    // This is a simple implementation - could be improved
                    wikis.push(file_stem.to_string());
                }
            }
        }

        Ok(wikis)
    }

    /// Clear all cached wikis
    pub async fn clear_all(&self) -> WikifyResult<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir).await?;
            info!("Cleared all wiki cache");
        }
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> WikifyResult<CacheStats> {
        if !self.cache_dir.exists() {
            return Ok(CacheStats::default());
        }

        let mut entries = fs::read_dir(&self.cache_dir).await?;

        let mut total_files = 0;
        let mut total_size = 0;
        let mut oldest_cache: Option<DateTime<Utc>> = None;
        let mut newest_cache: Option<DateTime<Utc>> = None;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                total_files += 1;

                if let Ok(metadata) = entry.metadata().await {
                    total_size += metadata.len();

                    if let Ok(modified) = metadata.modified() {
                        let modified_utc: DateTime<Utc> = modified.into();

                        if oldest_cache.is_none() || Some(modified_utc) < oldest_cache {
                            oldest_cache = Some(modified_utc);
                        }
                        if newest_cache.is_none() || Some(modified_utc) > newest_cache {
                            newest_cache = Some(modified_utc);
                        }
                    }
                }
            }
        }

        Ok(CacheStats {
            total_files,
            total_size_bytes: total_size,
            oldest_cache,
            newest_cache,
        })
    }

    /// Generate a cache key from repository path
    fn generate_cache_key(&self, repo_path: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        repo_path.hash(&mut hasher);
        format!("wiki_{:x}", hasher.finish())
    }

    /// Check if cache is still valid (not outdated)
    async fn is_cache_valid(&self, cache_file: &Path, repo_path: &str) -> WikifyResult<bool> {
        // Get cache file modification time
        let cache_metadata = fs::metadata(cache_file).await?;
        let cache_modified = cache_metadata.modified()?;

        // Check if any files in the repository are newer than the cache
        let repo_path = Path::new(repo_path);
        if !repo_path.exists() {
            return Ok(false);
        }

        // Simple check: compare with repository directory modification time
        // In a more sophisticated implementation, we could check individual files
        let repo_metadata = fs::metadata(repo_path).await?;
        let repo_modified = repo_metadata.modified()?;

        // Cache is valid if it's newer than the repository
        Ok(cache_modified >= repo_modified)
    }

    /// Get the default cache directory
    fn get_cache_directory() -> WikifyResult<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .or_else(|| dirs::home_dir().map(|d| d.join(".cache")))
            .ok_or_else(|| WikifyError::Config {
                message: "Could not determine cache directory".to_string(),
                source: None,
                context: ErrorContext::new("wiki_cache"),
            })?
            .join("wikify")
            .join("wiki");

        Ok(cache_dir)
    }
}

/// Statistics about the wiki cache
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total number of cached wiki files
    pub total_files: usize,
    /// Total size of cache in bytes
    pub total_size_bytes: u64,
    /// Oldest cache entry
    pub oldest_cache: Option<DateTime<Utc>>,
    /// Newest cache entry
    pub newest_cache: Option<DateTime<Utc>>,
}

impl CacheStats {
    /// Get total cache size in human-readable format
    pub fn total_size_human(&self) -> String {
        let size = self.total_size_bytes as f64;
        if size < 1024.0 {
            format!("{} B", size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cache_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = WikiCache::with_cache_dir(temp_dir.path()).unwrap();

        let stats = cache.get_cache_stats().await.unwrap();
        assert_eq!(stats.total_files, 0);
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let cache = WikiCache::new().unwrap();
        let key1 = cache.generate_cache_key("/path/to/repo");
        let key2 = cache.generate_cache_key("/path/to/repo");
        let key3 = cache.generate_cache_key("/different/path");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
