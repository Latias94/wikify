//! Unified logging system
//!
//! Provides structured logging with performance monitoring and configurable output

use serde::{Deserialize, Serialize};
use std::io;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Output format (json, pretty, compact)
    pub format: LogFormat,
    /// Whether to include file and line information
    pub include_location: bool,
    /// Whether to include thread information
    pub include_thread: bool,
    /// Whether to include timestamps
    pub include_timestamp: bool,
    /// Whether to log to file
    pub log_to_file: bool,
    /// Log file path (if log_to_file is true)
    pub log_file_path: Option<String>,
    /// Whether to enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Custom filter directives
    pub filter_directives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Pretty,
            include_location: true,
            include_thread: false,
            include_timestamp: true,
            log_to_file: false,
            log_file_path: None,
            enable_performance_monitoring: true,
            filter_directives: vec![
                "wikify=debug".to_string(),
                "wikify_core=debug".to_string(),
                "wikify_repo=debug".to_string(),
                "wikify_indexing=debug".to_string(),
                "wikify_rag=debug".to_string(),
            ],
        }
    }
}

/// Initialize the logging system
pub fn init_logging(
    config: &LoggingConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    // Add custom filter directives
    for directive in &config.filter_directives {
        filter = filter.add_directive(directive.parse()?);
    }

    let registry = tracing_subscriber::registry().with(filter);

    match config.format {
        LogFormat::Json => {
            let fmt_layer = fmt::layer()
                .with_span_events(if config.enable_performance_monitoring {
                    FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_file(config.include_location)
                .with_line_number(config.include_location)
                .with_thread_ids(config.include_thread)
                .with_thread_names(config.include_thread);

            if config.log_to_file {
                if let Some(log_path) = &config.log_file_path {
                    let file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(log_path)?;

                    registry.with(fmt_layer.with_writer(file)).init();
                } else {
                    return Err("log_file_path must be specified when log_to_file is true".into());
                }
            } else {
                registry.with(fmt_layer.with_writer(io::stdout)).init();
            }
        }
        LogFormat::Pretty => {
            let fmt_layer = fmt::layer()
                .pretty()
                .with_span_events(if config.enable_performance_monitoring {
                    FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_file(config.include_location)
                .with_line_number(config.include_location)
                .with_thread_ids(config.include_thread)
                .with_thread_names(config.include_thread);

            if config.log_to_file {
                if let Some(log_path) = &config.log_file_path {
                    let file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(log_path)?;

                    registry.with(fmt_layer.with_writer(file)).init();
                } else {
                    return Err("log_file_path must be specified when log_to_file is true".into());
                }
            } else {
                registry.with(fmt_layer.with_writer(io::stdout)).init();
            }
        }
        LogFormat::Compact => {
            let fmt_layer = fmt::layer()
                .compact()
                .with_span_events(if config.enable_performance_monitoring {
                    FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_file(config.include_location)
                .with_line_number(config.include_location)
                .with_thread_ids(config.include_thread)
                .with_thread_names(config.include_thread);

            if config.log_to_file {
                if let Some(log_path) = &config.log_file_path {
                    let file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(log_path)?;

                    registry.with(fmt_layer.with_writer(file)).init();
                } else {
                    return Err("log_file_path must be specified when log_to_file is true".into());
                }
            } else {
                registry.with(fmt_layer.with_writer(io::stdout)).init();
            }
        }
    }

    Ok(())
}

/// Performance monitoring utilities
pub mod performance {
    use std::time::Instant;
    use tracing::{info_span, Instrument};

    /// Measure and log execution time of an async operation
    pub async fn measure_async<F, T>(operation_name: &str, future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let span = info_span!("performance", operation = operation_name);
        let start = Instant::now();

        let result = future.instrument(span.clone()).await;

        let duration = start.elapsed();
        tracing::info!(
            target: "performance",
            operation = operation_name,
            duration_ms = duration.as_millis(),
            "Operation completed"
        );

        result
    }

    /// Measure and log execution time of a synchronous operation
    pub fn measure_sync<F, T>(operation_name: &str, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let _span = info_span!("performance", operation = operation_name).entered();
        let start = Instant::now();

        let result = f();

        let duration = start.elapsed();
        tracing::info!(
            target: "performance",
            operation = operation_name,
            duration_ms = duration.as_millis(),
            "Operation completed"
        );

        result
    }
}

/// Logging macros for common patterns
#[macro_export]
macro_rules! log_operation_start {
    ($operation:expr) => {
        tracing::info!(
            operation = $operation,
            "Starting operation"
        );
    };
    ($operation:expr, $($field:tt)*) => {
        tracing::info!(
            operation = $operation,
            $($field)*,
            "Starting operation"
        );
    };
}

#[macro_export]
macro_rules! log_operation_success {
    ($operation:expr) => {
        tracing::info!(
            operation = $operation,
            "Operation completed successfully"
        );
    };
    ($operation:expr, $($field:tt)*) => {
        tracing::info!(
            operation = $operation,
            $($field)*,
            "Operation completed successfully"
        );
    };
}

#[macro_export]
macro_rules! log_operation_error {
    ($operation:expr, $error:expr) => {
        tracing::error!(
            operation = $operation,
            error = %$error,
            "Operation failed"
        );
    };
    ($operation:expr, $error:expr, $($field:tt)*) => {
        tracing::error!(
            operation = $operation,
            error = %$error,
            $($field)*,
            "Operation failed"
        );
    };
}
