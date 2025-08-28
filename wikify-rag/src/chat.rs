//! Interactive chat system with multi-turn conversation support
//!
//! This module provides an interactive chat interface that maintains
//! conversation context and supports session management.

use crate::rag_pipeline::RagPipeline;
use crate::storage::ChatSessionManager;
use crate::token_counter::count_tokens;
use crate::types::{
    ChatConfig, ChatMessage, RagConfig, RagError, RagQuery, RagResponse, RagResult, StorageConfig,
};
use std::path::Path;
use tracing::{debug, info};

/// Interactive chat system with persistent sessions
pub struct ChatSystem {
    /// RAG pipeline for question answering
    rag_pipeline: RagPipeline,
    /// Chat session manager
    session_manager: ChatSessionManager,
    /// Current session ID
    current_session: Option<String>,
    /// Chat configuration
    chat_config: ChatConfig,
    /// Repository path
    repository: String,
}

impl ChatSystem {
    /// Create a new chat system
    pub async fn new(
        repository: String,
        rag_config: RagConfig,
        chat_config: ChatConfig,
        _storage_config: StorageConfig,
    ) -> RagResult<Self> {
        info!("Initializing chat system for repository: {}", repository);

        // Create RAG pipeline with persistent storage
        let mut rag_pipeline = RagPipeline::new(rag_config);
        rag_pipeline.initialize().await?;

        // Index the repository
        info!("Indexing repository for chat...");
        let indexing_stats = rag_pipeline.index_repository(&repository).await?;
        info!("Repository indexed: {}", indexing_stats.summary());

        // Create session manager
        let session_manager = ChatSessionManager::new(chat_config.clone())?;

        Ok(Self {
            rag_pipeline,
            session_manager,
            current_session: None,
            chat_config,
            repository,
        })
    }

    /// Start a new chat session
    pub fn start_session(&mut self) -> RagResult<String> {
        let session_id = self.session_manager.create_session(self.repository.clone());
        self.current_session = Some(session_id.clone());

        info!("Started new chat session: {}", session_id);
        Ok(session_id)
    }

    /// Resume an existing session
    pub fn resume_session(&mut self, session_id: &str) -> RagResult<()> {
        self.session_manager.load_session(session_id)?;
        self.current_session = Some(session_id.to_string());

        info!("Resumed chat session: {}", session_id);
        Ok(())
    }

    /// Send a message and get response
    pub async fn send_message(&mut self, message: &str) -> RagResult<RagResponse> {
        let session_id = self
            .current_session
            .as_ref()
            .ok_or_else(|| RagError::Config("No active session".to_string()))?;

        // Add user message to session
        self.session_manager
            .add_message(session_id, "user", message)?;

        // Get conversation context
        let context_messages = self.session_manager.get_context_messages(session_id);
        let conversation_context = self.build_conversation_context(&context_messages);
        let should_save = context_messages.len() % 5 == 0;

        // Create RAG query with conversation context
        let query = RagQuery {
            question: message.to_string(),
            context: Some(conversation_context),
            filters: None,
            retrieval_config: None,
        };

        // Get response from RAG pipeline
        let response = self.rag_pipeline.ask(query).await?;

        // Add assistant response to session
        self.session_manager
            .add_message(session_id, "assistant", &response.answer)?;

        // Save session periodically
        if should_save {
            self.session_manager.save_session(session_id)?;
        }

        debug!("Chat exchange completed for session: {}", session_id);
        Ok(response)
    }

    /// Build conversation context from messages with accurate token counting
    fn build_conversation_context(&self, messages: &[&ChatMessage]) -> String {
        if messages.is_empty() {
            return String::new();
        }

        let mut context_parts = Vec::new();
        let mut token_count = 0;

        // Get model name for accurate token counting
        // Default to gpt-4o if not available
        let model_name = "gpt-4o"; // TODO: Get from RAG pipeline config

        // Take recent messages within token limit
        for message in messages.iter().rev() {
            let message_text = format!(
                "{}: {}",
                if message.role == "user" {
                    "User"
                } else {
                    "Assistant"
                },
                message.content
            );

            // Use accurate token counting
            let message_tokens = match count_tokens(&message_text, model_name) {
                Ok(tokens) => tokens,
                Err(_) => {
                    // Fallback to rough estimation if token counting fails
                    message_text.len() / 4
                }
            };

            if token_count + message_tokens > self.chat_config.max_context_tokens {
                break;
            }

            context_parts.push(message_text);
            token_count += message_tokens;
        }

        // Reverse to get chronological order
        context_parts.reverse();
        context_parts.join("\n\n")
    }

    /// Get current session messages
    pub fn get_session_messages(&self) -> Vec<&ChatMessage> {
        if let Some(session_id) = &self.current_session {
            self.session_manager.get_context_messages(session_id)
        } else {
            Vec::new()
        }
    }

    /// List available sessions
    pub fn list_sessions(&self) -> RagResult<Vec<String>> {
        self.session_manager.list_sessions()
    }

    /// Save current session
    pub fn save_current_session(&self) -> RagResult<()> {
        if let Some(session_id) = &self.current_session {
            self.session_manager.save_session(session_id)?;
            info!("Saved current session: {}", session_id);
        }
        Ok(())
    }

    /// Get current session ID
    pub fn current_session_id(&self) -> Option<&str> {
        self.current_session.as_deref()
    }

    /// Get repository path
    pub fn repository(&self) -> &str {
        &self.repository
    }

    /// Get chat statistics
    pub fn get_stats(&self) -> ChatStats {
        let messages = self.get_session_messages();
        let user_messages = messages.iter().filter(|m| m.role == "user").count();
        let assistant_messages = messages.iter().filter(|m| m.role == "assistant").count();

        ChatStats {
            total_messages: messages.len(),
            user_messages,
            assistant_messages,
            session_id: self.current_session.clone(),
            repository: self.repository.clone(),
        }
    }
}

/// Statistics about the chat session
#[derive(Debug, Clone)]
pub struct ChatStats {
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub session_id: Option<String>,
    pub repository: String,
}

impl ChatStats {
    pub fn summary(&self) -> String {
        format!(
            "Chat: {} messages ({} user, {} assistant) in session {}",
            self.total_messages,
            self.user_messages,
            self.assistant_messages,
            self.session_id.as_deref().unwrap_or("none")
        )
    }
}

/// Helper function to create a chat system with auto-detected configuration
pub async fn create_auto_chat_system<P: AsRef<Path>>(repo_path: P) -> RagResult<ChatSystem> {
    let repo_path_str = repo_path.as_ref().to_string_lossy().to_string();

    // Use default configurations
    let rag_config = RagConfig::default();
    let chat_config = ChatConfig::default();
    let storage_config = StorageConfig::default();

    ChatSystem::new(repo_path_str, rag_config, chat_config, storage_config).await
}

/// Interactive chat interface for CLI
pub struct ChatInterface {
    chat_system: ChatSystem,
    show_sources: bool,
    show_stats: bool,
}

impl ChatInterface {
    /// Create a new chat interface
    pub fn new(chat_system: ChatSystem) -> Self {
        Self {
            chat_system,
            show_sources: true,
            show_stats: true,
        }
    }

    /// Configure display options
    pub fn with_display_options(mut self, show_sources: bool, show_stats: bool) -> Self {
        self.show_sources = show_sources;
        self.show_stats = show_stats;
        self
    }

    /// Start interactive chat session
    pub async fn start_interactive(&mut self) -> RagResult<()> {
        // Start a new session
        let session_id = self.chat_system.start_session()?;

        println!("🤖 **Wikify Chat Started**");
        println!("📁 Repository: {}", self.chat_system.repository());
        println!("🆔 Session: {}", session_id);
        println!("💡 Type 'help' for commands, 'quit' to exit\n");

        // Interactive loop
        loop {
            print!("💬 You: ");
            use std::io::{self, Write};
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            match input.to_lowercase().as_str() {
                "quit" | "exit" | "q" => {
                    self.chat_system.save_current_session()?;
                    println!("👋 Chat session saved. Goodbye!");
                    break;
                }
                "help" | "h" => {
                    self.show_help();
                    continue;
                }
                "stats" => {
                    let stats = self.chat_system.get_stats();
                    println!("📊 {}", stats.summary());
                    continue;
                }
                "save" => {
                    self.chat_system.save_current_session()?;
                    println!("💾 Session saved!");
                    continue;
                }
                _ => {}
            }

            // Send message and get response
            print!("🤔 Thinking...");
            io::stdout().flush().unwrap();

            match self.chat_system.send_message(input).await {
                Ok(response) => {
                    print!("\r🤖 Assistant: ");
                    println!("{}\n", response.answer);

                    if self.show_sources && !response.sources.is_empty() {
                        println!("📚 Sources:");
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
                        println!();
                    }

                    if self.show_stats {
                        println!(
                            "⚡ Stats: {} chunks, {}ms, model: {}\n",
                            response.metadata.chunks_retrieved,
                            response.metadata.retrieval_time_ms
                                + response.metadata.generation_time_ms,
                            response.metadata.model_used
                        );
                    }
                }
                Err(e) => {
                    println!("\r❌ Error: {}\n", e);
                }
            }
        }

        Ok(())
    }

    /// Show help message
    fn show_help(&self) {
        println!("🔧 **Available Commands:**");
        println!("  help, h     - Show this help message");
        println!("  stats       - Show session statistics");
        println!("  save        - Save current session");
        println!("  quit, exit  - Save and exit chat");
        println!("  <message>   - Ask a question about the repository\n");
    }
}
