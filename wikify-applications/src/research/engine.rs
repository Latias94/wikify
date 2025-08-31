//! Main research engine implementation
//! Now based on Repository Manager instead of Session Manager

use super::{
    planner::ResearchPlanner,
    strategy::{AdaptiveResearchStrategy, ResearchStrategySelector},
    synthesizer::ResearchSynthesizer,
    types::*,
};
use crate::{
    repository::RepositoryManager, ApplicationError, ApplicationResult, PermissionContext,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// Deep research engine for complex multi-step investigations
/// Now based on Repository Manager instead of Session Manager
pub struct ResearchEngine {
    /// Research planner for question decomposition
    planner: ResearchPlanner,
    /// Research synthesizer for result compilation
    synthesizer: ResearchSynthesizer,
    /// Strategy selector for adaptive research
    strategy_selector: ResearchStrategySelector,
    /// Repository manager for RAG operations
    repository_manager: Arc<RepositoryManager>,
    /// Active research sessions (now repository-based)
    active_sessions: Arc<RwLock<HashMap<String, ResearchContext>>>,
}

impl ResearchEngine {
    /// Create a new research engine with repository manager
    pub fn new(config: ResearchConfig, repository_manager: Arc<RepositoryManager>) -> Self {
        Self {
            planner: ResearchPlanner::new(config.clone()),
            synthesizer: ResearchSynthesizer::new(config.clone()),
            strategy_selector: ResearchStrategySelector::new(config),
            repository_manager,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new research engine with LLM support
    pub fn with_llm_client(
        config: ResearchConfig,
        repository_manager: Arc<RepositoryManager>,
        _llm_client: Box<dyn siumai::prelude::ChatCapability>,
    ) -> Self {
        // Clone the LLM client for different components
        // Note: In a real implementation, you'd want to use Arc<dyn ChatCapability>
        // For now, we'll create separate instances
        Self {
            planner: ResearchPlanner::new(config.clone()), // TODO: Add LLM support
            synthesizer: ResearchSynthesizer::new(config.clone()), // TODO: Add LLM support
            strategy_selector: ResearchStrategySelector::new(config), // TODO: Add LLM support
            repository_manager,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a new research process
    pub async fn start_research(
        &self,
        repository_id: String,
        topic: String,
        config: ResearchConfig,
    ) -> ApplicationResult<String> {
        let research_session_id = Uuid::new_v4().to_string();

        // Create initial research context
        let research_context = ResearchContext {
            id: research_session_id.clone(),
            repository_id: repository_id.clone(),
            topic: topic.clone(),
            config: config.clone(),
            questions: vec![
                // Create initial research question
                ResearchQuestion {
                    id: Uuid::new_v4(),
                    text: topic,
                    priority: 1.0,
                    answered: false,
                    source: QuestionSource::Initial,
                    created_at: chrono::Utc::now(),
                },
            ],
            findings: Vec::new(),
            iterations: Vec::new(),
            status: ResearchStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Store the research context
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(research_session_id.clone(), research_context);
        }

        info!(
            "Started research session {} for repository {}",
            research_session_id, repository_id
        );
        Ok(research_session_id)
    }

    /// Execute one research iteration using repository-based approach
    pub async fn research_iteration(
        &self,
        context: &PermissionContext,
        repository_id: &str,
        research_session_id: &str,
    ) -> ApplicationResult<ResearchProgress> {
        info!(
            "Executing research iteration for repository: {} session: {}",
            repository_id, research_session_id
        );

        // Get research context
        let research_context = {
            let sessions = self.active_sessions.read().await;
            sessions
                .get(research_session_id)
                .ok_or_else(|| ApplicationError::Research {
                    message: format!("Research session not found: {}", research_session_id),
                })?
                .clone()
        };

        let iteration_start = std::time::Instant::now();
        let iteration_num = research_context.iterations.len() + 1;

        // Select questions to research in this iteration
        let questions_to_research = self
            .select_questions_for_iteration(&research_context)
            .await?;

        if questions_to_research.is_empty() {
            info!(
                "No more questions to research for session: {}",
                research_session_id
            );
            return self
                .finalize_research_internal(context, research_session_id)
                .await;
        }

        let mut iteration_findings = Vec::new();
        let mut new_questions = Vec::new();

        // Research each selected question using Repository Manager
        for question in &questions_to_research {
            info!("Researching question: {}", question.text);

            // Create repository query
            let repo_query = crate::repository::RepositoryQuery {
                question: question.text.clone(),
                max_results: Some(10),
                parameters: None,
            };

            // Use Repository Manager to query
            let rag_response = self
                .repository_manager
                .query_repository(context, repository_id, repo_query)
                .await
                .map_err(|e| ApplicationError::Research {
                    message: format!("Repository query failed: {}", e),
                })?;

            // Convert repository response to research findings
            let findings = self
                .convert_repo_response_to_findings(question, &rag_response)
                .await?;
            iteration_findings.extend(findings);
        }

        // Generate follow-up questions based on findings
        let followup_questions = self
            .planner
            .plan_followup_questions(&research_context, &iteration_findings)
            .await?;
        new_questions.extend(followup_questions);

        // Create partial synthesis
        let all_findings: Vec<_> = research_context
            .findings
            .iter()
            .cloned()
            .chain(iteration_findings.iter().cloned())
            .collect();

        let partial_synthesis = self
            .synthesizer
            .create_partial_synthesis(&research_context.topic, &all_findings)
            .await?;

        // Calculate confidence
        let confidence = self.calculate_iteration_confidence(&iteration_findings);

        // Create iteration result
        let iteration = ResearchIteration {
            iteration: iteration_num,
            questions: questions_to_research,
            findings: iteration_findings.clone(),
            new_questions: new_questions.clone(),
            partial_synthesis,
            confidence,
            needs_more_research: iteration_num < research_context.config.max_iterations
                && !new_questions.is_empty(),
            duration: iteration_start.elapsed(),
        };

        // Update research context
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(context) = sessions.get_mut(research_session_id) {
                // Add new findings
                context.findings.extend(iteration_findings);

                // Add new questions
                context.questions.extend(new_questions);

                // Add iteration
                context.iterations.push(iteration.clone());
            }
        }

        // Return progress
        Ok(ResearchProgress {
            session_id: research_session_id.to_string(),
            repository_id: research_context.repository_id.clone(),
            current_iteration: iteration_num,
            total_iterations: research_context.config.max_iterations,
            stage: format!("Completed iteration {}", iteration_num),
            progress: iteration_num as f64 / research_context.config.max_iterations as f64,
            current_question: None,
            findings_count: research_context.findings.len() + iteration.findings.len(),
            estimated_remaining: Some(std::time::Duration::from_secs(
                (research_context.config.max_iterations - iteration_num) as u64 * 60,
            )),
        })
    }

    /// Get research progress
    pub async fn get_progress(
        &self,
        research_session_id: &str,
    ) -> ApplicationResult<ResearchProgress> {
        let sessions = self.active_sessions.read().await;
        let context =
            sessions
                .get(research_session_id)
                .ok_or_else(|| ApplicationError::Research {
                    message: format!("Research session not found: {}", research_session_id),
                })?;

        let current_iteration = context.iterations.len();
        let progress = if context.config.max_iterations > 0 {
            current_iteration as f64 / context.config.max_iterations as f64
        } else {
            0.0
        };

        Ok(ResearchProgress {
            session_id: research_session_id.to_string(),
            repository_id: context.repository_id.clone(),
            current_iteration,
            total_iterations: context.config.max_iterations,
            stage: if current_iteration == 0 {
                "Planning".to_string()
            } else {
                format!("Iteration {}", current_iteration)
            },
            progress,
            current_question: None,
            findings_count: context.findings.len(),
            estimated_remaining: Some(std::time::Duration::from_secs(
                (context
                    .config
                    .max_iterations
                    .saturating_sub(current_iteration)) as u64
                    * 60,
            )),
        })
    }

    /// List all active research processes
    pub async fn list_active_research(&self) -> Vec<String> {
        let sessions = self.active_sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Get research details
    pub async fn get_research_details(
        &self,
        research_id: &str,
    ) -> ApplicationResult<ResearchContext> {
        let sessions = self.active_sessions.read().await;
        sessions
            .get(research_id)
            .cloned()
            .ok_or_else(|| ApplicationError::Research {
                message: format!("Research not found: {}", research_id),
            })
    }

    /// Cancel research
    pub async fn cancel_research(&self, research_id: &str) -> ApplicationResult<()> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(mut context) = sessions.remove(research_id) {
            context.status = ResearchStatus::Cancelled;
            info!("Cancelled research: {}", research_id);
            Ok(())
        } else {
            Err(ApplicationError::Research {
                message: format!("Research not found: {}", research_id),
            })
        }
    }

    // Helper methods

    /// Select questions for the current iteration
    async fn select_questions_for_iteration(
        &self,
        context: &ResearchContext,
    ) -> ApplicationResult<Vec<ResearchQuestion>> {
        // Get unanswered questions
        let unanswered_questions: Vec<_> = context
            .questions
            .iter()
            .filter(|q| !q.answered)
            .cloned()
            .collect();

        if unanswered_questions.is_empty() {
            return Ok(Vec::new());
        }

        // Sort by priority (f64 doesn't implement Ord, so we use partial_cmp)
        let mut sorted_questions = unanswered_questions;
        sorted_questions.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top questions for this iteration
        let max_questions = context.config.max_sources_per_iteration.min(3);
        sorted_questions.truncate(max_questions);

        Ok(sorted_questions)
    }

    /// Convert repository response to research findings
    async fn convert_repo_response_to_findings(
        &self,
        question: &ResearchQuestion,
        response: &crate::repository::RepositoryQueryResponse,
    ) -> ApplicationResult<Vec<ResearchFinding>> {
        let mut findings = Vec::new();

        // Create a finding from the main answer
        let finding = ResearchFinding {
            id: Uuid::new_v4(),
            question_id: question.id,
            content: response.answer.clone(),
            source: SourceInfo {
                id: response.sources.join(", "),
                source_type: SourceType::SourceCode, // Default to source code
                title: None,
                author: None,
                last_modified: None,
                reliability: response.confidence.unwrap_or(0.5),
            },
            confidence: response.confidence.unwrap_or(0.5),
            timestamp: chrono::Utc::now(),
            relevance: response.confidence.unwrap_or(0.5),
            evidence: response.sources.clone(),
            limitations: Vec::new(),
        };
        findings.push(finding);

        Ok(findings)
    }

    /// Calculate confidence for iteration findings
    fn calculate_iteration_confidence(&self, findings: &[ResearchFinding]) -> f64 {
        if findings.is_empty() {
            return 0.0;
        }

        let total_confidence: f64 = findings.iter().map(|f| f.confidence).sum();
        total_confidence / findings.len() as f64
    }

    /// Finalize research session
    async fn finalize_research_internal(
        &self,
        _context: &PermissionContext,
        research_session_id: &str,
    ) -> ApplicationResult<ResearchProgress> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(context) = sessions.get_mut(research_session_id) {
            context.status = ResearchStatus::Completed;

            let final_synthesis = self
                .synthesizer
                .create_final_synthesis(&context.topic, &context.findings, &context.iterations)
                .await?;

            info!("Research session completed: {}", research_session_id);
            info!("Final synthesis: {}", final_synthesis);

            Ok(ResearchProgress {
                session_id: research_session_id.to_string(),
                repository_id: context.repository_id.clone(),
                current_iteration: context.iterations.len(),
                total_iterations: context.config.max_iterations,
                stage: "Completed".to_string(),
                progress: 1.0,
                current_question: None,
                findings_count: context.findings.len(),
                estimated_remaining: Some(std::time::Duration::from_secs(0)),
            })
        } else {
            Err(ApplicationError::Research {
                message: format!("Research session not found: {}", research_session_id),
            })
        }
    }
}
