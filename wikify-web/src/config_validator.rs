//! Configuration validation for Wikify Web Server
//!
//! This module provides comprehensive validation for server configuration
//! to ensure proper setup and prevent runtime errors.

use crate::{WebConfig, WebError, WebResult};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::Path,
};
use tracing::{info, warn};

/// Configuration validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub recommendations: Vec<String>,
}

/// Configuration validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: ErrorSeverity,
}

/// Configuration validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
    pub recommendation: String,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate complete web configuration
    pub fn validate_config(config: &WebConfig) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut recommendations = Vec::new();

        // Validate network configuration
        Self::validate_network_config(config, &mut errors, &mut warnings);

        // Validate database configuration
        Self::validate_database_config(config, &mut errors, &mut warnings);

        // Validate static files configuration
        Self::validate_static_config(config, &mut errors, &mut warnings);

        // Validate development mode settings
        Self::validate_dev_mode_config(config, &mut warnings, &mut recommendations);

        // Generate recommendations
        Self::generate_recommendations(config, &mut recommendations);

        let is_valid = errors.iter().all(|e| !matches!(e.severity, ErrorSeverity::Critical));

        ValidationResult {
            is_valid,
            errors,
            warnings,
            recommendations,
        }
    }

    /// Validate network configuration
    fn validate_network_config(
        config: &WebConfig,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        // Validate host
        if config.host.is_empty() {
            errors.push(ValidationError {
                field: "host".to_string(),
                message: "Host cannot be empty".to_string(),
                severity: ErrorSeverity::Critical,
            });
        } else if config.host.parse::<IpAddr>().is_err() && config.host != "localhost" {
            errors.push(ValidationError {
                field: "host".to_string(),
                message: format!("Invalid host format: {}", config.host),
                severity: ErrorSeverity::High,
            });
        }

        // Validate port
        if config.port == 0 {
            errors.push(ValidationError {
                field: "port".to_string(),
                message: "Port cannot be 0".to_string(),
                severity: ErrorSeverity::Critical,
            });
        } else if config.port < 1024 && !config.dev_mode {
            warnings.push(ValidationWarning {
                field: "port".to_string(),
                message: format!("Using privileged port {} in production", config.port),
                recommendation: "Consider using a port >= 1024 for production deployment".to_string(),
            });
        } else if config.port > 65535 {
            errors.push(ValidationError {
                field: "port".to_string(),
                message: format!("Port {} is out of valid range (1-65535)", config.port),
                severity: ErrorSeverity::High,
            });
        }

        // Test if address is bindable
        let address = format!("{}:{}", config.host, config.port);
        if let Ok(socket_addr) = address.parse::<SocketAddr>() {
            if let Err(_) = std::net::TcpListener::bind(socket_addr) {
                warnings.push(ValidationWarning {
                    field: "address".to_string(),
                    message: format!("Cannot bind to address {}", address),
                    recommendation: "Ensure the address is available and not in use".to_string(),
                });
            }
        }
    }

    /// Validate database configuration
    fn validate_database_config(
        config: &WebConfig,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        if let Some(database_url) = &config.database_url {
            if database_url.is_empty() {
                errors.push(ValidationError {
                    field: "database_url".to_string(),
                    message: "Database URL cannot be empty if provided".to_string(),
                    severity: ErrorSeverity::High,
                });
            } else if database_url == ":memory:" && !config.dev_mode {
                warnings.push(ValidationWarning {
                    field: "database_url".to_string(),
                    message: "Using in-memory database in production".to_string(),
                    recommendation: "Use a persistent database file for production".to_string(),
                });
            } else if !database_url.starts_with("sqlite:") && database_url != ":memory:" {
                // Validate file path for SQLite
                let path = database_url.strip_prefix("sqlite:").unwrap_or(database_url);
                if let Some(parent) = Path::new(path).parent() {
                    if !parent.exists() {
                        errors.push(ValidationError {
                            field: "database_url".to_string(),
                            message: format!("Database directory does not exist: {}", parent.display()),
                            severity: ErrorSeverity::High,
                        });
                    }
                }
            }
        } else if !config.dev_mode {
            warnings.push(ValidationWarning {
                field: "database_url".to_string(),
                message: "No database configured for production".to_string(),
                recommendation: "Configure a database for session persistence".to_string(),
            });
        }
    }

    /// Validate static files configuration
    fn validate_static_config(
        config: &WebConfig,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        if let Some(static_dir) = &config.static_dir {
            if static_dir.is_empty() {
                errors.push(ValidationError {
                    field: "static_dir".to_string(),
                    message: "Static directory cannot be empty if provided".to_string(),
                    severity: ErrorSeverity::Medium,
                });
            } else {
                let path = Path::new(static_dir);
                if !path.exists() {
                    errors.push(ValidationError {
                        field: "static_dir".to_string(),
                        message: format!("Static directory does not exist: {}", static_dir),
                        severity: ErrorSeverity::Medium,
                    });
                } else if !path.is_dir() {
                    errors.push(ValidationError {
                        field: "static_dir".to_string(),
                        message: format!("Static path is not a directory: {}", static_dir),
                        severity: ErrorSeverity::Medium,
                    });
                } else {
                    // Check for common static files
                    let index_html = path.join("index.html");
                    if !index_html.exists() {
                        warnings.push(ValidationWarning {
                            field: "static_dir".to_string(),
                            message: "No index.html found in static directory".to_string(),
                            recommendation: "Ensure index.html exists for proper SPA fallback".to_string(),
                        });
                    }
                }
            }
        }
    }

    /// Validate development mode configuration
    fn validate_dev_mode_config(
        config: &WebConfig,
        warnings: &mut Vec<ValidationWarning>,
        recommendations: &mut Vec<String>,
    ) {
        if config.dev_mode {
            warnings.push(ValidationWarning {
                field: "dev_mode".to_string(),
                message: "Development mode is enabled".to_string(),
                recommendation: "Disable development mode for production deployment".to_string(),
            });

            recommendations.push("Development mode enables additional logging and debugging features".to_string());
            recommendations.push("Ensure dev_mode is set to false for production".to_string());
        }
    }

    /// Generate general recommendations
    fn generate_recommendations(config: &WebConfig, recommendations: &mut Vec<String>) {
        // Security recommendations
        if config.host == "0.0.0.0" {
            recommendations.push("Consider using a reverse proxy (nginx, Apache) when binding to 0.0.0.0".to_string());
        }

        // Performance recommendations
        if config.database_url.is_none() {
            recommendations.push("Configure a database for better session management and persistence".to_string());
        }

        // Monitoring recommendations
        recommendations.push("Consider setting up monitoring and logging for production deployment".to_string());
        recommendations.push("Configure proper backup strategies for your database".to_string());
    }

    /// Validate environment variables
    pub fn validate_environment() -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut recommendations = Vec::new();

        // Check for common environment variables
        let env_vars = [
            ("WIKIFY_HOST", false),
            ("WIKIFY_PORT", false),
            ("WIKIFY_DATABASE_URL", false),
            ("WIKIFY_STATIC_DIR", false),
            ("WIKIFY_DEV_MODE", false),
            ("RUST_LOG", false),
        ];

        for (var_name, required) in env_vars {
            match std::env::var(var_name) {
                Ok(value) => {
                    if value.is_empty() {
                        warnings.push(ValidationWarning {
                            field: var_name.to_string(),
                            message: format!("Environment variable {} is empty", var_name),
                            recommendation: format!("Set a proper value for {}", var_name),
                        });
                    }
                }
                Err(_) => {
                    if required {
                        errors.push(ValidationError {
                            field: var_name.to_string(),
                            message: format!("Required environment variable {} is not set", var_name),
                            severity: ErrorSeverity::High,
                        });
                    }
                }
            }
        }

        // Check RUST_LOG specifically
        if let Ok(rust_log) = std::env::var("RUST_LOG") {
            let valid_levels = ["error", "warn", "info", "debug", "trace"];
            if !valid_levels.iter().any(|&level| rust_log.contains(level)) {
                warnings.push(ValidationWarning {
                    field: "RUST_LOG".to_string(),
                    message: "RUST_LOG may not contain valid log levels".to_string(),
                    recommendation: "Use format like 'info' or 'wikify_web=debug'".to_string(),
                });
            }
        } else {
            recommendations.push("Set RUST_LOG environment variable for proper logging".to_string());
        }

        let is_valid = errors.iter().all(|e| !matches!(e.severity, ErrorSeverity::Critical));

        ValidationResult {
            is_valid,
            errors,
            warnings,
            recommendations,
        }
    }

    /// Print validation results in a user-friendly format
    pub fn print_validation_results(result: &ValidationResult) {
        if result.is_valid {
            info!("✅ Configuration validation passed");
        } else {
            warn!("❌ Configuration validation failed");
        }

        // Print errors
        for error in &result.errors {
            let icon = match error.severity {
                ErrorSeverity::Critical => "🚨",
                ErrorSeverity::High => "❌",
                ErrorSeverity::Medium => "⚠️",
                ErrorSeverity::Low => "ℹ️",
            };
            eprintln!("{} {}: {}", icon, error.field, error.message);
        }

        // Print warnings
        for warning in &result.warnings {
            println!("⚠️  {}: {} (Recommendation: {})", warning.field, warning.message, warning.recommendation);
        }

        // Print recommendations
        if !result.recommendations.is_empty() {
            println!("\n💡 Recommendations:");
            for recommendation in &result.recommendations {
                println!("   • {}", recommendation);
            }
        }
    }
}

/// Validate configuration and return result
pub fn validate_config(config: &WebConfig) -> WebResult<ValidationResult> {
    let result = ConfigValidator::validate_config(config);
    
    if !result.is_valid {
        return Err(WebError::Configuration(
            "Configuration validation failed".to_string()
        ));
    }
    
    Ok(result)
}

/// Validate environment and print results
pub fn validate_and_print_environment() {
    let result = ConfigValidator::validate_environment();
    ConfigValidator::print_validation_results(&result);
}
