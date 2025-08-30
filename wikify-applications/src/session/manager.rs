//! Session Manager - Permission-aware session management
//!
//! Provides comprehensive session management with built-in permission checking
//! and resource limit enforcement.

use super::{
    ApplicationSession, IndexingUpdate, QueryContext, QueryMessage, RepositoryInfo, SessionConfig,
    SessionInfo, SessionOptions,
};
use crate::auth::{Permission, PermissionContext};
use crate::{ApplicationError, ApplicationResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info};
use wikify_rag::{IndexingManager, RagConfig, RagPipeline};

/// Permission-aware session manager
pub struct SessionManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, ApplicationSession>>>,
    /// Default session configuration
    default_config: SessionConfig,
    /// RAG configuration for creating pipelines
    rag_config: RagConfig,
    /// Progress broadcaster for indexing operations
    progress_broadcaster: broadcast::Sender<IndexingUpdate>,
    /// Indexing manager for concurrency control
    #[allow(dead_code)]
    indexing_manager: Arc<IndexingManager>,
    /// Query contexts for conversation history
    query_contexts: Arc<RwLock<HashMap<String, QueryContext>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(default_config: SessionConfig, rag_config: RagConfig) -> Self {
        // Create progress broadcaster with a buffer of 100 messages
        let (progress_broadcaster, _) = broadcast::channel::<IndexingUpdate>(100);

        // Create indexing manager with default concurrency limit (2)
        let indexing_manager = Arc::new(IndexingManager::new(2));

        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_config,
            rag_config,
            progress_broadcaster,
            indexing_manager,
            query_contexts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new repository session with permission checking
    pub async fn create_session(
        &self,
        context: &PermissionContext,
        repository: RepositoryInfo,
        options: SessionOptions,
    ) -> ApplicationResult<String> {
        // Check if user has permission to create sessions
        if !context.has_permission(&Permission::Query) {
            return Err(ApplicationError::Permission {
                message: "Insufficient permissions to create session".to_string(),
            });
        }

        // Get user identity
        let owner = context
            .identity
            .clone()
            .unwrap_or_else(crate::auth::UserIdentity::anonymous);

        // Check resource limits
        let current_sessions = self.count_user_sessions(&owner.user_id).await;
        if current_sessions >= context.limits.concurrent_sessions {
            return Err(ApplicationError::Session {
                message: format!(
                    "Maximum concurrent sessions ({}) reached",
                    context.limits.concurrent_sessions
                ),
            });
        }

        // Create session configuration
        let session_config = options
            .config
            .unwrap_or_else(|| self.default_config.clone());

        // Create the session
        let mut session = ApplicationSession::new(owner, repository, session_config);

        // Add any provided metadata
        session.metadata.extend(options.metadata);

        let session_id = session.id.clone();
        info!(
            "Creating new session: {} for repository: {}",
            session_id, session.repository.url
        );

        // Store the session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        // Initialize query context
        {
            let mut contexts = self.query_contexts.write().await;
            contexts.insert(
                session_id.clone(),
                QueryContext::new(
                    session_id.clone(),
                    self.default_config.max_conversation_history,
                ),
            );
        }

        // Start indexing in the background if requested
        if options.auto_generate_wiki {
            self.start_background_indexing(session_id.clone()).await?;
        }

        Ok(session_id)
    }

    /// Get session information with permission checking
    pub async fn get_session(
        &self,
        context: &PermissionContext,
        session_id: &str,
    ) -> ApplicationResult<SessionInfo> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ApplicationError::Session {
                message: format!("Session not found: {}", session_id),
            })?;

        // Check if user can access this session
        if !session.can_access(context) {
            return Err(ApplicationError::Permission {
                message: "Insufficient permissions to access session".to_string(),
            });
        }

        Ok(SessionInfo::from(session))
    }

    /// List sessions accessible to the user
    pub async fn list_sessions(&self, context: &PermissionContext) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;

        sessions
            .values()
            .filter(|session| session.can_access(context))
            .map(SessionInfo::from)
            .collect()
    }

    /// Execute a query with permission checking
    pub async fn query_session(
        &self,
        context: &PermissionContext,
        session_id: &str,
        question: String,
    ) -> ApplicationResult<wikify_rag::RagResponse> {
        // Check query permission
        if !context.has_permission(&Permission::Query) {
            return Err(ApplicationError::Permission {
                message: "Insufficient permissions to query".to_string(),
            });
        }

        // Get and validate session
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| ApplicationError::Session {
                message: format!("Session not found: {}", session_id),
            })?;

        // Check session access
        if !session.can_access(context) {
            return Err(ApplicationError::Permission {
                message: "Insufficient permissions to access session".to_string(),
            });
        }

        // Check if session is ready
        if !session.is_ready() {
            return Err(ApplicationError::Session {
                message: "Session is not ready for queries. Repository may not be indexed yet."
                    .to_string(),
            });
        }

        // Get RAG pipeline
        let rag_pipeline =
            session
                .rag_pipeline
                .as_mut()
                .ok_or_else(|| ApplicationError::Session {
                    message: "RAG pipeline not initialized".to_string(),
                })?;

        // Add user message to context
        {
            let mut contexts = self.query_contexts.write().await;
            if let Some(context) = contexts.get_mut(session_id) {
                context.add_message(QueryMessage::user(question.clone()));
            }
        }

        // Build conversation context
        let conversation_context = self.build_conversation_context(session_id).await;

        // Create RAG query
        let query = wikify_rag::RagQuery {
            question: question.clone(),
            context: Some(conversation_context),
            filters: None,
            retrieval_config: None,
        };

        // Execute query
        let response = rag_pipeline.ask(query).await.map_err(|e| {
            ApplicationError::Core(wikify_core::WikifyError::Rag {
                message: format!("RAG query failed: {}", e),
                source: Some(Box::new(e)),
                context: wikify_core::ErrorContext::new("session_query"),
            })
        })?;

        // Add assistant response to context
        {
            let mut contexts = self.query_contexts.write().await;
            if let Some(context) = contexts.get_mut(session_id) {
                context.add_message(QueryMessage::assistant(response.answer.clone()));
            }
        }

        // Record query in session stats
        session.record_query();

        debug!("Query completed for session: {}", session_id);
        Ok(response)
    }

    /// Remove a session with permission checking
    pub async fn remove_session(
        &self,
        context: &PermissionContext,
        session_id: &str,
    ) -> ApplicationResult<()> {
        // Get session first to check permissions
        let can_access = {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(session_id) {
                session.can_access(context)
            } else {
                false
            }
        };

        if !can_access {
            return Err(ApplicationError::Session {
                message: format!("Session not found: {}", session_id),
            });
        }

        // Check if user can manage this session
        if !context.has_permission(&Permission::ManageSession) {
            return Err(ApplicationError::Permission {
                message: "Insufficient permissions to remove session".to_string(),
            });
        }

        // Remove session and context
        {
            let mut sessions = self.sessions.write().await;
            let mut contexts = self.query_contexts.write().await;

            sessions.remove(session_id);
            contexts.remove(session_id);
        }

        info!("Removed session: {}", session_id);
        Ok(())
    }

    /// Clean up stale sessions
    pub async fn cleanup_stale_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let mut contexts = self.query_contexts.write().await;

        let stale_sessions: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.is_stale())
            .map(|(id, _)| id.clone())
            .collect();

        let count = stale_sessions.len();
        for session_id in stale_sessions {
            sessions.remove(&session_id);
            contexts.remove(&session_id);
            info!("Cleaned up stale session: {}", session_id);
        }

        count
    }

    /// Get progress broadcaster receiver
    pub fn subscribe_to_progress(&self) -> broadcast::Receiver<IndexingUpdate> {
        self.progress_broadcaster.subscribe()
    }

    /// Count sessions for a specific user
    async fn count_user_sessions(&self, user_id: &str) -> u32 {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|session| session.owner.user_id == user_id)
            .count() as u32
    }

    /// Start background indexing for a session
    async fn start_background_indexing(&self, session_id: String) -> ApplicationResult<()> {
        let sessions = self.sessions.clone();
        let progress_broadcaster = self.progress_broadcaster.clone();
        let rag_config = self.rag_config.clone();

        tokio::spawn(async move {
            info!("Starting background indexing for session: {}", session_id);

            // Get repository URL
            let repository_url = {
                let sessions_guard = sessions.read().await;
                if let Some(session) = sessions_guard.get(&session_id) {
                    session.repository.url.clone()
                } else {
                    error!("Session not found for indexing: {}", session_id);
                    return;
                }
            };

            // Send initial progress update
            let _ = progress_broadcaster.send(IndexingUpdate::progress(
                session_id.clone(),
                0.0,
                "Starting repository indexing...".to_string(),
            ));

            // Create and initialize RAG pipeline
            let mut rag_pipeline = RagPipeline::new(rag_config);

            match rag_pipeline.initialize().await {
                Ok(_) => {
                    let _ = progress_broadcaster.send(IndexingUpdate::progress(
                        session_id.clone(),
                        0.2,
                        "RAG pipeline initialized".to_string(),
                    ));
                }
                Err(e) => {
                    error!(
                        "Failed to initialize RAG pipeline for session {}: {}",
                        session_id, e
                    );
                    let _ = progress_broadcaster.send(IndexingUpdate::error(
                        session_id.clone(),
                        format!("Failed to initialize RAG pipeline: {}", e),
                    ));
                    return;
                }
            }

            // Index the repository
            match rag_pipeline.index_repository(&repository_url).await {
                Ok(stats) => {
                    info!(
                        "Repository indexed for session {}: {}",
                        session_id,
                        stats.summary()
                    );

                    // Update session with RAG pipeline and mark as indexed
                    {
                        let mut sessions_guard = sessions.write().await;
                        if let Some(session) = sessions_guard.get_mut(&session_id) {
                            session.rag_pipeline = Some(rag_pipeline);
                            session.set_indexing_progress(1.0);
                        }
                    }

                    let _ = progress_broadcaster.send(IndexingUpdate::complete(
                        session_id,
                        format!("Repository indexed successfully: {}", stats.summary()),
                    ));
                }
                Err(e) => {
                    error!(
                        "Failed to index repository for session {}: {}",
                        session_id, e
                    );
                    let _ = progress_broadcaster.send(IndexingUpdate::error(
                        session_id,
                        format!("Failed to index repository: {}", e),
                    ));
                }
            }
        });

        Ok(())
    }

    /// Build conversation context from query history
    async fn build_conversation_context(&self, session_id: &str) -> String {
        let contexts = self.query_contexts.read().await;

        if let Some(context) = contexts.get(session_id) {
            context.build_context_string(10) // Use last 10 messages for context
        } else {
            String::new()
        }
    }
}
