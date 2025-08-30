//! Research planning and question decomposition

use super::types::*;
use crate::{ApplicationError, ApplicationResult};
use serde_json::Value;
use siumai::prelude::{ChatCapability, ChatMessage};
use tracing::{debug, info};
use uuid::Uuid;

/// Research planner that breaks down complex topics into manageable questions
pub struct ResearchPlanner {
    config: ResearchConfig,
    llm_client: Option<Box<dyn ChatCapability>>,
}

impl ResearchPlanner {
    /// Create a new research planner
    pub fn new(config: ResearchConfig) -> Self {
        Self {
            config,
            llm_client: None,
        }
    }

    /// Create a new research planner with LLM client
    pub fn with_llm_client(config: ResearchConfig, llm_client: Box<dyn ChatCapability>) -> Self {
        Self {
            config,
            llm_client: Some(llm_client),
        }
    }

    /// Plan initial research questions for a given topic
    pub async fn plan_initial_research(
        &self,
        topic: &str,
    ) -> ApplicationResult<Vec<ResearchQuestion>> {
        info!("Planning initial research for topic: {}", topic);

        let questions = if let Some(ref llm_client) = self.llm_client {
            // Use LLM for intelligent question generation
            self.generate_questions_with_llm(topic, llm_client).await?
        } else {
            // Fallback to template-based generation
            self.decompose_topic(topic).await?
        };

        debug!("Generated {} initial questions", questions.len());
        for question in &questions {
            debug!("  - {} ({})", question.text, question.question_type);
        }

        Ok(questions)
    }

    /// Generate follow-up questions based on current findings
    pub async fn plan_followup_questions(
        &self,
        context: &ResearchContext,
        findings: &[ResearchFinding],
    ) -> ApplicationResult<Vec<ResearchQuestion>> {
        info!(
            "Planning follow-up questions based on {} findings",
            findings.len()
        );

        let mut new_questions = Vec::new();

        // Analyze findings for gaps and contradictions
        let gaps = self.identify_knowledge_gaps(context, findings).await?;
        let contradictions = self.identify_contradictions(findings).await?;

        // Generate questions to address gaps
        for gap in gaps {
            let question = self.create_gap_question(&gap, context).await?;
            new_questions.push(question);
        }

        // Generate questions to resolve contradictions
        for contradiction in contradictions {
            let question = self
                .create_contradiction_question(&contradiction, context)
                .await?;
            new_questions.push(question);
        }

        // Prioritize questions
        self.prioritize_questions(&mut new_questions, context)
            .await?;

        debug!("Generated {} follow-up questions", new_questions.len());
        Ok(new_questions)
    }

    /// Generate research questions using LLM
    async fn generate_questions_with_llm(
        &self,
        topic: &str,
        llm_client: &Box<dyn ChatCapability>,
    ) -> ApplicationResult<Vec<ResearchQuestion>> {
        let prompt = format!(
            r#"You are a research planning expert. Given a topic, generate a comprehensive set of research questions that would help understand the topic thoroughly.

Topic: "{}"

Please generate 8-12 diverse research questions covering different aspects:
1. Conceptual questions (what, why, purpose)
2. Technical questions (how, implementation, architecture)
3. Comparative questions (alternatives, trade-offs)
4. Historical questions (evolution, background)
5. Practical questions (usage, examples, best practices)

Format your response as a JSON array of objects with the following structure:
[
  {{
    "text": "What is the main purpose of...?",
    "type": "conceptual",
    "priority": 8,
    "complexity": 5,
    "keywords": ["keyword1", "keyword2"]
  }}
]

Ensure questions are specific, actionable, and would lead to meaningful research findings."#,
            topic
        );

        let messages = vec![ChatMessage::user(prompt).build()];

        let response = llm_client
            .chat_with_tools(messages, None)
            .await
            .map_err(|e| ApplicationError::Research {
                message: format!("Failed to generate questions with LLM: {}", e),
            })?;

        // Parse the JSON response
        let content_text = response.content_text().unwrap_or_default();
        self.parse_llm_questions_response(content_text, topic).await
    }

    /// Decompose a topic into initial research questions (fallback method)
    async fn decompose_topic(&self, topic: &str) -> ApplicationResult<Vec<ResearchQuestion>> {
        let mut questions = Vec::new();

        // Generate different types of questions based on the topic
        questions.extend(self.generate_conceptual_questions(topic).await?);
        questions.extend(self.generate_technical_questions(topic).await?);
        questions.extend(self.generate_architectural_questions(topic).await?);
        questions.extend(self.generate_historical_questions(topic).await?);

        // Limit to max questions and prioritize
        if questions.len() > self.config.max_sources_per_iteration {
            questions.truncate(self.config.max_sources_per_iteration);
        }

        Ok(questions)
    }

    /// Parse LLM response into research questions
    async fn parse_llm_questions_response(
        &self,
        response: &str,
        topic: &str,
    ) -> ApplicationResult<Vec<ResearchQuestion>> {
        use serde_json::Value;

        // Try to extract JSON from the response
        let json_str = if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            // If no JSON found, fallback to template-based generation
            debug!("No JSON found in LLM response, falling back to template-based generation");
            return self.decompose_topic(topic).await;
        };

        let questions_json: Value = serde_json::from_str(json_str).map_err(|e| {
            debug!(
                "Failed to parse LLM response as JSON: {}, falling back to templates",
                e
            );
            // Don't return error, just use fallback
            ApplicationError::Research {
                message: format!("JSON parse error: {}", e),
            }
        })?;

        let mut questions = Vec::new();

        if let Some(questions_array) = questions_json.as_array() {
            for (i, question_json) in questions_array.iter().enumerate() {
                if let Some(question) = self.parse_single_question(question_json, i) {
                    questions.push(question);
                }
            }
        }

        // If we got no questions from LLM, fallback to templates
        if questions.is_empty() {
            debug!("No valid questions parsed from LLM response, using template fallback");
            return self.decompose_topic(topic).await;
        }

        // Limit to max questions
        if questions.len() > self.config.max_sources_per_iteration {
            questions.truncate(self.config.max_sources_per_iteration);
        }

        Ok(questions)
    }

    /// Parse a single question from JSON
    fn parse_single_question(
        &self,
        question_json: &Value,
        _index: usize,
    ) -> Option<ResearchQuestion> {
        let text = question_json.get("text")?.as_str()?.to_string();
        let type_str = question_json.get("type")?.as_str().unwrap_or("conceptual");
        let priority = question_json.get("priority")?.as_u64().unwrap_or(5) as u8;
        let complexity = question_json.get("complexity")?.as_u64().unwrap_or(5) as u8;

        let keywords = if let Some(keywords_array) =
            question_json.get("keywords").and_then(|k| k.as_array())
        {
            keywords_array
                .iter()
                .filter_map(|k| k.as_str())
                .map(|s| s.to_string())
                .collect()
        } else {
            self.extract_keywords(&text)
        };

        let question_type = match type_str {
            "technical" => QuestionType::Technical,
            "architectural" => QuestionType::Architectural,
            "historical" => QuestionType::Historical,
            "comparative" => QuestionType::Comparative,
            _ => QuestionType::Conceptual,
        };

        Some(ResearchQuestion {
            id: Uuid::new_v4(),
            text,
            question_type,
            priority: priority.clamp(1, 10),
            parent_id: None,
            depth: 0,
            complexity: complexity.clamp(1, 10),
            keywords,
        })
    }

    /// Generate conceptual questions about the topic (fallback method)
    async fn generate_conceptual_questions(
        &self,
        topic: &str,
    ) -> ApplicationResult<Vec<ResearchQuestion>> {
        let templates = vec![
            "What is the main purpose and goal of {}?",
            "What are the key concepts and principles behind {}?",
            "What problem does {} solve?",
            "What are the main benefits and advantages of {}?",
            "What are the limitations and drawbacks of {}?",
        ];

        let mut questions = Vec::new();
        for (i, template) in templates.iter().enumerate() {
            let question_text = template.replace("{}", topic);
            let question = ResearchQuestion {
                id: Uuid::new_v4(),
                text: question_text,
                question_type: QuestionType::Conceptual,
                priority: 8 - i as u8, // Higher priority for earlier questions
                parent_id: None,
                depth: 0,
                complexity: 5,
                keywords: self.extract_keywords(topic),
            };
            questions.push(question);
        }

        Ok(questions)
    }

    /// Generate technical questions about the topic
    async fn generate_technical_questions(
        &self,
        topic: &str,
    ) -> ApplicationResult<Vec<ResearchQuestion>> {
        let templates = vec![
            "How is {} implemented technically?",
            "What are the key technical components of {}?",
            "What technologies and frameworks are used in {}?",
            "How does {} handle data and state management?",
            "What are the performance characteristics of {}?",
        ];

        let mut questions = Vec::new();
        for (i, template) in templates.iter().enumerate() {
            let question_text = template.replace("{}", topic);
            let question = ResearchQuestion {
                id: Uuid::new_v4(),
                text: question_text,
                question_type: QuestionType::Technical,
                priority: 7 - i as u8,
                parent_id: None,
                depth: 0,
                complexity: 7,
                keywords: self.extract_keywords(topic),
            };
            questions.push(question);
        }

        Ok(questions)
    }

    /// Generate architectural questions about the topic
    async fn generate_architectural_questions(
        &self,
        topic: &str,
    ) -> ApplicationResult<Vec<ResearchQuestion>> {
        let templates = vec![
            "What is the overall architecture of {}?",
            "How are the different components of {} organized?",
            "What are the main modules and their responsibilities in {}?",
            "How does {} handle scalability and extensibility?",
        ];

        let mut questions = Vec::new();
        for (i, template) in templates.iter().enumerate() {
            let question_text = template.replace("{}", topic);
            let question = ResearchQuestion {
                id: Uuid::new_v4(),
                text: question_text,
                question_type: QuestionType::Architectural,
                priority: 6 - i as u8,
                parent_id: None,
                depth: 0,
                complexity: 6,
                keywords: self.extract_keywords(topic),
            };
            questions.push(question);
        }

        Ok(questions)
    }

    /// Generate historical questions about the topic
    async fn generate_historical_questions(
        &self,
        topic: &str,
    ) -> ApplicationResult<Vec<ResearchQuestion>> {
        let templates = vec![
            "How has {} evolved over time?",
            "What are the major versions and changes in {}?",
            "What were the key decisions and trade-offs in developing {}?",
        ];

        let mut questions = Vec::new();
        for (i, template) in templates.iter().enumerate() {
            let question_text = template.replace("{}", topic);
            let question = ResearchQuestion {
                id: Uuid::new_v4(),
                text: question_text,
                question_type: QuestionType::Historical,
                priority: 4 - i as u8,
                parent_id: None,
                depth: 0,
                complexity: 4,
                keywords: self.extract_keywords(topic),
            };
            questions.push(question);
        }

        Ok(questions)
    }

    /// Identify knowledge gaps in current research
    async fn identify_knowledge_gaps(
        &self,
        context: &ResearchContext,
        findings: &[ResearchFinding],
    ) -> ApplicationResult<Vec<String>> {
        let mut gaps = Vec::new();

        // Check for unanswered questions
        for question in &context.questions {
            let has_findings = findings.iter().any(|f| f.question_id == question.id);
            if !has_findings {
                gaps.push(format!("No findings for: {}", question.text));
            }
        }

        // Check for low-confidence areas
        for finding in findings {
            if finding.confidence < self.config.confidence_threshold {
                gaps.push(format!("Low confidence in: {}", finding.content));
            }
        }

        Ok(gaps)
    }

    /// Identify contradictions in findings
    async fn identify_contradictions(
        &self,
        findings: &[ResearchFinding],
    ) -> ApplicationResult<Vec<String>> {
        let mut contradictions = Vec::new();

        // Simple contradiction detection based on keywords
        // In a real implementation, this would use more sophisticated NLP
        for (i, finding1) in findings.iter().enumerate() {
            for finding2 in findings.iter().skip(i + 1) {
                if finding1.question_id == finding2.question_id {
                    // Check for contradictory statements
                    if self.are_contradictory(&finding1.content, &finding2.content) {
                        contradictions.push(format!(
                            "Contradiction between findings: '{}' vs '{}'",
                            finding1.content, finding2.content
                        ));
                    }
                }
            }
        }

        Ok(contradictions)
    }

    /// Create a question to address a knowledge gap
    async fn create_gap_question(
        &self,
        gap: &str,
        context: &ResearchContext,
    ) -> ApplicationResult<ResearchQuestion> {
        Ok(ResearchQuestion {
            id: Uuid::new_v4(),
            text: format!("Research needed: {}", gap),
            question_type: QuestionType::Technical, // Default type
            priority: 6,
            parent_id: None,
            depth: 1,
            complexity: 5,
            keywords: self.extract_keywords(&context.topic),
        })
    }

    /// Create a question to resolve a contradiction
    async fn create_contradiction_question(
        &self,
        contradiction: &str,
        context: &ResearchContext,
    ) -> ApplicationResult<ResearchQuestion> {
        Ok(ResearchQuestion {
            id: Uuid::new_v4(),
            text: format!("Resolve contradiction: {}", contradiction),
            question_type: QuestionType::Diagnostic,
            priority: 8, // High priority for contradictions
            parent_id: None,
            depth: 1,
            complexity: 7,
            keywords: self.extract_keywords(&context.topic),
        })
    }

    /// Prioritize questions based on various factors
    async fn prioritize_questions(
        &self,
        questions: &mut [ResearchQuestion],
        _context: &ResearchContext,
    ) -> ApplicationResult<()> {
        questions.sort_by(|a, b| {
            // Sort by priority (higher first), then by complexity (lower first)
            b.priority
                .cmp(&a.priority)
                .then(a.complexity.cmp(&b.complexity))
        });

        Ok(())
    }

    /// Extract keywords from text (simplified implementation)
    fn extract_keywords(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .filter(|word| word.len() > 3)
            .map(|word| word.to_lowercase())
            .collect()
    }

    /// Check if two text snippets are contradictory (simplified implementation)
    fn are_contradictory(&self, text1: &str, text2: &str) -> bool {
        // Simple keyword-based contradiction detection
        let contradictory_pairs = vec![
            ("yes", "no"),
            ("true", "false"),
            ("supports", "doesn't support"),
            ("enables", "disables"),
            ("allows", "prevents"),
        ];

        for (word1, word2) in contradictory_pairs {
            if (text1.contains(word1) && text2.contains(word2))
                || (text1.contains(word2) && text2.contains(word1))
            {
                return true;
            }
        }

        false
    }
}
