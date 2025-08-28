//! Wikify Web Server
//!
//! A web interface for Wikify - AI-powered repository documentation and chat.

use clap::Parser;
use wikify_web::server::WikifyServerBuilder;
use wikify_web::{init_logging, WebConfig};

/// Wikify Web Server - AI-powered repository documentation and chat interface
#[derive(Parser)]
#[command(name = "wikify-web")]
#[command(about = "A web interface for Wikify")]
#[command(version)]
struct Args {
    /// Server host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Server port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Enable development mode
    #[arg(long)]
    dev: bool,

    /// Static files directory
    #[arg(long)]
    static_dir: Option<String>,

    /// Database URL for session storage
    #[arg(long)]
    database_url: Option<String>,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // Set up logging first
    std::env::set_var(
        "RUST_LOG",
        format!("wikify_web={},tower_http=debug", args.log_level),
    );
    init_logging();

    println!("🔧 Starting Wikify Web Server initialization...");

    // Load environment variables
    dotenvy::dotenv().ok();

    // Create web configuration
    let mut config = WebConfig::from_env();

    // Override with command line arguments
    config.host = args.host;
    config.port = args.port;
    config.dev_mode = args.dev;
    config.static_dir = args.static_dir;
    config.database_url = args.database_url;

    // Print startup information
    println!("🚀 Starting Wikify Web Server");
    println!("📍 Server: http://{}:{}", config.host, config.port);
    println!("🔧 Development mode: {}", config.dev_mode);

    if let Some(static_dir) = &config.static_dir {
        println!("📁 Static files: {}", static_dir);
    }

    if let Some(db_url) = &config.database_url {
        println!("🗄️  Database: {}", db_url);
    }

    // Check for required environment variables
    let mut missing_vars = Vec::new();

    if std::env::var("OPENAI_API_KEY").is_err()
        && std::env::var("ANTHROPIC_API_KEY").is_err()
        && std::env::var("OLLAMA_HOST").is_err()
    {
        missing_vars.push("LLM API key (OPENAI_API_KEY, ANTHROPIC_API_KEY, or OLLAMA_HOST)");
    }

    if !missing_vars.is_empty() {
        println!("⚠️  Warning: Missing environment variables:");
        for var in missing_vars {
            println!("   - {}", var);
        }
        println!("   The server will start but some features may not work properly.");
        println!("   See README.md for setup instructions.");
    }

    // Build and start the server
    println!("🏗️  Building server...");
    let server = match WikifyServerBuilder::new()
        .host(config.host.clone())
        .port(config.port)
        .dev_mode(config.dev_mode)
        .static_dir(
            config
                .static_dir
                .clone()
                .unwrap_or_else(|| "static".to_string()),
        )
        .database_url(
            config
                .database_url
                .clone()
                .unwrap_or_else(|| "sqlite::memory:".to_string()),
        )
        .build()
        .await
    {
        Ok(server) => {
            println!("✅ Server built successfully");
            server
        }
        Err(e) => {
            eprintln!("❌ Failed to build server: {}", e);
            std::process::exit(1);
        }
    };

    // Start the server (this will block until shutdown)
    println!("🚀 Starting server...");
    if let Err(e) = server.start().await {
        eprintln!("❌ Server failed to start: {}", e);
        std::process::exit(1);
    }

    println!("✅ Server shut down gracefully");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        use clap::Parser;

        // Test default values
        let args = Args::parse_from(&["wikify-web"]);
        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.port, 8080);
        assert!(!args.dev);

        // Test custom values
        let args =
            Args::parse_from(&["wikify-web", "--host", "0.0.0.0", "--port", "3000", "--dev"]);
        assert_eq!(args.host, "0.0.0.0");
        assert_eq!(args.port, 3000);
        assert!(args.dev);
    }
}
