//! Simplified Research Engine - Application layer coordination
//!
//! This module provides a simplified research engine that coordinates between
//! the application layer and the RAG layer for deep research functionality.

use super::types::*;
use crate::{
    repository::RepositoryManager, ApplicationError, ApplicationResult, PermissionContext,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Simplified research engine that coordinates deep research sessions
pub struct ResearchEngine {
    /// Repository manager for RAG operations
    repository_manager: Arc<RepositoryManager>,
    /// Active research sessions
    active_sessions: Arc<RwLock<HashMap<String, ResearchSession>>>,
}

/// Research session state
#[derive(Debug, Clone)]
pub struct ResearchSession {
    pub id: String,
    pub repository_id: String,
    pub query: String,
    pub status: ResearchStatus,
    pub config: ResearchConfig,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub result: Option<wikify_rag::DeepResearchResult>,
}

impl ResearchEngine {
    /// Create a new research engine with repository manager
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self {
            repository_manager,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a new deep research process
    pub async fn start_research(
        &self,
        context: &PermissionContext,
        repository_id: &str,
        query: String,
        config: Option<ResearchConfig>,
    ) -> ApplicationResult<String> {
        let session_id = Uuid::new_v4().to_string();
        let research_config = config.unwrap_or_default();

        info!(
            "Starting deep research session {} for repository {}",
            session_id, repository_id
        );

        // Create research session
        let session = ResearchSession {
            id: session_id.clone(),
            repository_id: repository_id.to_string(),
            query: query.clone(),
            status: ResearchStatus::InProgress,
            config: research_config.clone(),
            started_at: Utc::now(),
            completed_at: None,
            result: None,
        };

        // Store the session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        // Start background research task
        let repository_manager = self.repository_manager.clone();
        let sessions = self.active_sessions.clone();
        let session_id_clone = session_id.clone();
        let repository_id_clone = repository_id.to_string();

        tokio::spawn(async move {
            Self::execute_research_background(
                repository_manager,
                sessions,
                session_id_clone,
                repository_id_clone,
                query,
                research_config,
            )
            .await
        });

        Ok(session_id)
    }

    /// Get research progress for a session
    pub async fn get_research_progress(
        &self,
        session_id: &str,
    ) -> ApplicationResult<ResearchProgress> {
        let sessions = self.active_sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ApplicationError::Research {
                message: format!("Research session not found: {}", session_id),
            })?;

        // Convert session to progress
        let progress = if let Some(result) = &session.result {
            // Research is complete
            ResearchProgress {
                id: session.id.clone(),
                status: session.status.clone(),
                current_iteration: result.iterations.len(),
                max_iterations: session.config.max_iterations,
                progress: 1.0,
                current_response: result.iterations.last().map(|i| i.response.clone()),
                estimated_remaining_ms: None,
                last_updated: session.completed_at.unwrap_or(session.started_at),
            }
        } else {
            // Research is in progress
            ResearchProgress {
                id: session.id.clone(),
                status: session.status.clone(),
                current_iteration: 0, // We don't track individual iterations in this simplified version
                max_iterations: session.config.max_iterations,
                progress: 0.5, // Rough estimate
                current_response: None,
                estimated_remaining_ms: None,
                last_updated: chrono::Utc::now(),
            }
        };

        Ok(progress)
    }

    /// Stop a research session
    pub async fn stop_research(&self, session_id: &str) -> ApplicationResult<()> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = ResearchStatus::Cancelled;
            session.completed_at = Some(Utc::now());
            info!("Stopped research session: {}", session_id);
        } else {
            warn!(
                "Attempted to stop non-existent research session: {}",
                session_id
            );
        }
        Ok(())
    }

    /// Execute research in background
    async fn execute_research_background(
        repository_manager: Arc<RepositoryManager>,
        sessions: Arc<RwLock<HashMap<String, ResearchSession>>>,
        session_id: String,
        repository_id: String,
        query: String,
        config: ResearchConfig,
    ) {
        info!("Starting background research for session: {}", session_id);

        // Execute deep research using repository manager
        let rag_result = {
            // Create a dummy permission context for internal operations
            let context = PermissionContext::local(); // Use local context for internal operations

            // For now, we'll use a simple query approach instead of deep research
            // TODO: Implement proper deep research integration with repository manager
            let repo_query = crate::repository::RepositoryQuery {
                question: query.clone(),
                max_results: Some(10),
                parameters: None,
            };

            match repository_manager
                .query_repository(&context, &repository_id, repo_query)
                .await
            {
                Ok(response) => {
                    // Create a simplified deep research result
                    let iteration = wikify_rag::ResearchIteration {
                        iteration: 1,
                        query: query.clone(),
                        response: response.answer.clone(),
                        sources: vec![], // Simplified - we don't have the actual search results
                        duration_ms: 1000, // Placeholder
                        timestamp: Utc::now(),
                        confidence_score: response.confidence.map(|c| c as f32),
                    };

                    let result = wikify_rag::DeepResearchResult {
                        id: session_id.clone(),
                        original_query: query.clone(),
                        iterations: vec![iteration],
                        final_synthesis: response.answer,
                        status: wikify_rag::ResearchStatus::Completed,
                        total_duration_ms: 1000,
                        started_at: Utc::now(),
                        completed_at: Some(Utc::now()),
                        config: wikify_rag::DeepResearchConfig::default(),
                        all_sources: vec![],
                    };

                    Ok(result)
                }
                Err(e) => {
                    warn!("Failed to query repository for research: {}", e);
                    Err(wikify_rag::RagError::Config(format!(
                        "Repository query failed: {}",
                        e
                    )))
                }
            }
        };

        // Update session with result
        let mut sessions_guard = sessions.write().await;
        if let Some(session) = sessions_guard.get_mut(&session_id) {
            match rag_result {
                Ok(result) => {
                    session.status = ResearchStatus::Completed;
                    session.result = Some(result);
                    session.completed_at = Some(Utc::now());
                    info!(
                        "Research completed successfully for session: {}",
                        session_id
                    );
                }
                Err(e) => {
                    session.status = ResearchStatus::Failed(format!("Research failed: {}", e));
                    session.completed_at = Some(Utc::now());
                    warn!("Research failed for session {}: {}", session_id, e);
                }
            }
        }
    }

    /// List all active research sessions
    pub async fn list_active_research(&self) -> Vec<String> {
        let sessions = self.active_sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Get research result (if completed)
    pub async fn get_research_result(
        &self,
        session_id: &str,
    ) -> ApplicationResult<Option<wikify_rag::DeepResearchResult>> {
        let sessions = self.active_sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ApplicationError::Research {
                message: format!("Research session not found: {}", session_id),
            })?;

        Ok(session.result.clone())
    }
}
