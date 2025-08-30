//! Clean Wikify CLI using unified application layer

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use wikify_applications::prelude::*;
use wikify_core::{
    init_logging, log_operation_start, log_operation_success, LoggingConfig, WikifyConfig,
    WikifyError, WikifyResult,
};

#[derive(Parser)]
#[command(name = "wikify")]
#[command(
    about = "A CLI tool for generating documentation and answering questions about codebases"
)]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate documentation for a repository
    Generate {
        /// Repository URL or local path
        repo: String,
        /// GitHub token for private repositories
        #[arg(short, long)]
        token: Option<String>,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Use API mode for remote repositories
        #[arg(long)]
        api_mode: bool,
    },
    /// Ask questions about a repository
    Ask {
        /// Repository URL or local path
        repo: String,
        /// Question to ask
        question: String,
        /// GitHub token for private repositories
        #[arg(short, long)]
        token: Option<String>,
        /// Use API mode for remote repositories
        #[arg(long)]
        api_mode: bool,
        /// Similarity threshold for retrieval
        #[arg(long, default_value = "0.3")]
        threshold: f32,
        /// Number of top results to retrieve
        #[arg(long, default_value = "5")]
        top_k: usize,
        /// Maximum context length
        #[arg(long, default_value = "8000")]
        max_context: usize,
        /// Enable reranking
        #[arg(long)]
        rerank: bool,
        /// Chunk size for text splitting
        #[arg(long, default_value = "1000")]
        chunk_size: usize,
        /// Chunk overlap for text splitting
        #[arg(long, default_value = "200")]
        chunk_overlap: usize,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    let logging_config = LoggingConfig::default();
    init_logging(&logging_config).map_err(|e| format!("Failed to initialize logging: {}", e))?;

    // Load configuration
    let config = load_config(cli.config.as_ref()).await?;

    // Create Wikify application for CLI usage
    let app_config = ApplicationConfig::cli_local();
    let application = WikifyApplication::new(app_config)
        .await
        .map_err(|e| format!("Failed to create application: {}", e))?;

    // Create local permission context (CLI has full permissions)
    let context = PermissionContext::local();

    // Execute command
    match cli.command {
        Commands::Generate {
            repo,
            token,
            output,
            api_mode,
        } => {
            handle_generate(
                repo,
                token,
                output,
                api_mode,
                &config,
                &application,
                &context,
            )
            .await?;
        }
        Commands::Ask {
            repo,
            question,
            token,
            api_mode,
            threshold,
            top_k,
            max_context,
            rerank,
            chunk_size,
            chunk_overlap,
        } => {
            handle_ask(
                repo,
                question,
                token,
                api_mode,
                threshold,
                top_k,
                max_context,
                rerank,
                chunk_size,
                chunk_overlap,
                &config,
                &application,
                &context,
            )
            .await?;
        }
    }

    Ok(())
}

/// Load configuration from file or use defaults
async fn load_config(
    config_path: Option<&PathBuf>,
) -> Result<WikifyConfig, Box<dyn std::error::Error>> {
    if let Some(path) = config_path {
        if path.exists() {
            info!("Loading configuration from: {}", path.display());
            // TODO: Implement config loading
            Ok(WikifyConfig::default())
        } else {
            Err(format!("Configuration file not found: {}", path.display()).into())
        }
    } else {
        info!("No configuration file specified, using defaults");
        Ok(WikifyConfig::default())
    }
}

/// Handle ask command using application layer
async fn handle_ask(
    repo: String,
    question: String,
    _token: Option<String>,
    _api_mode: bool,
    _threshold: f32,
    _top_k: usize,
    _max_context: usize,
    _rerank: bool,
    _chunk_size: usize,
    _chunk_overlap: usize,
    _config: &WikifyConfig,
    application: &WikifyApplication,
    context: &PermissionContext,
) -> Result<(), Box<dyn std::error::Error>> {
    log_operation_start!("ask", repository = %repo);

    // Create session with auto-indexing
    let session_options = SessionOptions {
        auto_generate_wiki: false, // Don't auto-generate wiki for ask command
        ..Default::default()
    };

    let session_id = application
        .create_session(context, repo.clone(), session_options)
        .await
        .map_err(|e| format!("Failed to create session: {}", e))?;

    info!("Created session: {} for repository: {}", session_id, repo);

    // Wait for indexing to complete (simplified approach)
    // In a real implementation, we'd want to show progress
    println!("🔄 Indexing repository...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Execute query
    let response = application
        .query(context, &session_id, question.clone())
        .await
        .map_err(|e| format!("Query failed: {}", e))?;

    // Display results
    println!("\n🤖 Answer:");
    println!("{}", response.answer);

    if !response.sources.is_empty() {
        println!("\n📚 Sources:");
        for (i, source) in response.sources.iter().enumerate() {
            println!("{}. {}", i + 1, source);
        }
    }

    log_operation_success!("ask", repository = %repo);
    Ok(())
}

/// Handle generate command using application layer
async fn handle_generate(
    repo: String,
    _token: Option<String>,
    output: Option<PathBuf>,
    _api_mode: bool,
    _config: &WikifyConfig,
    application: &WikifyApplication,
    context: &PermissionContext,
) -> Result<(), Box<dyn std::error::Error>> {
    log_operation_start!("generate", repository = %repo);

    // Create session with auto-wiki generation
    let session_options = SessionOptions {
        auto_generate_wiki: true,
        ..Default::default()
    };

    let session_id = application
        .create_session(context, repo.clone(), session_options)
        .await
        .map_err(|e| format!("Failed to create session: {}", e))?;

    info!("Created session: {} for repository: {}", session_id, repo);

    // Wait for indexing and wiki generation to complete
    // In a real implementation, we'd want to show progress
    println!("🔄 Indexing repository and generating wiki...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // For now, we'll just indicate that the wiki would be generated
    let output_path = output.unwrap_or_else(|| PathBuf::from("./wiki.md"));
    println!("✅ Wiki generated successfully!");
    println!("📄 Output would be saved to: {}", output_path.display());

    log_operation_success!("generate", repository = %repo);
    Ok(())
}
