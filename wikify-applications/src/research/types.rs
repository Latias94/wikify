//! Types for the deep research system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Research session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ResearchConfig {
    /// Maximum number of research iterations
    pub max_iterations: usize,
    /// Maximum depth for sub-question decomposition
    pub max_depth: usize,
    /// Minimum confidence threshold for accepting findings
    pub confidence_threshold: f64,
    /// Maximum number of sources to consider per iteration
    pub max_sources_per_iteration: usize,
    /// Whether to enable parallel research paths
    pub enable_parallel_research: bool,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            max_iterations: 5,
            max_depth: 3,
            confidence_threshold: 0.7,
            max_sources_per_iteration: 10,
            enable_parallel_research: true,
        }
    }
}

/// Research question with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchQuestion {
    /// Unique identifier for the question
    pub id: Uuid,
    /// The question text
    pub text: String,
    /// Question type/category
    pub question_type: QuestionType,
    /// Priority level (1-10, higher is more important)
    pub priority: u8,
    /// Parent question ID (for sub-questions)
    pub parent_id: Option<Uuid>,
    /// Depth in the question hierarchy
    pub depth: usize,
    /// Expected complexity (1-10)
    pub complexity: u8,
    /// Keywords associated with this question
    pub keywords: Vec<String>,
}

/// Types of research questions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum QuestionType {
    /// High-level conceptual question
    Conceptual,
    /// Technical implementation question
    Technical,
    /// Architecture or design question
    Architectural,
    /// Historical or evolutionary question
    Historical,
    /// Comparative analysis question
    Comparative,
    /// Troubleshooting or debugging question
    Diagnostic,
    /// Best practices or recommendations
    Advisory,
}

impl std::fmt::Display for QuestionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuestionType::Conceptual => write!(f, "Conceptual"),
            QuestionType::Technical => write!(f, "Technical"),
            QuestionType::Architectural => write!(f, "Architectural"),
            QuestionType::Historical => write!(f, "Historical"),
            QuestionType::Comparative => write!(f, "Comparative"),
            QuestionType::Diagnostic => write!(f, "Diagnostic"),
            QuestionType::Advisory => write!(f, "Advisory"),
        }
    }
}

/// Research finding from a single source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchFinding {
    /// Unique identifier for the finding
    pub id: Uuid,
    /// Question this finding addresses
    pub question_id: Uuid,
    /// Source information
    pub source: SourceInfo,
    /// The actual finding content
    pub content: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Relevance score (0.0-1.0)
    pub relevance: f64,
    /// Supporting evidence
    pub evidence: Vec<String>,
    /// Contradictions or limitations
    pub limitations: Vec<String>,
    /// Timestamp when finding was discovered
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Information about a research source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SourceInfo {
    /// Source identifier (file path, URL, etc.)
    pub id: String,
    /// Source type
    pub source_type: SourceType,
    /// Source title or name
    pub title: Option<String>,
    /// Author or maintainer
    pub author: Option<String>,
    /// Last modified date
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    /// Source reliability score (0.0-1.0)
    pub reliability: f64,
}

/// Types of research sources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum SourceType {
    /// Source code file
    SourceCode,
    /// Documentation file
    Documentation,
    /// Configuration file
    Configuration,
    /// Test file
    Test,
    /// README or similar
    Readme,
    /// Comment or inline documentation
    Comment,
    /// External reference
    External,
}

/// Research iteration result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchIteration {
    /// Iteration number (1-based)
    pub iteration: usize,
    /// Questions explored in this iteration
    pub questions: Vec<ResearchQuestion>,
    /// Findings discovered
    pub findings: Vec<ResearchFinding>,
    /// New questions generated
    pub new_questions: Vec<ResearchQuestion>,
    /// Synthesis of findings so far
    pub partial_synthesis: String,
    /// Confidence in current understanding
    pub confidence: f64,
    /// Whether more research is needed
    pub needs_more_research: bool,
    /// Duration of this iteration
    pub duration: std::time::Duration,
}

/// Complete research session result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    /// Research session ID
    pub session_id: String,
    /// Original research topic/question
    pub topic: String,
    /// Research configuration used
    pub config: ResearchConfig,
    /// All iterations performed
    pub iterations: Vec<ResearchIteration>,
    /// Final synthesized report
    pub final_report: String,
    /// Executive summary
    pub summary: String,
    /// Key findings
    pub key_findings: Vec<String>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Areas needing further research
    pub further_research: Vec<String>,
    /// Overall confidence in results
    pub overall_confidence: f64,
    /// Total research time
    pub total_duration: std::time::Duration,
    /// Research quality metrics
    pub metrics: ResearchMetrics,
}

/// Metrics for evaluating research quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchMetrics {
    /// Number of sources consulted
    pub sources_consulted: usize,
    /// Number of questions explored
    pub questions_explored: usize,
    /// Number of findings discovered
    pub findings_discovered: usize,
    /// Average confidence across findings
    pub average_confidence: f64,
    /// Coverage score (how well the topic was covered)
    pub coverage_score: f64,
    /// Depth score (how deep the research went)
    pub depth_score: f64,
    /// Coherence score (how well findings fit together)
    pub coherence_score: f64,
}

/// Research summary containing final results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchSummary {
    /// Research session ID
    pub session_id: String,
    /// Research topic
    pub topic: String,
    /// Executive summary
    pub executive_summary: String,
    /// Key findings
    pub key_findings: Vec<String>,
    /// Detailed analysis
    pub detailed_analysis: String,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Sources consulted
    pub sources: Vec<String>,
    /// Research quality metrics
    pub quality_metrics: ResearchQualityMetrics,
    /// Generation timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Research quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchQualityMetrics {
    /// Completeness score (0.0 to 1.0)
    pub completeness_score: f64,
    /// Accuracy confidence (0.0 to 1.0)
    pub accuracy_confidence: f64,
    /// Source reliability score (0.0 to 1.0)
    pub source_reliability: f64,
    /// Coverage breadth (0.0 to 1.0)
    pub coverage_breadth: f64,
}

/// Research progress update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchProgress {
    /// Session ID
    pub session_id: String,
    /// Current iteration
    pub current_iteration: usize,
    /// Total planned iterations
    pub total_iterations: usize,
    /// Current stage description
    pub stage: String,
    /// Progress percentage (0.0-1.0)
    pub progress: f64,
    /// Current question being researched
    pub current_question: Option<String>,
    /// Number of findings so far
    pub findings_count: usize,
    /// Estimated time remaining
    pub estimated_remaining: Option<std::time::Duration>,
}

/// Research planning strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum ResearchStrategy {
    /// Breadth-first: explore many aspects shallowly first
    BreadthFirst,
    /// Depth-first: dive deep into specific areas
    DepthFirst,
    /// Priority-based: focus on high-priority questions first
    PriorityBased,
    /// Adaptive: adjust strategy based on findings
    Adaptive,
}

/// Research context for maintaining state across iterations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchContext {
    /// Session ID
    pub session_id: String,
    /// Original topic
    pub topic: String,
    /// Configuration
    pub config: ResearchConfig,
    /// Current iteration number
    pub current_iteration: usize,
    /// All questions (as a vector for ordered processing)
    pub questions: Vec<ResearchQuestion>,
    /// All findings (as a vector for chronological order)
    pub findings: Vec<ResearchFinding>,
    /// Research history
    pub iterations: Vec<ResearchIteration>,
    /// Start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}
