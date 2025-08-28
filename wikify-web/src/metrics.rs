//! Metrics collection and monitoring for Wikify Web Server
//!
//! This module provides comprehensive metrics collection for monitoring
//! server performance, usage patterns, and system health.

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::interval;
use tracing::{debug, info};

/// Application metrics collector
#[derive(Debug)]
pub struct MetricsCollector {
    /// HTTP request metrics
    pub http_requests_total: AtomicU64,
    pub http_requests_by_status: Arc<RwLock<HashMap<u16, u64>>>,
    pub http_request_duration: Arc<RwLock<Vec<Duration>>>,
    
    /// WebSocket metrics
    pub websocket_connections_total: AtomicU64,
    pub websocket_connections_active: AtomicU64,
    pub websocket_messages_sent: AtomicU64,
    pub websocket_messages_received: AtomicU64,
    
    /// Repository metrics
    pub repositories_initialized: AtomicU64,
    pub repositories_indexed: AtomicU64,
    pub indexing_duration: Arc<RwLock<Vec<Duration>>>,
    
    /// Chat metrics
    pub chat_queries_total: AtomicU64,
    pub chat_response_duration: Arc<RwLock<Vec<Duration>>>,
    pub chat_tokens_processed: AtomicU64,
    
    /// Wiki generation metrics
    pub wiki_generations_total: AtomicU64,
    pub wiki_generation_duration: Arc<RwLock<Vec<Duration>>>,
    pub wiki_pages_generated: AtomicU64,
    
    /// Database metrics
    pub database_queries_total: AtomicU64,
    pub database_query_duration: Arc<RwLock<Vec<Duration>>>,
    pub database_connections_active: AtomicU64,
    
    /// System metrics
    pub memory_usage_bytes: AtomicU64,
    pub cpu_usage_percent: Arc<RwLock<f64>>,
    pub uptime_seconds: Arc<RwLock<u64>>,
    
    /// Error metrics
    pub errors_total: AtomicU64,
    pub errors_by_type: Arc<RwLock<HashMap<String, u64>>>,
    
    /// Start time for uptime calculation
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let collector = Self {
            http_requests_total: AtomicU64::new(0),
            http_requests_by_status: Arc::new(RwLock::new(HashMap::new())),
            http_request_duration: Arc::new(RwLock::new(Vec::new())),
            
            websocket_connections_total: AtomicU64::new(0),
            websocket_connections_active: AtomicU64::new(0),
            websocket_messages_sent: AtomicU64::new(0),
            websocket_messages_received: AtomicU64::new(0),
            
            repositories_initialized: AtomicU64::new(0),
            repositories_indexed: AtomicU64::new(0),
            indexing_duration: Arc::new(RwLock::new(Vec::new())),
            
            chat_queries_total: AtomicU64::new(0),
            chat_response_duration: Arc::new(RwLock::new(Vec::new())),
            chat_tokens_processed: AtomicU64::new(0),
            
            wiki_generations_total: AtomicU64::new(0),
            wiki_generation_duration: Arc::new(RwLock::new(Vec::new())),
            wiki_pages_generated: AtomicU64::new(0),
            
            database_queries_total: AtomicU64::new(0),
            database_query_duration: Arc::new(RwLock::new(Vec::new())),
            database_connections_active: AtomicU64::new(0),
            
            memory_usage_bytes: AtomicU64::new(0),
            cpu_usage_percent: Arc::new(RwLock::new(0.0)),
            uptime_seconds: Arc::new(RwLock::new(0)),
            
            errors_total: AtomicU64::new(0),
            errors_by_type: Arc::new(RwLock::new(HashMap::new())),
            
            start_time: Instant::now(),
        };

        // Start background metrics collection
        collector.start_system_metrics_collection();
        collector
    }

    /// Record HTTP request
    pub fn record_http_request(&self, status_code: u16, duration: Duration) {
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);
        
        {
            let mut status_counts = self.http_requests_by_status.write().unwrap();
            *status_counts.entry(status_code).or_insert(0) += 1;
        }
        
        {
            let mut durations = self.http_request_duration.write().unwrap();
            durations.push(duration);
            // Keep only last 1000 measurements
            if durations.len() > 1000 {
                durations.remove(0);
            }
        }
        
        debug!("HTTP request recorded: {} - {:?}", status_code, duration);
    }

    /// Record WebSocket connection
    pub fn record_websocket_connection(&self) {
        self.websocket_connections_total.fetch_add(1, Ordering::Relaxed);
        self.websocket_connections_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record WebSocket disconnection
    pub fn record_websocket_disconnection(&self) {
        self.websocket_connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record WebSocket message
    pub fn record_websocket_message(&self, sent: bool) {
        if sent {
            self.websocket_messages_sent.fetch_add(1, Ordering::Relaxed);
        } else {
            self.websocket_messages_received.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record repository initialization
    pub fn record_repository_initialization(&self) {
        self.repositories_initialized.fetch_add(1, Ordering::Relaxed);
    }

    /// Record repository indexing
    pub fn record_repository_indexing(&self, duration: Duration) {
        self.repositories_indexed.fetch_add(1, Ordering::Relaxed);
        
        let mut durations = self.indexing_duration.write().unwrap();
        durations.push(duration);
        if durations.len() > 100 {
            durations.remove(0);
        }
    }

    /// Record chat query
    pub fn record_chat_query(&self, duration: Duration, tokens_processed: u64) {
        self.chat_queries_total.fetch_add(1, Ordering::Relaxed);
        self.chat_tokens_processed.fetch_add(tokens_processed, Ordering::Relaxed);
        
        let mut durations = self.chat_response_duration.write().unwrap();
        durations.push(duration);
        if durations.len() > 1000 {
            durations.remove(0);
        }
    }

    /// Record wiki generation
    pub fn record_wiki_generation(&self, duration: Duration, pages_generated: u64) {
        self.wiki_generations_total.fetch_add(1, Ordering::Relaxed);
        self.wiki_pages_generated.fetch_add(pages_generated, Ordering::Relaxed);
        
        let mut durations = self.wiki_generation_duration.write().unwrap();
        durations.push(duration);
        if durations.len() > 100 {
            durations.remove(0);
        }
    }

    /// Record database query
    pub fn record_database_query(&self, duration: Duration) {
        self.database_queries_total.fetch_add(1, Ordering::Relaxed);
        
        let mut durations = self.database_query_duration.write().unwrap();
        durations.push(duration);
        if durations.len() > 1000 {
            durations.remove(0);
        }
    }

    /// Record error
    pub fn record_error(&self, error_type: &str) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
        
        let mut error_counts = self.errors_by_type.write().unwrap();
        *error_counts.entry(error_type.to_string()).or_insert(0) += 1;
    }

    /// Get comprehensive metrics snapshot
    pub fn get_metrics(&self) -> MetricsSnapshot {
        let uptime = self.start_time.elapsed().as_secs();
        
        MetricsSnapshot {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            uptime_seconds: uptime,
            
            http: HttpMetrics {
                requests_total: self.http_requests_total.load(Ordering::Relaxed),
                requests_by_status: self.http_requests_by_status.read().unwrap().clone(),
                average_response_time_ms: self.calculate_average_duration(&self.http_request_duration),
            },
            
            websocket: WebSocketMetrics {
                connections_total: self.websocket_connections_total.load(Ordering::Relaxed),
                connections_active: self.websocket_connections_active.load(Ordering::Relaxed),
                messages_sent: self.websocket_messages_sent.load(Ordering::Relaxed),
                messages_received: self.websocket_messages_received.load(Ordering::Relaxed),
            },
            
            repository: RepositoryMetrics {
                initialized_total: self.repositories_initialized.load(Ordering::Relaxed),
                indexed_total: self.repositories_indexed.load(Ordering::Relaxed),
                average_indexing_time_ms: self.calculate_average_duration(&self.indexing_duration),
            },
            
            chat: ChatMetrics {
                queries_total: self.chat_queries_total.load(Ordering::Relaxed),
                tokens_processed: self.chat_tokens_processed.load(Ordering::Relaxed),
                average_response_time_ms: self.calculate_average_duration(&self.chat_response_duration),
            },
            
            wiki: WikiMetrics {
                generations_total: self.wiki_generations_total.load(Ordering::Relaxed),
                pages_generated: self.wiki_pages_generated.load(Ordering::Relaxed),
                average_generation_time_ms: self.calculate_average_duration(&self.wiki_generation_duration),
            },
            
            database: DatabaseMetrics {
                queries_total: self.database_queries_total.load(Ordering::Relaxed),
                connections_active: self.database_connections_active.load(Ordering::Relaxed),
                average_query_time_ms: self.calculate_average_duration(&self.database_query_duration),
            },
            
            system: SystemMetrics {
                memory_usage_bytes: self.memory_usage_bytes.load(Ordering::Relaxed),
                cpu_usage_percent: *self.cpu_usage_percent.read().unwrap(),
            },
            
            errors: ErrorMetrics {
                total: self.errors_total.load(Ordering::Relaxed),
                by_type: self.errors_by_type.read().unwrap().clone(),
            },
        }
    }

    fn calculate_average_duration(&self, durations: &Arc<RwLock<Vec<Duration>>>) -> f64 {
        let durations = durations.read().unwrap();
        if durations.is_empty() {
            0.0
        } else {
            let total_ms: f64 = durations.iter().map(|d| d.as_millis() as f64).sum();
            total_ms / durations.len() as f64
        }
    }

    fn start_system_metrics_collection(&self) {
        let memory_usage = Arc::clone(&self.memory_usage_bytes);
        let cpu_usage = Arc::clone(&self.cpu_usage_percent);
        let uptime = Arc::clone(&self.uptime_seconds);
        let start_time = self.start_time;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Update uptime
                {
                    let mut uptime_guard = uptime.write().unwrap();
                    *uptime_guard = start_time.elapsed().as_secs();
                }

                // Update memory usage (simplified - in production you'd use a proper system metrics library)
                // This is a placeholder implementation
                memory_usage.store(get_memory_usage(), Ordering::Relaxed);

                // Update CPU usage (placeholder)
                {
                    let mut cpu_guard = cpu_usage.write().unwrap();
                    *cpu_guard = get_cpu_usage();
                }

                debug!("System metrics updated");
            }
        });
    }
}

// Placeholder functions for system metrics
// In production, you'd use libraries like `sysinfo` or `procfs`
fn get_memory_usage() -> u64 {
    // Placeholder implementation
    1024 * 1024 * 100 // 100MB
}

fn get_cpu_usage() -> f64 {
    // Placeholder implementation
    15.5 // 15.5%
}

/// Complete metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub http: HttpMetrics,
    pub websocket: WebSocketMetrics,
    pub repository: RepositoryMetrics,
    pub chat: ChatMetrics,
    pub wiki: WikiMetrics,
    pub database: DatabaseMetrics,
    pub system: SystemMetrics,
    pub errors: ErrorMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpMetrics {
    pub requests_total: u64,
    pub requests_by_status: HashMap<u16, u64>,
    pub average_response_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMetrics {
    pub connections_total: u64,
    pub connections_active: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryMetrics {
    pub initialized_total: u64,
    pub indexed_total: u64,
    pub average_indexing_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMetrics {
    pub queries_total: u64,
    pub tokens_processed: u64,
    pub average_response_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiMetrics {
    pub generations_total: u64,
    pub pages_generated: u64,
    pub average_generation_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    pub queries_total: u64,
    pub connections_active: u64,
    pub average_query_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub total: u64,
    pub by_type: HashMap<String, u64>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
