//! Async utilities and patterns
//!
//! Provides common async patterns, retry logic, and concurrency control

use crate::error::{ErrorContext, WikifyError, WikifyResult};
use futures::future::{BoxFuture, FutureExt};
use std::sync::Arc;
use tokio::time::{sleep, timeout, Duration};
use tracing::{debug, error, warn};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Initial delay between retries in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    /// Backoff multiplier (exponential backoff)
    pub backoff_multiplier: f64,
    /// Whether to add jitter to delays
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Retry an async operation with exponential backoff
pub async fn retry_async<F, T, E>(
    operation: F,
    config: RetryConfig,
    operation_name: &str,
) -> Result<T, E>
where
    F: Fn() -> BoxFuture<'static, Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay_ms;

    loop {
        attempt += 1;

        debug!(
            operation = operation_name,
            attempt = attempt,
            max_attempts = config.max_attempts,
            "Attempting operation"
        );

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!(
                        operation = operation_name,
                        attempt = attempt,
                        "Operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(error) => {
                if attempt >= config.max_attempts {
                    error!(
                        operation = operation_name,
                        attempt = attempt,
                        error = %error,
                        "Operation failed after all retry attempts"
                    );
                    return Err(error);
                }

                warn!(
                    operation = operation_name,
                    attempt = attempt,
                    error = %error,
                    delay_ms = delay,
                    "Operation failed, retrying"
                );

                // Calculate next delay with exponential backoff
                let actual_delay = if config.jitter {
                    let jitter_factor = 0.1;
                    let jitter = (fastrand::f64() - 0.5) * 2.0 * jitter_factor;
                    ((delay as f64) * (1.0 + jitter)) as u64
                } else {
                    delay
                };

                sleep(Duration::from_millis(actual_delay)).await;

                delay = ((delay as f64) * config.backoff_multiplier) as u64;
                delay = delay.min(config.max_delay_ms);
            }
        }
    }
}

/// Timeout wrapper for async operations
pub async fn with_timeout<F, T>(future: F, timeout_ms: u64, operation_name: &str) -> WikifyResult<T>
where
    F: std::future::Future<Output = T>,
{
    match timeout(Duration::from_millis(timeout_ms), future).await {
        Ok(result) => Ok(result),
        Err(_) => Err(Box::new(WikifyError::Timeout {
            operation: operation_name.to_string(),
            duration_ms: timeout_ms,
            context: ErrorContext::new("async_utils")
                .with_operation("timeout")
                .with_metadata("timeout_ms", &timeout_ms.to_string())
                .with_suggestion("Increase timeout duration")
                .with_suggestion("Check network connectivity")
                .with_suggestion("Verify service availability"),
        })),
    }
}

/// Concurrent processing with controlled parallelism
pub async fn process_concurrently<T, R, F, Fut>(
    items: Vec<T>,
    max_concurrent: usize,
    processor: F,
) -> Vec<WikifyResult<R>>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = WikifyResult<R>> + Send + 'static,
{
    use futures::stream::{self, StreamExt};

    stream::iter(items)
        .map(|item| {
            let processor = processor.clone();
            tokio::spawn(async move { processor(item).await })
        })
        .buffer_unordered(max_concurrent)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(|join_result| match join_result {
            Ok(result) => result,
            Err(join_error) => Err(Box::new(WikifyError::Internal {
                message: format!("Task join error: {}", join_error),
                source: Some(Box::new(join_error)),
                context: ErrorContext::new("async_utils")
                    .with_operation("process_concurrently")
                    .with_suggestion("Check for panics in concurrent tasks"),
            })),
        })
        .collect()
}

/// Rate limiter for API calls
#[derive(Debug)]
pub struct RateLimiter {
    permits: Arc<tokio::sync::Semaphore>,
    min_interval: Duration,
    last_request: Arc<tokio::sync::Mutex<Option<tokio::time::Instant>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_concurrent: usize, min_interval_ms: u64) -> Self {
        Self {
            permits: Arc::new(tokio::sync::Semaphore::new(max_concurrent)),
            min_interval: Duration::from_millis(min_interval_ms),
            last_request: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Acquire a permit and enforce rate limiting
    pub async fn acquire(&self) -> WikifyResult<RateLimitGuard> {
        // Acquire semaphore permit
        let permit =
            self.permits
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| WikifyError::Internal {
                    message: format!("Failed to acquire rate limit permit: {}", e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("rate_limiter").with_operation("acquire"),
                })?;

        // Enforce minimum interval
        let mut last_request = self.last_request.lock().await;
        if let Some(last) = *last_request {
            let elapsed = last.elapsed();
            if elapsed < self.min_interval {
                let sleep_duration = self.min_interval - elapsed;
                debug!(
                    sleep_ms = sleep_duration.as_millis(),
                    "Rate limiting: sleeping to enforce minimum interval"
                );
                sleep(sleep_duration).await;
            }
        }
        *last_request = Some(tokio::time::Instant::now());

        Ok(RateLimitGuard { _permit: permit })
    }
}

/// RAII guard for rate limiter permits
pub struct RateLimitGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

/// Batch processor for efficient bulk operations
pub struct BatchProcessor<T> {
    batch_size: usize,
    flush_interval: Duration,
    processor: Arc<dyn Fn(Vec<T>) -> BoxFuture<'static, WikifyResult<()>> + Send + Sync>,
    buffer: Arc<tokio::sync::Mutex<Vec<T>>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl<T: Send + 'static> BatchProcessor<T> {
    /// Create a new batch processor
    pub fn new<F, Fut>(batch_size: usize, flush_interval_ms: u64, processor: F) -> Self
    where
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = WikifyResult<()>> + Send + 'static,
    {
        let processor = Arc::new(move |items: Vec<T>| processor(items).boxed());

        Self {
            batch_size,
            flush_interval: Duration::from_millis(flush_interval_ms),
            processor,
            buffer: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            shutdown_tx: None,
        }
    }

    /// Start the batch processor background task
    pub fn start(&mut self) -> WikifyResult<()> {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let buffer = Arc::clone(&self.buffer);
        let processor = Arc::clone(&self.processor);
        let _batch_size = self.batch_size;
        let flush_interval = self.flush_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(flush_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut buffer = buffer.lock().await;
                        if !buffer.is_empty() {
                            let items = std::mem::take(&mut *buffer);
                            drop(buffer);

                            if let Err(e) = processor(items).await {
                                error!(error = %e, "Batch processing failed");
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        // Final flush before shutdown
                        let mut buffer = buffer.lock().await;
                        if !buffer.is_empty() {
                            let items = std::mem::take(&mut *buffer);
                            drop(buffer);

                            if let Err(e) = processor(items).await {
                                error!(error = %e, "Final batch processing failed");
                            }
                        }
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Add an item to the batch
    pub async fn add(&self, item: T) -> WikifyResult<()> {
        let mut buffer = self.buffer.lock().await;
        buffer.push(item);

        if buffer.len() >= self.batch_size {
            let items = std::mem::take(&mut *buffer);
            drop(buffer);

            (self.processor)(items).await?;
        }

        Ok(())
    }

    /// Flush all pending items
    pub async fn flush(&self) -> WikifyResult<()> {
        let mut buffer = self.buffer.lock().await;
        if !buffer.is_empty() {
            let items = std::mem::take(&mut *buffer);
            drop(buffer);

            (self.processor)(items).await?;
        }

        Ok(())
    }
}

impl<T> Drop for BatchProcessor<T> {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }
}
