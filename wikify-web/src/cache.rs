//! Caching system for Wikify Web Server
//!
//! This module provides in-memory and persistent caching for improved performance.

use crate::{WebError, WebResult};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Cache entry with expiration
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub value: T,
    pub created_at: Instant,
    pub expires_at: Option<Instant>,
    pub access_count: u64,
    pub last_accessed: Instant,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            expires_at: ttl.map(|duration| now + duration),
            access_count: 0,
            last_accessed: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|expires_at| Instant::now() > expires_at)
            .unwrap_or(false)
    }

    pub fn access(&mut self) -> &T {
        self.access_count += 1;
        self.last_accessed = Instant::now();
        &self.value
    }
}

/// In-memory cache with TTL and LRU eviction
#[derive(Debug)]
pub struct MemoryCache<K, V> {
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    max_size: usize,
    default_ttl: Option<Duration>,
}

impl<K, V> MemoryCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(max_size: usize, default_ttl: Option<Duration>) -> Self {
        let cache = Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            default_ttl,
        };

        // Start cleanup task
        cache.start_cleanup_task();
        cache
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().unwrap();
        
        if let Some(entry) = entries.get_mut(key) {
            if entry.is_expired() {
                entries.remove(key);
                debug!("Cache entry expired and removed: {:?}", key);
                return None;
            }
            
            let value = entry.access().clone();
            debug!("Cache hit for key: {:?}", key);
            Some(value)
        } else {
            debug!("Cache miss for key: {:?}", key);
            None
        }
    }

    pub fn set(&self, key: K, value: V, ttl: Option<Duration>) -> WebResult<()> {
        let mut entries = self.entries.write().unwrap();
        
        // Check if we need to evict entries
        if entries.len() >= self.max_size && !entries.contains_key(&key) {
            self.evict_lru(&mut entries);
        }
        
        let ttl = ttl.or(self.default_ttl);
        let entry = CacheEntry::new(value, ttl);
        entries.insert(key, entry);
        
        Ok(())
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().unwrap();
        entries.remove(key).map(|entry| entry.value)
    }

    pub fn clear(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.clear();
        info!("Cache cleared");
    }

    pub fn size(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.len()
    }

    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.read().unwrap();
        let mut total_access_count = 0;
        let mut expired_count = 0;
        
        for entry in entries.values() {
            total_access_count += entry.access_count;
            if entry.is_expired() {
                expired_count += 1;
            }
        }
        
        CacheStats {
            total_entries: entries.len(),
            expired_entries: expired_count,
            total_access_count,
            max_size: self.max_size,
        }
    }

    fn evict_lru(&self, entries: &mut HashMap<K, CacheEntry<V>>) {
        if entries.is_empty() {
            return;
        }

        // Find the least recently used entry
        let lru_key = entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone());

        if let Some(key) = lru_key {
            entries.remove(&key);
            debug!("Evicted LRU cache entry: {:?}", key);
        }
    }

    fn start_cleanup_task(&self) {
        let entries = Arc::clone(&self.entries);
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // Cleanup every 5 minutes
            
            loop {
                interval.tick().await;
                
                let mut entries = entries.write().unwrap();
                let initial_size = entries.len();
                
                entries.retain(|_, entry| !entry.is_expired());
                
                let removed_count = initial_size - entries.len();
                if removed_count > 0 {
                    info!("Cache cleanup: removed {} expired entries", removed_count);
                }
            }
        });
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub total_access_count: u64,
    pub max_size: usize,
}

/// Application-specific cache manager
#[derive(Debug)]
pub struct CacheManager {
    /// Cache for repository information
    pub repositories: MemoryCache<String, wikify_core::types::RepositoryInfo>,
    /// Cache for chat responses
    pub chat_responses: MemoryCache<String, String>,
    /// Cache for wiki content
    pub wiki_content: MemoryCache<String, serde_json::Value>,
    /// Cache for file content
    pub file_content: MemoryCache<String, String>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            repositories: MemoryCache::new(100, Some(Duration::from_secs(3600))), // 1 hour
            chat_responses: MemoryCache::new(1000, Some(Duration::from_secs(1800))), // 30 minutes
            wiki_content: MemoryCache::new(50, Some(Duration::from_secs(7200))), // 2 hours
            file_content: MemoryCache::new(500, Some(Duration::from_secs(600))), // 10 minutes
        }
    }

    pub fn get_overall_stats(&self) -> HashMap<String, CacheStats> {
        let mut stats = HashMap::new();
        stats.insert("repositories".to_string(), self.repositories.stats());
        stats.insert("chat_responses".to_string(), self.chat_responses.stats());
        stats.insert("wiki_content".to_string(), self.wiki_content.stats());
        stats.insert("file_content".to_string(), self.file_content.stats());
        stats
    }

    pub fn clear_all(&self) {
        self.repositories.clear();
        self.chat_responses.clear();
        self.wiki_content.clear();
        self.file_content.clear();
        info!("All caches cleared");
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache key generators
pub mod keys {
    use sha2::{Digest, Sha256};

    pub fn repository_key(repo_url: &str) -> String {
        format!("repo:{}", repo_url)
    }

    pub fn chat_key(session_id: &str, question: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", session_id, question));
        let hash = hasher.finalize();
        format!("chat:{:x}", hash)
    }

    pub fn wiki_key(session_id: &str, config_hash: &str) -> String {
        format!("wiki:{}:{}", session_id, config_hash)
    }

    pub fn file_key(repo_path: &str, file_path: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", repo_path, file_path));
        let hash = hasher.finalize();
        format!("file:{:x}", hash)
    }
}
