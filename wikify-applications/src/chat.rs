//! Chat Application - Interactive conversation system
//!
//! This module provides a high-level chat application that builds on
//! the basic RAG pipeline to offer conversational AI capabilities.

use crate::{ApplicationError, ApplicationResult, SessionManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};
use wikify_rag::{RagPipeline, RagQuery, RagResponse};

/// Configuration for chat applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    /// Maximum number of messages to keep in context
    pub max_context_messages: usize,
    /// Directory for storing chat history
    pub history_dir: std::path::PathBuf,
    /// Whether to save conversations automatically
    pub auto_save: bool,
    /// Token limit for context window
    pub context_token_limit: usize,
}

impl Default for ChatConfig {
    fn default() -> Self {
        let history_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("wikify")
            .join("chat_history");

        Self {
            max_context_messages: 20,
            history_dir,
            auto_save: true,
            context_token_limit: 4000,
        }
    }
}

/// High-level chat application
pub struct ChatApplication {
    /// Underlying RAG pipeline
    rag_pipeline: RagPipeline,
    /// Session manager for conversation history
    session_manager: SessionManager,
    /// Current active session
    current_session: Option<String>,
    /// Configuration
    config: ChatConfig,
    /// Repository being discussed
    repository: String,
}

impl ChatApplication {
    /// Create a new chat application
    pub async fn new(
        repository: String,
        rag_config: wikify_rag::RagConfig,
        config: ChatConfig,
    ) -> ApplicationResult<Self> {
        info!("Initializing chat application for repository: {}", repository);

        // Create and initialize RAG pipeline
        let mut rag_pipeline = RagPipeline::new(rag_config);
        rag_pipeline.initialize().await?;

        // Index the repository
        info!("Indexing repository for chat...");
        let indexing_stats = rag_pipeline.index_repository(&repository).await?;
        info!("Repository indexed: {}", indexing_stats.summary());

        // Create session manager
        let storage_path = config.history_dir.clone();
        std::fs::create_dir_all(&storage_path)?;
        
        let session_storage = crate::session::SessionStorage::new(storage_path)?;
        let session_manager = SessionManager::new(session_storage);

        Ok(Self {
            rag_pipeline,
            session_manager,
            current_session: None,
            config,
            repository,
        })
    }

    /// Start a new chat session
    pub fn start_session(&mut self) -> ApplicationResult<String> {
        let session_id = self.session_manager.create_session(
            self.repository.clone(),
            "chat".to_string(),
        )?;
        self.current_session = Some(session_id.clone());

        info!("Started new chat session: {}", session_id);
        Ok(session_id)
    }

    /// Resume an existing session
    pub fn resume_session(&mut self, session_id: &str) -> ApplicationResult<()> {
        // Verify session exists
        self.session_manager.get_session(session_id)?;
        self.current_session = Some(session_id.to_string());

        info!("Resumed chat session: {}", session_id);
        Ok(())
    }

    /// Send a message and get response
    pub async fn send_message(&mut self, message: &str) -> ApplicationResult<RagResponse> {
        let session_id = self
            .current_session
            .as_ref()
            .ok_or_else(|| ApplicationError::Session {
                message: "No active session".to_string(),
            })?;

        // Add user message to session
        self.session_manager.add_message(
            session_id,
            "user".to_string(),
            message.to_string(),
            HashMap::new(),
        )?;

        // Get conversation context
        let context_messages = self.session_manager.get_recent_messages(
            session_id,
            self.config.max_context_messages,
        )?;
        
        let conversation_context = self.build_conversation_context(&context_messages);

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
        self.session_manager.add_message(
            session_id,
            "assistant".to_string(),
            response.answer.clone(),
            HashMap::new(),
        )?;

        // Auto-save if enabled
        if self.config.auto_save {
            self.session_manager.save_session(session_id)?;
        }

        debug!("Chat exchange completed for session: {}", session_id);
        Ok(response)
    }

    /// Build conversation context from recent messages
    fn build_conversation_context(&self, messages: &[crate::session::SessionMessage]) -> String {
        if messages.is_empty() {
            return String::new();
        }

        let mut context = String::new();
        context.push_str("Previous conversation:\n");

        for message in messages.iter().rev().take(self.config.max_context_messages) {
            context.push_str(&format!(
                "{}: {}\n",
                message.role,
                message.content
            ));
        }

        context
    }

    /// Get current session ID
    pub fn current_session_id(&self) -> Option<&str> {
        self.current_session.as_deref()
    }

    /// List all sessions for this repository
    pub fn list_sessions(&self) -> ApplicationResult<Vec<String>> {
        self.session_manager.list_sessions_for_repository(&self.repository)
    }

    /// Get session statistics
    pub fn get_session_stats(&self, session_id: &str) -> ApplicationResult<SessionStats> {
        let messages = self.session_manager.get_all_messages(session_id)?;
        
        let user_messages = messages.iter().filter(|m| m.role == "user").count();
        let assistant_messages = messages.iter().filter(|m| m.role == "assistant").count();
        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();

        Ok(SessionStats {
            total_messages: messages.len(),
            user_messages,
            assistant_messages,
            total_characters: total_chars,
            session_duration: messages.last()
                .and_then(|last| messages.first().map(|first| last.timestamp - first.timestamp)),
        })
    }

    /// Save current session
    pub fn save_current_session(&mut self) -> ApplicationResult<()> {
        if let Some(session_id) = &self.current_session {
            self.session_manager.save_session(session_id)?;
            info!("Saved chat session: {}", session_id);
        }
        Ok(())
    }

    /// Clear conversation history for current session
    pub fn clear_current_session(&mut self) -> ApplicationResult<()> {
        if let Some(session_id) = &self.current_session {
            self.session_manager.clear_session_messages(session_id)?;
            info!("Cleared chat session: {}", session_id);
        }
        Ok(())
    }
}

/// Statistics for a chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub total_characters: usize,
    pub session_duration: Option<chrono::Duration>,
}

impl SessionStats {
    pub fn summary(&self) -> String {
        format!(
            "Messages: {} (user: {}, assistant: {}), Characters: {}, Duration: {}",
            self.total_messages,
            self.user_messages,
            self.assistant_messages,
            self.total_characters,
            self.session_duration
                .map(|d| format!("{}m", d.num_minutes()))
                .unwrap_or_else(|| "unknown".to_string())
        )
    }
}
