//! Wikify Web Server
//!
//! Main web server implementation using Axum.

use crate::{create_app, AppState, WebConfig, WebError, WebResult};
use axum::serve;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Main Wikify web server
pub struct WikifyServer {
    config: WebConfig,
    state: AppState,
}

impl WikifyServer {
    /// Create a new Wikify server
    pub async fn new(config: WebConfig) -> WebResult<Self> {
        let state = AppState::new(config.clone()).await?;

        Ok(Self { config, state })
    }

    /// Start the web server
    pub async fn start(self) -> WebResult<()> {
        let address = self.config.address();

        info!("🚀 Starting Wikify Web Server");
        info!("📍 Server address: http://{}", address);
        info!("🔧 Development mode: {}", self.config.dev_mode);

        // Create the application
        let app = create_app(self.state.clone());

        // Create TCP listener
        let listener = TcpListener::bind(&address)
            .await
            .map_err(WebError::Server)?;

        info!("✅ Server listening on http://{}", address);

        // Start cleanup task for old sessions
        let cleanup_state = self.state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour
            loop {
                interval.tick().await;
                cleanup_state.cleanup_old_data().await;
            }
        });

        // Start the server
        if let Err(e) = serve(listener, app).await {
            error!("❌ Server error: {}", e);
            return Err(WebError::Server(e));
        }

        Ok(())
    }

    /// Get server configuration
    pub fn config(&self) -> &WebConfig {
        &self.config
    }

    /// Get application state
    pub fn state(&self) -> &AppState {
        &self.state
    }
}

/// Builder for WikifyServer
pub struct WikifyServerBuilder {
    config: WebConfig,
}

impl WikifyServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        Self {
            config: WebConfig::default(),
        }
    }

    /// Set the server host
    pub fn host<S: Into<String>>(mut self, host: S) -> Self {
        self.config.host = host.into();
        self
    }

    /// Set the server port
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// Enable development mode
    pub fn dev_mode(mut self, dev_mode: bool) -> Self {
        self.config.dev_mode = dev_mode;
        self
    }

    /// Set static files directory
    pub fn static_dir<S: Into<String>>(mut self, static_dir: S) -> Self {
        self.config.static_dir = Some(static_dir.into());
        self
    }

    /// Set database URL
    pub fn database_url<S: Into<String>>(mut self, database_url: S) -> Self {
        self.config.database_url = Some(database_url.into());
        self
    }

    /// Build the server
    pub async fn build(self) -> WebResult<WikifyServer> {
        WikifyServer::new(self.config).await
    }
}

impl Default for WikifyServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to start a server with default configuration
pub async fn start_server() -> WebResult<()> {
    let config = WebConfig::from_env();
    let server = WikifyServer::new(config).await?;
    server.start().await
}

/// Convenience function to start a development server
pub async fn start_dev_server(port: Option<u16>) -> WebResult<()> {
    let server = WikifyServerBuilder::new()
        .host("127.0.0.1")
        .port(port.unwrap_or(8080))
        .dev_mode(true)
        .build()
        .await?;

    server.start().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let config = WebConfig::default();
        let server = WikifyServer::new(config).await;
        assert!(server.is_ok());
    }

    #[test]
    fn test_server_builder() {
        let builder = WikifyServerBuilder::new()
            .host("localhost")
            .port(3000)
            .dev_mode(true);

        assert_eq!(builder.config.host, "localhost");
        assert_eq!(builder.config.port, 3000);
        assert!(builder.config.dev_mode);
    }

    #[test]
    fn test_config_from_env() {
        // Test default values when env vars are not set
        let config = WebConfig::from_env();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(!config.dev_mode);
    }
}
