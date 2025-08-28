//! Security middleware and utilities for Wikify Web Server
//!
//! This module provides security features including rate limiting, input validation,
//! and request sanitization.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tracing::{debug, warn};

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub cleanup_interval: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// Rate limiter entry for tracking requests
#[derive(Debug, Clone)]
struct RateLimitEntry {
    requests: Vec<Instant>,
    last_request: Instant,
}

impl RateLimitEntry {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            last_request: Instant::now(),
        }
    }

    fn add_request(&mut self, now: Instant) {
        self.requests.push(now);
        self.last_request = now;
    }

    fn cleanup_old_requests(&mut self, window: Duration) {
        let cutoff = Instant::now() - window;
        self.requests.retain(|&request_time| request_time > cutoff);
    }

    fn request_count(&self) -> usize {
        self.requests.len()
    }
}

/// Rate limiter implementation
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    entries: Arc<Mutex<HashMap<IpAddr, RateLimitEntry>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let limiter = Self {
            config,
            entries: Arc::new(Mutex::new(HashMap::new())),
        };

        // Start cleanup task
        limiter.start_cleanup_task();
        limiter
    }

    pub fn check_rate_limit(&self, ip: IpAddr) -> bool {
        let mut entries = self.entries.lock().unwrap();
        let now = Instant::now();
        let window = Duration::from_secs(60);

        let entry = entries.entry(ip).or_insert_with(RateLimitEntry::new);
        entry.cleanup_old_requests(window);

        if entry.request_count() >= self.config.requests_per_minute as usize {
            warn!("Rate limit exceeded for IP: {}", ip);
            false
        } else {
            entry.add_request(now);
            debug!("Request allowed for IP: {} ({}/{})", ip, entry.request_count(), self.config.requests_per_minute);
            true
        }
    }

    fn start_cleanup_task(&self) {
        let entries = Arc::clone(&self.entries);
        let cleanup_interval = self.config.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);

            loop {
                interval.tick().await;

                let mut entries = entries.lock().unwrap();
                let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes

                entries.retain(|_, entry| entry.last_request > cutoff);
                debug!("Rate limiter cleanup completed");
            }
        });
    }
}

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityHeaders {
    pub content_security_policy: Option<String>,
    pub x_frame_options: Option<String>,
    pub x_content_type_options: bool,
    pub x_xss_protection: bool,
    pub strict_transport_security: Option<String>,
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self {
            content_security_policy: Some(
                "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'"
                    .to_string(),
            ),
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: true,
            x_xss_protection: true,
            strict_transport_security: Some("max-age=31536000; includeSubDomains".to_string()),
        }
    }
}

/// Input validation utilities
pub mod validation {
    use regex::Regex;
    use std::sync::OnceLock;

    static URL_REGEX: OnceLock<Regex> = OnceLock::new();
    static SESSION_ID_REGEX: OnceLock<Regex> = OnceLock::new();

    pub fn is_valid_url(url: &str) -> bool {
        let regex = URL_REGEX.get_or_init(|| {
            Regex::new(r"^https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(/.*)?$").unwrap()
        });
        regex.is_match(url) && url.len() <= 2048
    }

    pub fn is_valid_session_id(session_id: &str) -> bool {
        let regex = SESSION_ID_REGEX.get_or_init(|| {
            Regex::new(r"^[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}$").unwrap()
        });
        regex.is_match(session_id)
    }

    pub fn sanitize_string(input: &str, max_length: usize) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?-_()[]{}".contains(*c))
            .take(max_length)
            .collect()
    }

    pub fn is_safe_path(path: &str) -> bool {
        !path.contains("..") && !path.starts_with('/') && path.len() <= 1024
    }
}

/// Security middleware state
#[derive(Debug, Clone)]
pub struct SecurityState {
    pub rate_limiter: Arc<RateLimiter>,
    pub security_headers: SecurityHeaders,
}

impl SecurityState {
    pub fn new(rate_limit_config: RateLimitConfig) -> Self {
        Self {
            rate_limiter: Arc::new(RateLimiter::new(rate_limit_config)),
            security_headers: SecurityHeaders::default(),
        }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(security_state): State<SecurityState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract client IP
    let client_ip = extract_client_ip(&request);

    // Check rate limit
    if !security_state.rate_limiter.check_rate_limit(client_ip) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Continue to next middleware
    Ok(next.run(request).await)
}

/// Security headers middleware
pub async fn security_headers_middleware(
    State(security_state): State<SecurityState>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Add security headers
    if let Some(csp) = &security_state.security_headers.content_security_policy {
        headers.insert("Content-Security-Policy", csp.parse().unwrap());
    }

    if let Some(frame_options) = &security_state.security_headers.x_frame_options {
        headers.insert("X-Frame-Options", frame_options.parse().unwrap());
    }

    if security_state.security_headers.x_content_type_options {
        headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    }

    if security_state.security_headers.x_xss_protection {
        headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    }

    if let Some(hsts) = &security_state.security_headers.strict_transport_security {
        headers.insert("Strict-Transport-Security", hsts.parse().unwrap());
    }

    response
}

/// Extract client IP from request
fn extract_client_ip(request: &Request) -> IpAddr {
    // Check X-Forwarded-For header first (for reverse proxies)
    if let Some(forwarded) = request.headers().get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse() {
                    return ip;
                }
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = request.headers().get("X-Real-IP") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse() {
                return ip;
            }
        }
    }

    // Fallback to connection info (this would need to be passed from the connection)
    // For now, return localhost as default
    "127.0.0.1".parse().unwrap()
}

/// Request validation utilities
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

pub fn validate_repository_request(
    repository: &str,
    repo_type: &Option<String>,
) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Validate repository URL/path
    if repository.is_empty() {
        errors.push(ValidationError {
            field: "repository".to_string(),
            message: "Repository cannot be empty".to_string(),
        });
    } else if repository.starts_with("http") && !validation::is_valid_url(repository) {
        errors.push(ValidationError {
            field: "repository".to_string(),
            message: "Invalid repository URL format".to_string(),
        });
    } else if !repository.starts_with("http") && !validation::is_safe_path(repository) {
        errors.push(ValidationError {
            field: "repository".to_string(),
            message: "Invalid repository path".to_string(),
        });
    }

    // Validate repo_type if provided
    if let Some(repo_type) = repo_type {
        let valid_types = ["github", "gitlab", "local", "auto"];
        if !valid_types.contains(&repo_type.as_str()) {
            errors.push(ValidationError {
                field: "repo_type".to_string(),
                message: format!("Invalid repo_type. Must be one of: {}", valid_types.join(", ")),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
