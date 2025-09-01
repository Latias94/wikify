//! Research functionality related types

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use wikify_applications::{ResearchCategory, ResearchProgress, ResearchTemplate};

/// Research start request
#[derive(Deserialize, ToSchema)]
pub struct StartResearchRequest {
    #[schema(example = "repo-uuid-string")]
    pub repository_id: String,
    #[schema(example = "Deep analysis of authentication system")]
    pub research_question: String,
    pub config: Option<ResearchConfigRequest>,
}

/// Research configuration request
#[derive(Deserialize, ToSchema)]
pub struct ResearchConfigRequest {
    #[schema(example = 5)]
    pub max_iterations: Option<usize>,
    #[schema(example = 10)]
    pub max_sources_per_iteration: Option<usize>,
    #[schema(example = true)]
    pub include_code_analysis: Option<bool>,
    #[schema(example = true)]
    pub generate_diagrams: Option<bool>,
}

/// Research start response
#[derive(Serialize, ToSchema)]
pub struct StartResearchResponse {
    #[schema(example = "research-uuid-string")]
    pub research_id: String,
    #[schema(example = "started")]
    pub status: String,
    #[schema(example = "Research session started successfully")]
    pub message: String,
}

/// Research progress response
#[derive(Serialize, ToSchema)]
pub struct ResearchProgressResponse {
    #[schema(example = "research-uuid-string")]
    pub research_id: String,
    #[schema(example = "in_progress")]
    pub status: String,
    #[schema(example = 3)]
    pub current_iteration: usize,
    #[schema(example = 5)]
    pub total_iterations: usize,
    #[schema(example = "Analyzing authentication patterns")]
    pub current_focus: String,
    pub findings: Vec<String>,
    #[schema(example = 0.6)]
    pub progress_percentage: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl From<ResearchProgress> for ResearchProgressResponse {
    fn from(progress: ResearchProgress) -> Self {
        Self {
            research_id: progress.session_id,
            status: "in_progress".to_string(), // TODO: Add status field to ResearchProgress
            current_iteration: progress.current_iteration,
            total_iterations: progress.total_iterations,
            current_focus: progress.stage, // Use stage as current_focus
            findings: vec![],              // TODO: Add findings field to ResearchProgress
            progress_percentage: progress.progress * 100.0, // Convert to percentage
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Research template response
#[derive(Serialize, ToSchema)]
pub struct ResearchTemplateResponse {
    #[schema(example = "template-uuid")]
    pub id: String,
    #[schema(example = "Security Analysis")]
    pub name: String,
    #[schema(example = "Comprehensive security analysis template")]
    pub description: String,
    pub category: ResearchCategory,
    pub questions: Vec<String>,
    pub config: serde_json::Value,
}

impl From<ResearchTemplate> for ResearchTemplateResponse {
    fn from(template: ResearchTemplate) -> Self {
        Self {
            id: template.id,
            name: template.name,
            description: template.description,
            category: template.category,
            questions: template
                .initial_questions
                .into_iter()
                .map(|q| q.text)
                .collect(),
            config: serde_json::to_value(template.config).unwrap_or_default(),
        }
    }
}

/// Start research from template request
#[derive(Deserialize, ToSchema)]
pub struct StartResearchFromTemplateRequest {
    #[schema(example = "repo-uuid-string")]
    pub repository_id: String,
    #[schema(example = "template-uuid")]
    pub template_id: String,
    pub custom_questions: Option<Vec<String>>,
    pub config_overrides: Option<serde_json::Value>,
}
