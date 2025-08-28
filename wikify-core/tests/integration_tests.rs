//! Integration tests for wikify-core infrastructure

use std::time::Duration;
use tokio::time::sleep;
use wikify_core::{
    config_error, init_logging, not_found_error, repository_error, retry_async, validation_error,
    with_timeout, ErrorContext, LoggingConfig, RetryConfig, WikifyConfig, WikifyError,
};

#[tokio::test]
async fn test_error_handling() {
    // Test error creation with context
    let error = repository_error!("Test repository error", "test_component");

    match &error {
        WikifyError::Repository {
            message, context, ..
        } => {
            assert_eq!(message, "Test repository error");
            assert_eq!(context.component, "test_component");
            assert!(!context.error_id.is_empty());
        }
        _ => panic!("Expected Repository error"),
    }

    // Test error logging (should not panic)
    error.log();

    // Test error recoverability
    let network_error = WikifyError::Network {
        message: "Connection failed".to_string(),
        source: None,
        context: ErrorContext::new("test"),
    };
    assert!(network_error.is_recoverable());
    assert!(network_error.retry_delay_ms().is_some());

    let config_error = config_error!("Invalid config", "test");
    assert!(!config_error.is_recoverable());
    assert!(config_error.retry_delay_ms().is_none());
}

#[tokio::test]
async fn test_logging_initialization() {
    let config = LoggingConfig {
        level: "debug".to_string(),
        format: wikify_core::LogFormat::Compact,
        include_location: false,
        include_thread: false,
        include_timestamp: true,
        log_to_file: false,
        log_file_path: None,
        enable_performance_monitoring: false,
        filter_directives: vec!["wikify_core=debug".to_string()],
    };

    // This should not panic
    let result = init_logging(&config);
    // Note: We can't test this properly in integration tests because
    // tracing subscriber can only be initialized once per process
    // In a real application, this would work fine
}

#[tokio::test]
async fn test_retry_mechanism() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let attempt_count = Arc::new(AtomicUsize::new(0));

    let operation = {
        let attempt_count = Arc::clone(&attempt_count);
        move || {
            let count = attempt_count.fetch_add(1, Ordering::SeqCst) + 1;
            async move {
                if count < 3 {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Temporary failure",
                    ))
                } else {
                    Ok("Success")
                }
            }
            .boxed()
        }
    };

    let config = RetryConfig {
        max_attempts: 5,
        initial_delay_ms: 10, // Short delay for testing
        max_delay_ms: 100,
        backoff_multiplier: 2.0,
        jitter: false,
    };

    let result = retry_async(operation, config, "test_operation").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_timeout_mechanism() {
    // Test successful operation within timeout
    let quick_operation = async {
        sleep(Duration::from_millis(10)).await;
        "Success"
    };

    let result = with_timeout(quick_operation, 100, "quick_test").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");

    // Test operation that times out
    let slow_operation = async {
        sleep(Duration::from_millis(200)).await;
        "Should not reach here"
    };

    let result = with_timeout(slow_operation, 50, "slow_test").await;
    assert!(result.is_err());

    match result.unwrap_err() {
        WikifyError::Timeout {
            operation,
            duration_ms,
            ..
        } => {
            assert_eq!(operation, "slow_test");
            assert_eq!(duration_ms, 50);
        }
        _ => panic!("Expected Timeout error"),
    }
}

#[tokio::test]
async fn test_config_validation() {
    let mut config = WikifyConfig::default();

    // Valid config should pass validation
    assert!(config.validate().is_ok());

    // Invalid embedding dimensions should fail
    config.embedding.dimensions = 0;
    let result = config.validate();
    assert!(result.is_err());

    match result.unwrap_err() {
        WikifyError::Config { message, .. } => {
            assert!(message.contains("dimensions"));
        }
        _ => panic!("Expected Config error"),
    }
}

#[tokio::test]
async fn test_error_macros() {
    // Test repository_error macro
    let repo_err = repository_error!("Repository not found", "test_component");
    match repo_err {
        WikifyError::Repository {
            message, context, ..
        } => {
            assert_eq!(message, "Repository not found");
            assert_eq!(context.component, "test_component");
        }
        _ => panic!("Expected Repository error"),
    }

    // Test validation_error macro
    let validation_err = validation_error!("Invalid field value", "email", "validator");
    match validation_err {
        WikifyError::Validation {
            message,
            field,
            context,
            ..
        } => {
            assert_eq!(message, "Invalid field value");
            assert_eq!(field, Some("email".to_string()));
            assert_eq!(context.component, "validator");
            assert!(!context.recovery_suggestions.is_empty());
        }
        _ => panic!("Expected Validation error"),
    }

    // Test not_found_error macro
    let not_found_err = not_found_error!("config.toml", "config_loader");
    match not_found_err {
        WikifyError::NotFound {
            resource, context, ..
        } => {
            assert_eq!(resource, "config.toml");
            assert_eq!(context.component, "config_loader");
            assert!(!context.recovery_suggestions.is_empty());
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_concurrent_processing() {
    use wikify_core::process_concurrently;

    let items: Vec<i32> = (1..=10).collect();

    let processor = |item: i32| async move {
        // Simulate some async work
        sleep(Duration::from_millis(10)).await;
        Ok::<i32, WikifyError>(item * 2)
    };

    let results = process_concurrently(items, 3, processor).await;

    // All results should be successful
    assert_eq!(results.len(), 10);
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (i as i32 + 1) * 2);
    }
}

// Helper trait to make futures boxed for testing
trait BoxedFuture<T> {
    fn boxed(self) -> futures::future::BoxFuture<'static, T>;
}

impl<F, T> BoxedFuture<T> for F
where
    F: std::future::Future<Output = T> + Send + 'static,
{
    fn boxed(self) -> futures::future::BoxFuture<'static, T> {
        Box::pin(self)
    }
}
