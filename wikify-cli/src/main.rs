//! Wikify CLI - Command-line interface for Wikify
//!
//! Provides a user-friendly command-line interface for repository analysis and wiki generation

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{error, info};
use wikify_core::{
    init_logging, log_operation_error, log_operation_start, log_operation_success, LoggingConfig,
    WikifyConfig, WikifyError, WikifyResult,
};
use wikify_indexing::create_deepwiki_compatible_pipeline;
use wikify_rag::{
    create_auto_chat_system, create_auto_rag_pipeline, create_simple_query, ChatInterface,
    LlmConfig, WikifyLlmClient,
};
use wikify_repo::RepositoryProcessor;

#[derive(Parser)]
#[command(name = "wikify")]
#[command(about = "A self-hosted wiki generator for code repositories")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate wiki for a repository
    Generate {
        /// Repository URL or local path
        repo: String,

        /// Access token for private repositories
        #[arg(short, long)]
        token: Option<String>,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Ask questions about a repository
    Ask {
        /// Repository URL or local path
        repo: String,

        /// Question to ask
        question: String,

        /// Access token for private repositories
        #[arg(short, long)]
        token: Option<String>,

        /// Similarity threshold for document retrieval (0.0-1.0)
        #[arg(long, default_value = "0.3")]
        threshold: f32,

        /// Number of top documents to retrieve
        #[arg(long, default_value = "5")]
        top_k: usize,

        /// Maximum context length
        #[arg(long, default_value = "8000")]
        max_context: usize,

        /// Enable reranking
        #[arg(long)]
        rerank: bool,

        /// Chunk size for text splitting
        #[arg(long, default_value = "350")]
        chunk_size: usize,

        /// Chunk overlap
        #[arg(long, default_value = "100")]
        chunk_overlap: usize,
    },

    /// Deep research mode for complex topics
    Research {
        /// Repository URL or local path
        repo: String,

        /// Research topic
        topic: String,

        /// Maximum research iterations
        #[arg(long, default_value = "5")]
        max_iterations: usize,

        /// Access token for private repositories
        #[arg(short, long)]
        token: Option<String>,
    },

    /// Interactive chat mode
    Chat {
        /// Repository URL or local path
        repo: String,

        /// Access token for private repositories
        #[arg(short, long)]
        token: Option<String>,

        /// Resume existing session by ID
        #[arg(long)]
        session: Option<String>,

        /// List available sessions
        #[arg(long)]
        list_sessions: bool,

        /// Show sources in responses
        #[arg(long, default_value = "true")]
        show_sources: bool,

        /// Show statistics
        #[arg(long, default_value = "true")]
        show_stats: bool,
    },

    /// Test embedding generation (for debugging)
    TestEmbedding {
        /// Number of test nodes to generate
        #[arg(short, long, default_value = "5")]
        count: usize,
    },

    /// Generate wiki documentation for a repository
    Wiki {
        /// Repository URL or local path
        repo: String,

        /// Output directory for generated wiki
        #[arg(short, long, default_value = "./wiki")]
        output: String,

        /// Export format (markdown, html, json)
        #[arg(short, long, default_value = "markdown")]
        format: String,

        /// Force regeneration even if cache exists
        #[arg(long)]
        force: bool,

        /// Language for content generation
        #[arg(long, default_value = "en")]
        language: String,

        /// Maximum number of pages to generate
        #[arg(long)]
        max_pages: Option<usize>,

        /// Include diagrams and visualizations
        #[arg(long)]
        diagrams: bool,

        /// Access token for private repositories
        #[arg(short, long)]
        token: Option<String>,
    },

    /// Manage configuration
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Initialize default configuration
        #[arg(long)]
        init: bool,

        /// Set a configuration value (key=value format)
        #[arg(long)]
        set: Option<String>,

        /// Get a configuration value
        #[arg(long)]
        get: Option<String>,

        /// Reset configuration to defaults
        #[arg(long)]
        reset: bool,

        /// Validate current configuration
        #[arg(long)]
        validate: bool,
    },
}

#[tokio::main]
async fn main() -> WikifyResult<()> {
    let cli = Cli::parse();

    // Initialize logging with unified system
    let mut logging_config = LoggingConfig::default();
    if cli.verbose {
        logging_config.level = "debug".to_string();
    }

    init_logging(&logging_config).map_err(|e| wikify_core::WikifyError::Config {
        message: format!("Failed to initialize logging: {}", e),
        source: Some(e),
        context: wikify_core::ErrorContext::new("cli")
            .with_operation("init_logging")
            .with_suggestion("Check logging configuration"),
    })?;

    info!("Starting Wikify CLI v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = load_config(cli.config.as_ref()).await?;

    // Execute command
    match cli.command {
        Commands::Generate {
            repo,
            token,
            output,
        } => {
            handle_generate(repo, token, output, &config).await?;
        }
        Commands::Ask {
            repo,
            question,
            token,
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
                threshold,
                top_k,
                max_context,
                rerank,
                chunk_size,
                chunk_overlap,
                &config,
            )
            .await?;
        }
        Commands::Research {
            repo,
            topic,
            max_iterations,
            token,
        } => {
            handle_research(repo, topic, max_iterations, token, &config).await?;
        }
        Commands::Chat {
            repo,
            token,
            session,
            list_sessions,
            show_sources,
            show_stats,
        } => {
            handle_chat(
                repo,
                token,
                session,
                list_sessions,
                show_sources,
                show_stats,
                &config,
            )
            .await?;
        }
        Commands::TestEmbedding { count } => {
            handle_test_embedding(count, &config).await?;
        }
        Commands::Wiki {
            repo,
            output,
            format,
            force,
            language,
            max_pages,
            diagrams,
            token,
        } => {
            handle_wiki(
                repo, output, format, force, language, max_pages, diagrams, token, &config,
            )
            .await?;
        }

        Commands::Config {
            show,
            init,
            set,
            get,
            reset,
            validate,
        } => {
            handle_config(show, init, set, get, reset, validate).await?;
        }
    }

    Ok(())
}

async fn load_config(config_path: Option<&PathBuf>) -> WikifyResult<WikifyConfig> {
    if let Some(path) = config_path {
        info!("Loading configuration from {:?}", path);
        WikifyConfig::from_file(path)
    } else {
        // Try to load from default locations
        let default_paths = [
            dirs::config_dir().map(|d| d.join("wikify").join("config.toml")),
            dirs::home_dir().map(|d| d.join(".wikify").join("config.toml")),
            Some(PathBuf::from("wikify.toml")),
        ];

        for path_opt in default_paths.iter() {
            if let Some(path) = path_opt {
                if path.exists() {
                    info!("Loading configuration from {:?}", path);
                    return WikifyConfig::from_file(path);
                }
            }
        }

        info!("No configuration file found, using defaults");
        Ok(WikifyConfig::default())
    }
}

async fn handle_generate(
    repo: String,
    token: Option<String>,
    output: Option<PathBuf>,
    config: &WikifyConfig,
) -> WikifyResult<()> {
    log_operation_start!("generate_wiki", repo = %repo);

    // Parse repository information
    let mut repo_info = RepositoryProcessor::parse_repo_url(&repo).map_err(|e| {
        log_operation_error!("parse_repo_url", e, repo = %repo);
        e
    })?;
    repo_info.access_token = token;

    // Initialize repository processor
    let data_dir = PathBuf::from(
        &config
            .storage
            .data_dir
            .replace("~", &dirs::home_dir().unwrap().to_string_lossy()),
    );
    let processor = RepositoryProcessor::new(&data_dir);

    // Clone repository with error handling
    let repo_path = processor.clone_repository(&repo_info).await.map_err(|e| {
        log_operation_error!("clone_repository", e, repo = %repo);
        e
    })?;

    info!(repo_path = %repo_path, "Repository cloned successfully");

    // Create indexing pipeline
    println!("📚 Starting document indexing...");
    let pipeline = create_deepwiki_compatible_pipeline(&repo_path).map_err(|e| {
        log_operation_error!("create_pipeline", e, repo_path = %repo_path);
        e
    })?;

    // Run indexing pipeline
    let result = pipeline.run().await.map_err(|e| {
        log_operation_error!("run_indexing_pipeline", e, repo_path = %repo_path);
        e
    })?;

    // Display results
    println!("✅ Repository indexed successfully!");
    println!("📁 Location: {}", repo_path);
    println!("📊 {}", result.summary());

    // Show breakdown by file type
    let by_type = result.documents_by_type();
    println!("\n📋 Documents by type:");
    for (file_type, docs) in by_type {
        println!(
            "  {} {}: {} files",
            match file_type.as_str() {
                "code" => "💻",
                "documentation" => "📝",
                "configuration" => "⚙️",
                _ => "📄",
            },
            file_type,
            docs.len()
        );
    }

    if let Some(output_path) = output {
        println!("\n📝 Wiki will be generated to: {:?}", output_path);
        println!("🚧 Wiki generation is not yet implemented. Coming soon!");
    }

    log_operation_success!("generate_wiki",
        repo = %repo,
        repo_path = %repo_path,
        total_documents = result.stats.total_documents,
        total_nodes = result.stats.total_nodes
    );
    Ok(())
}

async fn handle_ask(
    repo: String,
    question: String,
    _token: Option<String>,
    threshold: f32,
    top_k: usize,
    max_context: usize,
    rerank: bool,
    chunk_size: usize,
    chunk_overlap: usize,
    config: &WikifyConfig,
) -> WikifyResult<()> {
    log_operation_start!("ask_question");

    println!("🚀 Starting ask command...");
    info!("Asking question about repository: {}", repo);
    info!("Question: {}", question);

    println!("📂 Repository: {}", repo);
    println!("❓ Question: {}", question);

    // Parse repository info
    println!("🔍 Parsing repository URL...");
    let repo_info = RepositoryProcessor::parse_repo_url(&repo).map_err(|e| {
        println!("❌ Failed to parse repository URL: {}", e);
        log_operation_error!("parse_repo_url", e, repo = %repo);
        e
    })?;
    println!("✅ Repository parsed successfully");

    // Get repository path
    println!("📁 Processing repository...");
    let processor = RepositoryProcessor::new(&config.storage.data_dir);
    let repo_path = processor.clone_repository(&repo_info).await.map_err(|e| {
        println!("❌ Failed to process repository: {}", e);
        log_operation_error!("clone_repository", e, repo = %repo);
        e
    })?;
    println!("✅ Repository processed: {}", repo_path);

    println!("🤖 Initializing RAG system...");

    // Create custom RAG configuration from CLI parameters
    let mut rag_config = wikify_rag::RagConfig::default();

    // Override with CLI parameters
    rag_config.retrieval.similarity_threshold = threshold;
    rag_config.retrieval.top_k = top_k;
    rag_config.retrieval.max_context_length = max_context;
    rag_config.retrieval.enable_reranking = rerank;

    // Note: Indexing configuration (chunk_size, chunk_overlap) will be handled
    // by the indexing pipeline separately

    // Auto-detect LLM provider
    if std::env::var("OPENAI_API_KEY").is_ok() {
        rag_config.llm = wikify_rag::llm_client::configs::openai_gpt4o_mini();
        rag_config.embeddings.provider = "openai".to_string();
        rag_config.embeddings.model = "text-embedding-3-small".to_string();
    } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        rag_config.llm = wikify_rag::llm_client::configs::anthropic_claude_haiku();
    } else if std::env::var("GROQ_API_KEY").is_ok() {
        rag_config.llm = wikify_rag::llm_client::configs::groq_llama3();
    } else {
        rag_config.llm = wikify_rag::llm_client::configs::ollama_llama3(None);
    }

    // Create and initialize RAG pipeline with custom config
    let mut rag_pipeline = wikify_rag::RagPipeline::new(rag_config);
    rag_pipeline.initialize().await
        .map_err(|e| {
            log_operation_error!("initialize_rag_pipeline", e);
            WikifyError::Repository {
                message: format!("Failed to initialize RAG pipeline: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("ask_command")
                    .with_operation("initialize_rag_pipeline")
                    .with_suggestion("Check if LLM API keys are configured (OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.)"),
            }
        })?;

    println!("📚 Indexing repository...");

    // Index the repository
    let indexing_stats = rag_pipeline
        .index_repository(&repo_path)
        .await
        .map_err(|e| {
            log_operation_error!("index_repository", e, repo_path = %repo_path);
            WikifyError::Repository {
                message: format!("Failed to index repository: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("ask_command")
                    .with_operation("index_repository"),
            }
        })?;

    println!("✅ Repository indexed: {}", indexing_stats.summary());
    println!("🔍 Processing your question...");

    // Create query and get response
    let query = create_simple_query(&question);
    let response = rag_pipeline.ask(query).await.map_err(|e| {
        log_operation_error!("rag_ask", e, question = %question);
        WikifyError::Repository {
            message: format!("Failed to process question: {}", e),
            source: Some(Box::new(e)),
            context: wikify_core::ErrorContext::new("ask_command").with_operation("rag_ask"),
        }
    })?;

    // Display results
    println!("\n🎯 **Answer:**");
    println!("{}", response.answer);

    if !response.sources.is_empty() {
        println!("\n📋 **Sources ({} chunks):**", response.sources.len());
        for (i, source) in response.sources.iter().take(3).enumerate() {
            let file_path = source
                .chunk
                .metadata
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!(
                "  {}. {} (similarity: {:.2})",
                i + 1,
                file_path,
                source.score
            );
        }

        if response.sources.len() > 3 {
            println!("  ... and {} more sources", response.sources.len() - 3);
        }
    }

    println!("\n📊 **Stats:**");
    println!(
        "  • Retrieved: {} chunks",
        response.metadata.chunks_retrieved
    );
    println!("  • Context tokens: ~{}", response.metadata.context_tokens);
    println!(
        "  • Generation tokens: ~{}",
        response.metadata.generation_tokens
    );
    println!("  • Model: {}", response.metadata.model_used);
    println!(
        "  • Total time: {}ms",
        response.metadata.retrieval_time_ms + response.metadata.generation_time_ms
    );

    log_operation_success!("ask_question",
        repo = %repo,
        question = %question,
        chunks_retrieved = response.metadata.chunks_retrieved,
        total_time_ms = response.metadata.retrieval_time_ms + response.metadata.generation_time_ms
    );

    Ok(())
}

async fn handle_research(
    repo: String,
    topic: String,
    max_iterations: usize,
    token: Option<String>,
    config: &WikifyConfig,
) -> WikifyResult<()> {
    info!("Starting deep research on topic: {}", topic);

    // TODO: Implement deep research functionality
    println!("🔬 Deep research functionality is not yet implemented. Coming soon!");
    println!("📝 Topic: {}", topic);
    println!("🔄 Max iterations: {}", max_iterations);

    Ok(())
}

async fn handle_chat(
    repo: String,
    _token: Option<String>,
    session: Option<String>,
    list_sessions: bool,
    show_sources: bool,
    show_stats: bool,
    config: &WikifyConfig,
) -> WikifyResult<()> {
    log_operation_start!("chat_mode");

    info!("Starting interactive chat mode for repository: {}", repo);

    // Parse repository info
    let repo_info = RepositoryProcessor::parse_repo_url(&repo).map_err(|e| {
        log_operation_error!("parse_repo_url", e, repo = %repo);
        e
    })?;

    // Get repository path
    let processor = RepositoryProcessor::new(&config.storage.data_dir);
    let repo_path = processor.clone_repository(&repo_info).await.map_err(|e| {
        log_operation_error!("clone_repository", e, repo = %repo);
        e
    })?;

    println!("🤖 Initializing chat system...");

    // Create chat system
    let mut chat_system = create_auto_chat_system(&repo_path).await
        .map_err(|e| {
            log_operation_error!("create_chat_system", e);
            WikifyError::Repository {
                message: format!("Failed to create chat system: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("chat_command")
                    .with_operation("create_chat_system")
                    .with_suggestion("Check if LLM API keys are configured (OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.)"),
            }
        })?;

    // Handle list sessions command
    if list_sessions {
        println!("📋 **Available Chat Sessions:**");
        match chat_system.list_sessions() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    println!("  No saved sessions found.");
                } else {
                    for (i, session_id) in sessions.iter().enumerate() {
                        println!("  {}. {}", i + 1, session_id);
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to list sessions: {}", e);
            }
        }
        return Ok(());
    }

    // Handle session resumption
    if let Some(session_id) = session {
        println!("🔄 Resuming session: {}", session_id);
        match chat_system.resume_session(&session_id) {
            Ok(()) => {
                println!("✅ Session resumed successfully!");

                // Show previous messages
                let messages = chat_system.get_session_messages();
                if !messages.is_empty() {
                    println!("\n📜 **Previous Conversation:**");
                    for message in messages.iter().take(10) {
                        // Show last 10 messages
                        let role_icon = if message.role == "user" {
                            "💬"
                        } else {
                            "🤖"
                        };
                        let role_name = if message.role == "user" {
                            "You"
                        } else {
                            "Assistant"
                        };
                        println!(
                            "{} {}: {}",
                            role_icon,
                            role_name,
                            if message.content.len() > 100 {
                                format!("{}...", &message.content[..100])
                            } else {
                                message.content.clone()
                            }
                        );
                    }
                    println!();
                }
            }
            Err(e) => {
                println!("❌ Failed to resume session: {}", e);
                println!("🆕 Starting new session instead...");
                chat_system
                    .start_session()
                    .map_err(|e| WikifyError::Repository {
                        message: format!("Failed to start new session: {}", e),
                        source: Some(Box::new(e)),
                        context: wikify_core::ErrorContext::new("chat_command"),
                    })?;
            }
        }
    } else {
        // Start new session
        chat_system
            .start_session()
            .map_err(|e| WikifyError::Repository {
                message: format!("Failed to start new session: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("chat_command"),
            })?;
    }

    // Create chat interface with user preferences
    let mut chat_interface =
        ChatInterface::new(chat_system).with_display_options(show_sources, show_stats);

    // Start interactive chat
    chat_interface.start_interactive().await.map_err(|e| {
        log_operation_error!("interactive_chat", e);
        WikifyError::Repository {
            message: format!("Chat session failed: {}", e),
            source: Some(Box::new(e)),
            context: wikify_core::ErrorContext::new("chat_command")
                .with_operation("interactive_chat"),
        }
    })?;

    log_operation_success!("chat_mode", repo = %repo);
    Ok(())
}

async fn handle_test_embedding(count: usize, _config: &WikifyConfig) -> WikifyResult<()> {
    log_operation_start!("test_embedding");

    info!("Testing embedding generation with {} nodes", count);

    // Check if API key is available
    if std::env::var("OPENAI_API_KEY").is_err() {
        println!("❌ OPENAI_API_KEY not found. Please set it to test embeddings.");
        println!("   export OPENAI_API_KEY=your_key_here");
        return Ok(());
    }

    println!("🧪 Testing embedding generation with {} texts...", count);

    // Create LLM config for embeddings
    let config = LlmConfig {
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        api_key: None, // Will use environment variable
        base_url: None,
        temperature: 0.0,
        max_tokens: None,
    };

    // Create LLM client
    let client = WikifyLlmClient::new(config)
        .await
        .map_err(|e| WikifyError::Repository {
            message: format!("Failed to create LLM client: {}", e),
            source: Some(Box::new(e)),
            context: wikify_core::ErrorContext::new("test_embedding"),
        })?;

    // Create test text samples
    let test_texts: Vec<String> = (0..count).map(|i| {
        format!("This is test document number {}. It contains some sample text for embedding generation testing. The content varies to ensure different embeddings.", i + 1)
    }).collect();

    println!("📝 Created {} test texts", test_texts.len());

    // Test direct embedding generation
    let start_time = std::time::Instant::now();

    println!("🔄 Testing embedding API directly...");
    let embeddings = client
        .generate_embeddings(test_texts.clone())
        .await
        .map_err(|e| WikifyError::Repository {
            message: format!("Failed to generate embeddings: {}", e),
            source: Some(Box::new(e)),
            context: wikify_core::ErrorContext::new("test_embedding"),
        })?;
    let duration = start_time.elapsed();

    println!(
        "✅ Generated {} embeddings in {:?}",
        embeddings.len(),
        duration
    );

    // Verify embeddings
    for (i, (text, embedding)) in test_texts.iter().zip(embeddings.iter()).enumerate() {
        println!(
            "  {}. Content: {}",
            i + 1,
            if text.len() > 50 {
                format!("{}...", &text[..50])
            } else {
                text.clone()
            }
        );
        println!("     Embedding dimension: {}", embedding.len());
        println!(
            "     First 5 values: {:?}",
            &embedding[..5.min(embedding.len())]
        );
    }

    println!("🎉 Embedding test completed successfully!");

    log_operation_success!(
        "test_embedding",
        count = count,
        duration_ms = duration.as_millis()
    );
    Ok(())
}

async fn handle_config(
    show: bool,
    init: bool,
    set: Option<String>,
    get: Option<String>,
    reset: bool,
    validate: bool,
) -> WikifyResult<()> {
    if init {
        let config = WikifyConfig::default();
        let config_dir = dirs::config_dir()
            .or_else(|| dirs::home_dir().map(|d| d.join(".config")))
            .unwrap()
            .join("wikify");

        tokio::fs::create_dir_all(&config_dir).await?;
        let config_path = config_dir.join("config.toml");

        config.save_to_file(&config_path)?;
        println!("✅ Configuration initialized at: {:?}", config_path);
        println!("📝 Please edit the file to add your API keys and customize settings.");
    }

    if show {
        let config = load_config(None).await?;
        println!("📋 Current configuration:");
        println!("{}", toml::to_string_pretty(&config).unwrap());
    }

    if reset {
        let config = WikifyConfig::default();
        let config_path = get_config_path();
        config.save_to_file(&config_path)?;
        println!("🔄 Configuration reset to defaults at: {:?}", config_path);
    }

    if validate {
        let config = load_config(None).await?;
        match config.validate() {
            Ok(()) => println!("✅ Configuration is valid"),
            Err(e) => {
                println!("❌ Configuration validation failed: {}", e);
                return Err(e);
            }
        }
    }

    if let Some(key_value) = set {
        if let Some((key, value)) = key_value.split_once('=') {
            set_config_value(key, value).await?;
            println!("✅ Set {} = {}", key, value);
        } else {
            return Err(WikifyError::Config {
                message: "Invalid format. Use key=value format".to_string(),
                source: None,
                context: wikify_core::ErrorContext::new("config_set")
                    .with_suggestion("Example: --set rag.similarity_threshold=0.6"),
            });
        }
    }

    if let Some(key) = get {
        let value = get_config_value(&key).await?;
        println!("{} = {}", key, value);
    }

    Ok(())
}

/// Get the default configuration file path
fn get_config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|d| d.join(".config")))
        .unwrap()
        .join("wikify")
        .join("config.toml")
}

/// Set a configuration value
async fn set_config_value(key: &str, value: &str) -> WikifyResult<()> {
    let config_path = get_config_path();
    let mut config = if config_path.exists() {
        WikifyConfig::from_file(&config_path)?
    } else {
        WikifyConfig::default()
    };

    // Parse the key path (e.g., "rag.similarity_threshold")
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        ["rag", "similarity_threshold"] => {
            config.rag.similarity_threshold = value.parse().map_err(|_| WikifyError::Config {
                message: format!("Invalid float value: {}", value),
                source: None,
                context: wikify_core::ErrorContext::new("config_set"),
            })?;
        }
        ["rag", "top_k"] => {
            config.rag.top_k = value.parse().map_err(|_| WikifyError::Config {
                message: format!("Invalid integer value: {}", value),
                source: None,
                context: wikify_core::ErrorContext::new("config_set"),
            })?;
        }
        ["rag", "max_context_length"] => {
            config.rag.max_context_length = value.parse().map_err(|_| WikifyError::Config {
                message: format!("Invalid integer value: {}", value),
                source: None,
                context: wikify_core::ErrorContext::new("config_set"),
            })?;
        }
        ["rag", "enable_reranking"] => {
            config.rag.enable_reranking = value.parse().map_err(|_| WikifyError::Config {
                message: format!("Invalid boolean value: {}", value),
                source: None,
                context: wikify_core::ErrorContext::new("config_set"),
            })?;
        }
        ["indexing", "chunk_size"] => {
            config.indexing.chunk_size = value.parse().map_err(|_| WikifyError::Config {
                message: format!("Invalid integer value: {}", value),
                source: None,
                context: wikify_core::ErrorContext::new("config_set"),
            })?;
        }
        ["indexing", "chunk_overlap"] => {
            config.indexing.chunk_overlap = value.parse().map_err(|_| WikifyError::Config {
                message: format!("Invalid integer value: {}", value),
                source: None,
                context: wikify_core::ErrorContext::new("config_set"),
            })?;
        }
        ["llm", "model"] => {
            config.llm.model = value.to_string();
        }
        ["llm", "temperature"] => {
            config.llm.temperature = value.parse().map_err(|_| WikifyError::Config {
                message: format!("Invalid float value: {}", value),
                source: None,
                context: wikify_core::ErrorContext::new("config_set"),
            })?;
        }
        _ => {
            return Err(WikifyError::Config {
                message: format!("Unknown configuration key: {}", key),
                source: None,
                context: wikify_core::ErrorContext::new("config_set")
                    .with_suggestion("Use --show to see available configuration keys"),
            });
        }
    }

    // Ensure config directory exists
    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    config.save_to_file(&config_path)?;
    Ok(())
}

/// Get a configuration value
async fn get_config_value(key: &str) -> WikifyResult<String> {
    let config = load_config(None).await?;

    let parts: Vec<&str> = key.split('.').collect();
    let value = match parts.as_slice() {
        ["rag", "similarity_threshold"] => config.rag.similarity_threshold.to_string(),
        ["rag", "top_k"] => config.rag.top_k.to_string(),
        ["rag", "max_context_length"] => config.rag.max_context_length.to_string(),
        ["rag", "enable_reranking"] => config.rag.enable_reranking.to_string(),
        ["indexing", "chunk_size"] => config.indexing.chunk_size.to_string(),
        ["indexing", "chunk_overlap"] => config.indexing.chunk_overlap.to_string(),
        ["llm", "model"] => config.llm.model.clone(),
        ["llm", "temperature"] => config.llm.temperature.to_string(),
        _ => {
            return Err(WikifyError::Config {
                message: format!("Unknown configuration key: {}", key),
                source: None,
                context: wikify_core::ErrorContext::new("config_get")
                    .with_suggestion("Use --show to see available configuration keys"),
            });
        }
    };

    Ok(value)
}

/// Handle wiki generation command
async fn handle_wiki(
    repo: String,
    output: String,
    format: String,
    force: bool,
    language: String,
    max_pages: Option<usize>,
    diagrams: bool,
    _token: Option<String>,
    _config: &WikifyConfig,
) -> WikifyResult<()> {
    println!("📚 Starting wiki generation for repository: {}", repo);
    println!("📁 Output directory: {}", output);
    println!("📄 Format: {}", format);

    // Parse export format
    let export_format = match format.to_lowercase().as_str() {
        "markdown" | "md" => wikify_wiki::ExportFormat::Markdown,
        "html" => wikify_wiki::ExportFormat::Html,
        "json" => wikify_wiki::ExportFormat::Json,
        "pdf" => wikify_wiki::ExportFormat::Pdf,
        _ => {
            return Err(WikifyError::Config {
                message: format!("Unsupported export format: {}", format),
                source: None,
                context: wikify_core::ErrorContext::new("wiki_command")
                    .with_suggestion("Supported formats: markdown, html, json, pdf"),
            });
        }
    };

    // Create wiki configuration
    let mut wiki_config = wikify_wiki::WikiConfig::default();
    wiki_config.force_regenerate = force;
    wiki_config.language = language;
    wiki_config.max_pages = max_pages;
    wiki_config.include_diagrams = diagrams;

    // Create wiki service
    println!("🔧 Creating wiki service...");
    let mut wiki_service = wikify_wiki::WikiService::new().map_err(|e| {
        println!("❌ Failed to create wiki service: {}", e);
        log_operation_error!("create_wiki_service", e);
        WikifyError::Repository {
            message: format!("Failed to create wiki service: {}", e),
            source: None,
            context: wikify_core::ErrorContext::new("wiki_command")
                .with_operation("create_wiki_service")
                .with_suggestion("Check if all dependencies are properly configured"),
        }
    })?;
    println!("✅ Wiki service created successfully");

    // Check for cached wiki first
    if !force {
        if let Ok(Some(cached_wiki)) = wiki_service.get_cached_wiki(&repo).await {
            println!("📋 Found cached wiki, using cached version (use --force to regenerate)");

            // Export cached wiki
            wiki_service
                .export_wiki(&cached_wiki, export_format, &output)
                .await
                .map_err(|e| {
                    log_operation_error!("export_cached_wiki", e);
                    WikifyError::Repository {
                        message: format!("Failed to export cached wiki: {}", e),
                        source: None,
                        context: wikify_core::ErrorContext::new("wiki_command")
                            .with_operation("export_cached_wiki"),
                    }
                })?;

            println!("✅ Cached wiki exported to: {}", output);
            return Ok(());
        }
    }

    // Generate new wiki
    println!("🔄 Generating wiki structure and content...");
    let wiki_structure = wiki_service
        .generate_wiki(&repo, &wiki_config)
        .await
        .map_err(|e| {
            log_operation_error!("generate_wiki", e);
            WikifyError::Repository {
                message: format!("Failed to generate wiki: {}", e),
                source: None,
                context: wikify_core::ErrorContext::new("wiki_command")
                    .with_operation("generate_wiki")
                    .with_suggestion("Check if the repository path is valid and accessible"),
            }
        })?;

    // Export wiki
    println!("📤 Exporting wiki as {} to: {}", format, output);
    wiki_service
        .export_wiki(&wiki_structure, export_format, &output)
        .await
        .map_err(|e| {
            log_operation_error!("export_wiki", e);
            WikifyError::Repository {
                message: format!("Failed to export wiki: {}", e),
                source: None,
                context: wikify_core::ErrorContext::new("wiki_command")
                    .with_operation("export_wiki"),
            }
        })?;

    // Print summary
    println!("\n🎉 Wiki generation completed!");
    println!("📊 Statistics:");
    println!("   • Pages generated: {}", wiki_structure.pages.len());
    println!("   • Sections created: {}", wiki_structure.sections.len());
    println!(
        "   • Total reading time: {} minutes",
        wiki_structure.total_reading_time()
    );
    println!("   • Export format: {}", format);
    println!("   • Output location: {}", output);

    if let Some(stats) = wiki_structure
        .metadata
        .stats
        .total_tokens_used
        .checked_sub(0)
    {
        if stats > 0 {
            println!("   • Tokens used: {}", stats);
            if wiki_structure.metadata.stats.estimated_cost > 0.0 {
                println!(
                    "   • Estimated cost: ${:.4}",
                    wiki_structure.metadata.stats.estimated_cost
                );
            }
        }
    }

    Ok(())
}
