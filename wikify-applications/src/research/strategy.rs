//! Research strategy selection and adaptation

use super::types::*;
use crate::ApplicationResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Different research strategies for different types of questions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AdaptiveResearchStrategy {
    /// Quick overview for simple questions
    QuickScan {
        max_iterations: usize,
        depth_level: u8,
    },
    /// Deep dive for complex technical questions
    DeepDive {
        max_iterations: usize,
        focus_areas: Vec<String>,
    },
    /// Comparative analysis for "how does X compare to Y" questions
    Comparative {
        subjects: Vec<String>,
        comparison_aspects: Vec<String>,
    },
    /// Architectural analysis for system design questions
    Architectural {
        components: Vec<String>,
        interaction_depth: u8,
    },
    /// Historical analysis for evolution/change questions
    Historical {
        time_periods: Vec<String>,
        change_aspects: Vec<String>,
    },
}

/// Strategy selector that chooses the best approach for a given question
pub struct ResearchStrategySelector {
    /// Configuration for strategy selection
    #[allow(dead_code)]
    config: ResearchConfig,
    /// LLM client for strategy analysis
    llm_client: Option<Box<dyn siumai::prelude::ChatCapability>>,
}

impl ResearchStrategySelector {
    /// Create a new strategy selector
    pub fn new(config: ResearchConfig) -> Self {
        Self {
            config,
            llm_client: None,
        }
    }

    /// Create strategy selector with LLM support
    pub fn with_llm_client(
        config: ResearchConfig,
        llm_client: Box<dyn siumai::prelude::ChatCapability>,
    ) -> Self {
        Self {
            config,
            llm_client: Some(llm_client),
        }
    }

    /// Analyze a research topic and select the best strategy
    pub async fn select_strategy(
        &self,
        topic: &str,
        context: &ResearchContext,
    ) -> ApplicationResult<AdaptiveResearchStrategy> {
        if let Some(ref llm_client) = self.llm_client {
            self.select_strategy_with_llm(topic, context, llm_client)
                .await
        } else {
            self.select_strategy_heuristic(topic, context).await
        }
    }

    /// Use LLM to intelligently select research strategy
    async fn select_strategy_with_llm(
        &self,
        topic: &str,
        _context: &ResearchContext,
        llm_client: &Box<dyn siumai::prelude::ChatCapability>,
    ) -> ApplicationResult<AdaptiveResearchStrategy> {
        let prompt = format!(
            r#"Analyze this research topic and determine the best research strategy.

Topic: "{}"
Context: Repository analysis for code understanding

Available strategies:
1. QuickScan - For simple, direct questions (1-2 iterations)
2. DeepDive - For complex technical questions (3-5 iterations)  
3. Comparative - For comparison questions ("X vs Y")
4. Architectural - For system design questions
5. Historical - For evolution/change questions

Respond with JSON:
{{
    "strategy": "QuickScan|DeepDive|Comparative|Architectural|Historical",
    "reasoning": "Why this strategy is best",
    "parameters": {{
        "max_iterations": 3,
        "focus_areas": ["area1", "area2"],
        "depth_level": 2
    }}
}}

Consider:
- Question complexity and scope
- Whether it asks for comparison
- Whether it's about system architecture
- Whether it's about changes over time
- How deep the analysis needs to be"#,
            topic
        );

        let messages = vec![siumai::prelude::ChatMessage::user(prompt).build()];

        let response = llm_client
            .chat_with_tools(messages, None)
            .await
            .map_err(|e| crate::ApplicationError::Research {
                message: format!("Failed to select strategy with LLM: {}", e),
            })?;

        let content = response.content_text().unwrap_or_default();

        // Parse LLM response and create strategy
        self.parse_strategy_response(content, topic).await
    }

    /// Parse LLM response into a research strategy
    async fn parse_strategy_response(
        &self,
        response: &str,
        topic: &str,
    ) -> ApplicationResult<AdaptiveResearchStrategy> {
        // Try to parse JSON response
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response) {
            let strategy_type = parsed["strategy"].as_str().unwrap_or("DeepDive");
            let params = &parsed["parameters"];

            match strategy_type {
                "QuickScan" => Ok(AdaptiveResearchStrategy::QuickScan {
                    max_iterations: params["max_iterations"].as_u64().unwrap_or(2) as usize,
                    depth_level: params["depth_level"].as_u64().unwrap_or(1) as u8,
                }),
                "DeepDive" => {
                    let focus_areas = params["focus_areas"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_else(|| vec![topic.to_string()]);

                    Ok(AdaptiveResearchStrategy::DeepDive {
                        max_iterations: params["max_iterations"].as_u64().unwrap_or(4) as usize,
                        focus_areas,
                    })
                }
                "Comparative" => {
                    let subjects = self.extract_comparison_subjects(topic);
                    Ok(AdaptiveResearchStrategy::Comparative {
                        subjects,
                        comparison_aspects: vec![
                            "functionality".to_string(),
                            "performance".to_string(),
                            "design".to_string(),
                        ],
                    })
                }
                "Architectural" => {
                    let components = params["components"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_else(|| vec!["core".to_string(), "interfaces".to_string()]);

                    Ok(AdaptiveResearchStrategy::Architectural {
                        components,
                        interaction_depth: params["depth_level"].as_u64().unwrap_or(2) as u8,
                    })
                }
                "Historical" => Ok(AdaptiveResearchStrategy::Historical {
                    time_periods: vec!["recent".to_string(), "historical".to_string()],
                    change_aspects: vec!["features".to_string(), "architecture".to_string()],
                }),
                _ => Ok(AdaptiveResearchStrategy::DeepDive {
                    max_iterations: 3,
                    focus_areas: vec![topic.to_string()],
                }),
            }
        } else {
            // Fallback to heuristic if JSON parsing fails
            info!(
                "Failed to parse LLM strategy response: {}, falling back to heuristic",
                response
            );
            self.select_strategy_heuristic(topic, &ResearchContext::default())
                .await
        }
    }

    /// Heuristic-based strategy selection (fallback)
    async fn select_strategy_heuristic(
        &self,
        topic: &str,
        _context: &ResearchContext,
    ) -> ApplicationResult<AdaptiveResearchStrategy> {
        let topic_lower = topic.to_lowercase();

        // Check for comparison keywords
        if topic_lower.contains(" vs ")
            || topic_lower.contains(" versus ")
            || topic_lower.contains("compare")
            || topic_lower.contains("difference")
        {
            let subjects = self.extract_comparison_subjects(topic);
            return Ok(AdaptiveResearchStrategy::Comparative {
                subjects,
                comparison_aspects: vec!["functionality".to_string(), "performance".to_string()],
            });
        }

        // Check for architectural keywords
        if topic_lower.contains("architecture")
            || topic_lower.contains("design")
            || topic_lower.contains("structure")
            || topic_lower.contains("components")
        {
            return Ok(AdaptiveResearchStrategy::Architectural {
                components: vec!["core".to_string(), "modules".to_string()],
                interaction_depth: 2,
            });
        }

        // Check for historical keywords
        if topic_lower.contains("history")
            || topic_lower.contains("evolution")
            || topic_lower.contains("changes")
            || topic_lower.contains("development")
        {
            return Ok(AdaptiveResearchStrategy::Historical {
                time_periods: vec!["recent".to_string(), "historical".to_string()],
                change_aspects: vec!["features".to_string(), "implementation".to_string()],
            });
        }

        // Check for simple questions
        if topic_lower.starts_with("what is")
            || topic_lower.starts_with("how to")
            || topic.len() < 50
        {
            return Ok(AdaptiveResearchStrategy::QuickScan {
                max_iterations: 2,
                depth_level: 1,
            });
        }

        // Default to deep dive for complex questions
        Ok(AdaptiveResearchStrategy::DeepDive {
            max_iterations: 4,
            focus_areas: vec![topic.to_string()],
        })
    }

    /// Extract subjects for comparison from topic
    fn extract_comparison_subjects(&self, topic: &str) -> Vec<String> {
        // Simple extraction logic - can be enhanced with NLP
        if let Some(vs_pos) = topic.find(" vs ") {
            let before = topic[..vs_pos].trim();
            let after = topic[vs_pos + 4..].trim();
            vec![before.to_string(), after.to_string()]
        } else if let Some(versus_pos) = topic.find(" versus ") {
            let before = topic[..versus_pos].trim();
            let after = topic[versus_pos + 8..].trim();
            vec![before.to_string(), after.to_string()]
        } else if topic.to_lowercase().contains("compare") {
            // Try to extract subjects from "Compare X and Y" or "Compare X with Y"
            let words: Vec<&str> = topic.split_whitespace().collect();
            if let Some(compare_idx) = words
                .iter()
                .position(|&w| w.to_lowercase().starts_with("compare"))
            {
                let remaining: Vec<String> = words[compare_idx + 1..]
                    .iter()
                    .filter(|&&w| {
                        !["and", "with", "to", "against"].contains(&w.to_lowercase().as_str())
                    })
                    .map(|s| s.to_string())
                    .collect();
                if remaining.len() >= 2 {
                    return remaining;
                }
            }
            // Fallback for comparison topics
            vec!["option1".to_string(), "option2".to_string()]
        } else {
            // Extract potential subjects from comparison-related topics
            vec!["subject1".to_string(), "subject2".to_string()]
        }
    }

    /// Adapt strategy based on intermediate results
    pub async fn adapt_strategy(
        &self,
        current_strategy: &AdaptiveResearchStrategy,
        findings: &[ResearchFinding],
        iteration: usize,
    ) -> ApplicationResult<AdaptiveResearchStrategy> {
        debug!(
            "Adapting strategy based on {} findings at iteration {}",
            findings.len(),
            iteration
        );

        match current_strategy {
            AdaptiveResearchStrategy::DeepDive {
                max_iterations,
                focus_areas,
            } => {
                // If we have high-confidence findings early, we might reduce iterations
                let avg_confidence: f64 =
                    findings.iter().map(|f| f.confidence).sum::<f64>() / findings.len() as f64;

                if avg_confidence > 0.9 && iteration >= 2 {
                    info!("High confidence achieved early, reducing iterations");
                    Ok(AdaptiveResearchStrategy::DeepDive {
                        max_iterations: iteration + 1,
                        focus_areas: focus_areas.clone(),
                    })
                } else if avg_confidence < 0.5 && iteration < *max_iterations {
                    info!("Low confidence, extending research");
                    Ok(AdaptiveResearchStrategy::DeepDive {
                        max_iterations: max_iterations + 1,
                        focus_areas: focus_areas.clone(),
                    })
                } else {
                    Ok(current_strategy.clone())
                }
            }
            _ => Ok(current_strategy.clone()),
        }
    }
}

impl Default for ResearchContext {
    fn default() -> Self {
        Self {
            session_id: "default".to_string(),
            topic: "".to_string(),
            config: ResearchConfig::default(),
            current_iteration: 0,
            questions: vec![],
            findings: vec![],
            iterations: vec![],
            start_time: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }
}
