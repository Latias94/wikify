//! Main research engine implementation

use super::{
    planner::ResearchPlanner,
    strategy::{AdaptiveResearchStrategy, ResearchStrategySelector},
    synthesizer::ResearchSynthesizer,
    types::*,
};
use crate::{
    ApplicationError, ApplicationResult, PermissionContext, QueryResponse, SessionManager,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// Deep research engine for complex multi-step investigations
pub struct ResearchEngine {
    /// Research planner for question decomposition
    planner: ResearchPlanner,
    /// Research synthesizer for combining findings
    synthesizer: ResearchSynthesizer,
    /// Strategy selector for adaptive research
    strategy_selector: ResearchStrategySelector,
    /// Session manager for RAG operations
    session_manager: Arc<SessionManager>,
    /// Active research sessions
    active_sessions: Arc<RwLock<HashMap<String, ResearchContext>>>,
}

impl ResearchEngine {
    /// Create a new research engine
    pub fn new(config: ResearchConfig, session_manager: Arc<SessionManager>) -> Self {
        Self {
            planner: ResearchPlanner::new(config.clone()),
            synthesizer: ResearchSynthesizer::new(config.clone()),
            strategy_selector: ResearchStrategySelector::new(config),
            session_manager,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new research engine with LLM support
    pub fn with_llm_client(
        config: ResearchConfig,
        session_manager: Arc<SessionManager>,
        _llm_client: Box<dyn siumai::prelude::ChatCapability>,
    ) -> Self {
        // Clone the LLM client for different components
        // Note: In a real implementation, you'd want to use Arc<dyn ChatCapability>
        // For now, we'll create separate instances
        Self {
            planner: ResearchPlanner::new(config.clone()), // TODO: Add LLM support
            synthesizer: ResearchSynthesizer::new(config.clone()), // TODO: Add LLM support
            strategy_selector: ResearchStrategySelector::new(config), // TODO: Add LLM support
            session_manager,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a new research session with intelligent strategy selection
    pub async fn start_research(
        &self,
        _context: &PermissionContext,
        session_id: String,
        topic: String,
        config: ResearchConfig,
    ) -> ApplicationResult<ResearchProgress> {
        info!(
            "Starting research session: {} for topic: {}",
            session_id, topic
        );

        // Create initial research context
        let mut research_context = ResearchContext {
            session_id: session_id.clone(),
            topic: topic.clone(),
            config: config.clone(),
            current_iteration: 0,
            questions: Vec::new(),
            findings: Vec::new(),
            iterations: Vec::new(),
            start_time: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        // Select research strategy based on the topic
        let strategy = self
            .strategy_selector
            .select_strategy(&topic, &research_context)
            .await?;
        info!("Selected research strategy: {:?}", strategy);

        // Plan initial research questions based on strategy
        let initial_questions = match &strategy {
            AdaptiveResearchStrategy::QuickScan { .. } => {
                // Generate fewer, more focused questions for quick scan
                let mut questions = self.planner.plan_initial_research(&topic).await?;
                questions.truncate(3); // Limit to 3 questions for quick scan
                questions
            }
            AdaptiveResearchStrategy::DeepDive { focus_areas, .. } => {
                // Generate comprehensive questions for deep dive
                let mut all_questions = Vec::new();
                for area in focus_areas {
                    let area_questions = self.planner.plan_initial_research(area).await?;
                    all_questions.extend(area_questions);
                }
                all_questions
            }
            AdaptiveResearchStrategy::Comparative {
                subjects,
                comparison_aspects,
            } => {
                // Generate comparison-focused questions
                let mut questions = Vec::new();
                for aspect in comparison_aspects {
                    for subject in subjects {
                        let question_text = format!("How does {} handle {}?", subject, aspect);
                        questions.push(ResearchQuestion {
                            id: Uuid::new_v4(),
                            text: question_text,
                            question_type: QuestionType::Comparative,
                            priority: 7,
                            complexity: 5,
                            keywords: vec![subject.clone(), aspect.clone()],
                            parent_id: None,
                            depth: 0,
                        });
                    }
                }
                questions
            }
            _ => self.planner.plan_initial_research(&topic).await?,
        };

        // Update context with initial questions
        research_context.questions = initial_questions;

        // Store the context
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_id.clone(), research_context.clone());
        }

        // Return initial progress
        Ok(ResearchProgress {
            session_id,
            current_iteration: 0,
            total_iterations: match &strategy {
                AdaptiveResearchStrategy::QuickScan { max_iterations, .. } => *max_iterations,
                AdaptiveResearchStrategy::DeepDive { max_iterations, .. } => *max_iterations,
                _ => config.max_iterations,
            },
            stage: format!("Planning initial research using {:?} strategy", strategy),
            progress: 0.1,
            current_question: research_context.questions.first().map(|q| q.text.clone()),
            findings_count: 0,
            estimated_remaining: Some(std::time::Duration::from_secs(300)), // 5 minutes estimate
        })
    }

    /// Execute one research iteration
    pub async fn research_iteration(
        &self,
        context: &PermissionContext,
        session_id: &str,
    ) -> ApplicationResult<ResearchProgress> {
        info!("Executing research iteration for session: {}", session_id);

        let research_context = {
            let sessions = self.active_sessions.read().await;
            sessions
                .get(session_id)
                .ok_or_else(|| ApplicationError::Research {
                    message: format!("Research session not found: {}", session_id),
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
            info!("No more questions to research for session: {}", session_id);
            return self.finalize_research_internal(context, session_id).await;
        }

        let mut iteration_findings = Vec::new();
        let mut new_questions = Vec::new();

        // Research each selected question
        for question in &questions_to_research {
            info!("Researching question: {}", question.text);

            // Use RAG to find relevant information
            let rag_response = self
                .session_manager
                .query_session(context, session_id, question.text.clone())
                .await
                .map_err(|e| ApplicationError::Research {
                    message: format!("RAG query failed: {}", e),
                })?;

            // Convert RAG response to research findings
            let sources: Vec<String> = rag_response
                .sources
                .iter()
                .map(|s| s.chunk.content.clone())
                .collect();
            let query_response = QueryResponse {
                answer: rag_response.answer,
                sources,
                metadata: std::collections::HashMap::new(),
            };
            let findings = self
                .convert_rag_to_findings(question, &query_response)
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
            if let Some(context) = sessions.get_mut(session_id) {
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
            session_id: session_id.to_string(),
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
    pub async fn get_progress(&self, session_id: &str) -> ApplicationResult<ResearchProgress> {
        let sessions = self.active_sessions.read().await;
        let context = sessions
            .get(session_id)
            .ok_or_else(|| ApplicationError::Research {
                message: format!("Research session not found: {}", session_id),
            })?;

        let current_iteration = context.iterations.len();
        let progress = if context.config.max_iterations > 0 {
            current_iteration as f64 / context.config.max_iterations as f64
        } else {
            0.0
        };

        Ok(ResearchProgress {
            session_id: session_id.to_string(),
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

    /// Select questions to research in the current iteration
    async fn select_questions_for_iteration(
        &self,
        context: &ResearchContext,
    ) -> ApplicationResult<Vec<ResearchQuestion>> {
        let mut unanswered_questions: Vec<_> = context
            .questions
            .iter()
            .filter(|q| !context.findings.iter().any(|f| f.question_id == q.id))
            .cloned()
            .collect();

        // Sort by priority
        unanswered_questions.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Limit to max sources per iteration
        unanswered_questions.truncate(context.config.max_sources_per_iteration);

        Ok(unanswered_questions)
    }

    /// Convert RAG response to research findings
    async fn convert_rag_to_findings(
        &self,
        question: &ResearchQuestion,
        rag_response: &crate::QueryResponse,
    ) -> ApplicationResult<Vec<ResearchFinding>> {
        let mut findings = Vec::new();

        // Create main finding from the answer
        let main_finding = ResearchFinding {
            id: Uuid::new_v4(),
            question_id: question.id,
            source: SourceInfo {
                id: "rag_synthesis".to_string(),
                source_type: SourceType::External,
                title: Some("RAG Synthesis".to_string()),
                author: None,
                last_modified: None,
                reliability: 0.8,
            },
            content: rag_response.answer.clone(),
            confidence: 0.8, // Default confidence
            relevance: 0.9,  // High relevance since it's directly answering the question
            evidence: rag_response.sources.clone(),
            limitations: Vec::new(),
            timestamp: chrono::Utc::now(),
        };
        findings.push(main_finding);

        Ok(findings)
    }

    /// Internal method to finalize research
    async fn finalize_research_internal(
        &self,
        _context: &PermissionContext,
        session_id: &str,
    ) -> ApplicationResult<ResearchProgress> {
        info!("Finalizing research for session: {}", session_id);

        let research_context = {
            let sessions = self.active_sessions.read().await;
            sessions
                .get(session_id)
                .ok_or_else(|| crate::ApplicationError::Research {
                    message: format!("Research session not found: {}", session_id),
                })?
                .clone()
        };

        // Clean up session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(session_id);
        }

        Ok(ResearchProgress {
            session_id: session_id.to_string(),
            current_iteration: research_context.iterations.len(),
            total_iterations: research_context.config.max_iterations,
            stage: "Research completed".to_string(),
            progress: 1.0,
            current_question: None,
            findings_count: research_context.findings.len(),
            estimated_remaining: None,
        })
    }

    /// Calculate confidence for an iteration based on findings
    fn calculate_iteration_confidence(&self, findings: &[ResearchFinding]) -> f64 {
        if findings.is_empty() {
            return 0.0;
        }

        let total_confidence: f64 = findings.iter().map(|f| f.confidence).sum();
        total_confidence / findings.len() as f64
    }
}
