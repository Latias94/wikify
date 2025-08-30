//! Research templates for different types of investigations

use super::types::{QuestionType, ResearchConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Simplified template question for initial research
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct TemplateQuestion {
    /// Question text
    pub text: String,
    /// Question type
    pub question_type: QuestionType,
    /// Priority level (1-10)
    pub priority: u8,
    /// Expected complexity (1-10)
    pub complexity: u8,
    /// Keywords for this question
    pub keywords: Vec<String>,
}

/// Research template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchTemplate {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Template category
    pub category: ResearchCategory,
    /// Research configuration
    pub config: ResearchConfig,
    /// Initial research questions
    pub initial_questions: Vec<TemplateQuestion>,
    /// Template-specific parameters
    pub parameters: HashMap<String, TemplateParameter>,
}

/// Research template category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum ResearchCategory {
    /// Technical analysis and code review
    Technical,
    /// Architecture and design analysis
    Architecture,
    /// Security assessment
    Security,
    /// Performance analysis
    Performance,
    /// Documentation and knowledge extraction
    Documentation,
    /// Business and product analysis
    Business,
    /// Custom research
    Custom,
}

/// Template parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct TemplateParameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Parameter type
    pub param_type: ParameterType,
    /// Default value
    pub default_value: Option<String>,
    /// Whether parameter is required
    pub required: bool,
}

/// Parameter type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum ParameterType {
    String,
    Integer,
    Boolean,
    Choice(Vec<String>),
}

/// Research template manager
#[derive(Debug, Clone)]
pub struct ResearchTemplateManager {
    templates: HashMap<String, ResearchTemplate>,
}

impl Default for ResearchTemplateManager {
    fn default() -> Self {
        let mut manager = Self {
            templates: HashMap::new(),
        };

        // Load built-in templates
        manager.load_builtin_templates();
        manager
    }
}

impl ResearchTemplateManager {
    /// Create new template manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Load built-in research templates
    fn load_builtin_templates(&mut self) {
        // Technical Analysis Template
        self.add_template(ResearchTemplate {
            id: "technical-analysis".to_string(),
            name: "Technical Analysis".to_string(),
            description: "Comprehensive technical analysis of codebase including architecture, patterns, and best practices".to_string(),
            category: ResearchCategory::Technical,
            config: ResearchConfig {
                max_iterations: 5,
                max_depth: 3,
                confidence_threshold: 0.7,
                max_sources_per_iteration: 10,
                enable_parallel_research: true,
            },
            initial_questions: vec![
                TemplateQuestion {
                    text: "What is the overall architecture and main components of this codebase?".to_string(),
                    question_type: QuestionType::Architectural,
                    priority: 9,
                    complexity: 7,
                    keywords: vec!["architecture".to_string(), "components".to_string(), "structure".to_string()],
                },
                TemplateQuestion {
                    text: "What technologies, frameworks, and libraries are being used?".to_string(),
                    question_type: QuestionType::Technical,
                    priority: 8,
                    complexity: 5,
                    keywords: vec!["technologies".to_string(), "frameworks".to_string(), "libraries".to_string()],
                },
                TemplateQuestion {
                    text: "What design patterns and architectural patterns are implemented?".to_string(),
                    question_type: QuestionType::Architectural,
                    priority: 7,
                    complexity: 8,
                    keywords: vec!["patterns".to_string(), "design".to_string(), "architecture".to_string()],
                },
            ],
            parameters: HashMap::from([
                ("focus_language".to_string(), TemplateParameter {
                    name: "Focus Language".to_string(),
                    description: "Primary programming language to focus analysis on".to_string(),
                    param_type: ParameterType::Choice(vec![
                        "rust".to_string(),
                        "python".to_string(),
                        "javascript".to_string(),
                        "java".to_string(),
                        "go".to_string(),
                        "any".to_string(),
                    ]),
                    default_value: Some("any".to_string()),
                    required: false,
                }),
            ]),
        });

        // Architecture Assessment Template
        self.add_template(ResearchTemplate {
            id: "architecture-assessment".to_string(),
            name: "Architecture Assessment".to_string(),
            description: "Deep dive into system architecture, scalability, and design decisions"
                .to_string(),
            category: ResearchCategory::Architecture,
            config: ResearchConfig {
                max_iterations: 4,
                max_depth: 2,
                confidence_threshold: 0.8,
                max_sources_per_iteration: 8,
                enable_parallel_research: false,
            },
            initial_questions: vec![
                TemplateQuestion {
                    text: "What are the main system components and how do they interact?"
                        .to_string(),
                    question_type: QuestionType::Architectural,
                    priority: 9,
                    complexity: 8,
                    keywords: vec![
                        "components".to_string(),
                        "interaction".to_string(),
                        "system".to_string(),
                    ],
                },
                TemplateQuestion {
                    text: "How does data flow through the system?".to_string(),
                    question_type: QuestionType::Technical,
                    priority: 8,
                    complexity: 7,
                    keywords: vec![
                        "data".to_string(),
                        "flow".to_string(),
                        "processing".to_string(),
                    ],
                },
            ],
            parameters: HashMap::new(),
        });

        // Security Analysis Template
        self.add_template(ResearchTemplate {
            id: "security-analysis".to_string(),
            name: "Security Analysis".to_string(),
            description: "Security-focused analysis including vulnerability assessment and security best practices".to_string(),
            category: ResearchCategory::Security,
            config: ResearchConfig::default(),
            initial_questions: vec![
                TemplateQuestion {
                    text: "What authentication and authorization mechanisms are implemented?".to_string(),
                    question_type: QuestionType::Technical,
                    priority: 9,
                    complexity: 8,
                    keywords: vec!["authentication".to_string(), "authorization".to_string(), "security".to_string()],
                },
            ],
            parameters: HashMap::new(),
        });

        // Documentation Extraction Template
        self.add_template(ResearchTemplate {
            id: "documentation-extraction".to_string(),
            name: "Documentation Extraction".to_string(),
            description: "Extract and organize knowledge from codebase for documentation purposes"
                .to_string(),
            category: ResearchCategory::Documentation,
            config: ResearchConfig::default(),
            initial_questions: vec![TemplateQuestion {
                text: "What is the main functionality and purpose of this codebase?".to_string(),
                question_type: QuestionType::Conceptual,
                priority: 10,
                complexity: 6,
                keywords: vec![
                    "functionality".to_string(),
                    "purpose".to_string(),
                    "overview".to_string(),
                ],
            }],
            parameters: HashMap::new(),
        });
    }

    /// Add a research template
    pub fn add_template(&mut self, template: ResearchTemplate) {
        self.templates.insert(template.id.clone(), template);
    }

    /// Get template by ID
    pub fn get_template(&self, template_id: &str) -> Option<&ResearchTemplate> {
        self.templates.get(template_id)
    }

    /// List all templates
    pub fn list_templates(&self) -> Vec<&ResearchTemplate> {
        self.templates.values().collect()
    }

    /// List templates by category
    pub fn list_templates_by_category(
        &self,
        category: &ResearchCategory,
    ) -> Vec<&ResearchTemplate> {
        self.templates
            .values()
            .filter(|t| &t.category == category)
            .collect()
    }

    /// Create research config from template with parameters
    pub fn create_config_from_template(
        &self,
        template_id: &str,
        _parameters: HashMap<String, String>,
    ) -> Option<(ResearchConfig, Vec<TemplateQuestion>)> {
        let template = self.get_template(template_id)?;

        // TODO: Apply parameters to customize the template
        // For now, return the template as-is
        Some((template.config.clone(), template.initial_questions.clone()))
    }
}
