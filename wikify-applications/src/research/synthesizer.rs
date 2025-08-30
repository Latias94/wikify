//! Research synthesizer for combining findings into coherent reports

use super::types::*;
use crate::ApplicationResult;
use siumai::prelude::ChatCapability;
use std::collections::HashMap;
use tracing::info;

/// Research synthesizer that combines findings into coherent reports
pub struct ResearchSynthesizer {
    config: ResearchConfig,
    llm_client: Option<Box<dyn ChatCapability>>,
}

impl ResearchSynthesizer {
    /// Create a new research synthesizer
    pub fn new(config: ResearchConfig) -> Self {
        Self {
            config,
            llm_client: None,
        }
    }

    /// Create a new research synthesizer with LLM client
    pub fn with_llm_client(config: ResearchConfig, llm_client: Box<dyn ChatCapability>) -> Self {
        Self {
            config,
            llm_client: Some(llm_client),
        }
    }

    /// Create a partial synthesis of current findings
    pub async fn create_partial_synthesis(
        &self,
        topic: &str,
        findings: &[ResearchFinding],
    ) -> ApplicationResult<String> {
        info!("Creating partial synthesis for {} findings", findings.len());

        if findings.is_empty() {
            return Ok("No findings available yet.".to_string());
        }

        let mut synthesis = String::new();
        synthesis.push_str(&format!("# Research Progress: {}\n\n", topic));

        // Group findings by question type
        let grouped_findings = self.group_findings_by_type(findings);

        // Synthesize each group
        for (question_type, type_findings) in grouped_findings {
            synthesis.push_str(&format!(
                "## {} Findings\n\n",
                self.format_question_type(&question_type)
            ));

            for finding in type_findings {
                synthesis.push_str(&format!(
                    "- **{}** (Confidence: {:.1}%)\n",
                    finding.content,
                    finding.confidence * 100.0
                ));

                if !finding.evidence.is_empty() {
                    synthesis.push_str("  - Evidence: ");
                    synthesis.push_str(&finding.evidence.join(", "));
                    synthesis.push('\n');
                }

                if !finding.limitations.is_empty() {
                    synthesis.push_str("  - Limitations: ");
                    synthesis.push_str(&finding.limitations.join(", "));
                    synthesis.push('\n');
                }
                synthesis.push('\n');
            }
        }

        // Add confidence assessment
        let overall_confidence = self.calculate_overall_confidence(findings);
        synthesis.push_str(&format!(
            "\n## Overall Confidence: {:.1}%\n",
            overall_confidence * 100.0
        ));

        Ok(synthesis)
    }

    /// Create a final comprehensive research report
    pub async fn create_final_report(
        &self,
        topic: &str,
        findings: &[ResearchFinding],
        iterations: &[ResearchIteration],
    ) -> ApplicationResult<ResearchResult> {
        info!("Creating final research report for topic: {}", topic);

        // Generate executive summary
        let summary = self.generate_executive_summary(topic, findings).await?;

        // Generate final report content
        let final_report = self
            .generate_final_report_content(topic, findings, iterations)
            .await?;

        // Extract key findings
        let key_findings = self.extract_key_findings(findings).await?;

        // Generate recommendations
        let recommendations = self.generate_recommendations(topic, findings).await?;

        // Identify areas for further research
        let further_research = self.identify_further_research(findings, iterations).await?;

        // Calculate metrics
        let metrics = self.calculate_research_metrics(findings, iterations);

        // Calculate total duration
        let total_duration = iterations.iter().map(|i| i.duration).sum();

        Ok(ResearchResult {
            session_id: "placeholder".to_string(), // Will be set by caller
            topic: topic.to_string(),
            config: self.config.clone(),
            iterations: iterations.to_vec(),
            final_report,
            summary,
            key_findings,
            recommendations,
            further_research,
            overall_confidence: self.calculate_overall_confidence(findings),
            total_duration,
            metrics,
        })
    }

    /// Group findings by question type
    fn group_findings_by_type<'a>(
        &self,
        findings: &'a [ResearchFinding],
    ) -> HashMap<QuestionType, Vec<&'a ResearchFinding>> {
        let mut grouped = HashMap::new();

        for finding in findings {
            // For now, we'll use a default type since we don't have question type in findings
            // In a real implementation, we'd look up the question type from the question
            let question_type = QuestionType::Technical; // Default
            grouped
                .entry(question_type)
                .or_insert_with(Vec::new)
                .push(finding);
        }

        grouped
    }

    /// Format question type for display
    fn format_question_type(&self, question_type: &QuestionType) -> String {
        match question_type {
            QuestionType::Conceptual => "Conceptual",
            QuestionType::Technical => "Technical",
            QuestionType::Architectural => "Architectural",
            QuestionType::Historical => "Historical",
            QuestionType::Comparative => "Comparative",
            QuestionType::Diagnostic => "Diagnostic",
            QuestionType::Advisory => "Advisory",
        }
        .to_string()
    }

    /// Calculate overall confidence across all findings
    fn calculate_overall_confidence(&self, findings: &[ResearchFinding]) -> f64 {
        if findings.is_empty() {
            return 0.0;
        }

        let total_confidence: f64 = findings.iter().map(|f| f.confidence).sum();
        total_confidence / findings.len() as f64
    }

    /// Generate executive summary
    async fn generate_executive_summary(
        &self,
        topic: &str,
        findings: &[ResearchFinding],
    ) -> ApplicationResult<String> {
        let mut summary = String::new();

        summary.push_str(&format!("This research investigated {}. ", topic));
        summary.push_str(&format!(
            "Based on {} findings from multiple sources, ",
            findings.len()
        ));

        let confidence = self.calculate_overall_confidence(findings);
        if confidence > 0.8 {
            summary.push_str("we have high confidence in our understanding. ");
        } else if confidence > 0.6 {
            summary.push_str("we have moderate confidence in our understanding. ");
        } else {
            summary
                .push_str("our understanding is preliminary and requires further investigation. ");
        }

        // Add key insights
        let high_confidence_findings: Vec<_> = findings
            .iter()
            .filter(|f| f.confidence > 0.8)
            .take(3)
            .collect();

        if !high_confidence_findings.is_empty() {
            summary.push_str("Key insights include: ");
            for (i, finding) in high_confidence_findings.iter().enumerate() {
                if i > 0 {
                    summary.push_str(", ");
                }
                summary.push_str(&finding.content);
            }
            summary.push('.');
        }

        Ok(summary)
    }

    /// Generate final report content
    async fn generate_final_report_content(
        &self,
        topic: &str,
        findings: &[ResearchFinding],
        iterations: &[ResearchIteration],
    ) -> ApplicationResult<String> {
        if let Some(ref llm_client) = self.llm_client {
            // Use LLM for intelligent report generation
            self.generate_report_with_llm(topic, findings, iterations, llm_client)
                .await
        } else {
            // Fallback to template-based generation
            self.generate_report_template_based(topic, findings, iterations)
                .await
        }
    }

    /// Generate report using LLM
    async fn generate_report_with_llm(
        &self,
        topic: &str,
        findings: &[ResearchFinding],
        iterations: &[ResearchIteration],
        llm_client: &Box<dyn ChatCapability>,
    ) -> ApplicationResult<String> {
        // Prepare findings summary for LLM
        let findings_summary = findings
            .iter()
            .map(|f| format!("- {} (Confidence: {:.1}%)", f.content, f.confidence * 100.0))
            .collect::<Vec<_>>()
            .join("\n");

        let iterations_summary = iterations
            .iter()
            .enumerate()
            .map(|(i, iter)| {
                format!(
                    "Iteration {}: {} questions explored, {} findings discovered",
                    i + 1,
                    iter.questions.len(),
                    iter.findings.len()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"You are a research analyst tasked with creating a comprehensive research report. Based on the research conducted, generate a well-structured, professional report.

Research Topic: "{}"

Research Process:
{}

Key Findings:
{}

Please generate a comprehensive research report with the following structure:
1. Executive Summary
2. Research Overview and Methodology
3. Detailed Findings Analysis
4. Key Insights and Patterns
5. Conclusions and Implications
6. Recommendations for Further Research

Requirements:
- Use clear, professional language
- Organize information logically
- Highlight the most significant findings
- Provide actionable insights
- Include confidence levels where relevant
- Make connections between different findings
- Ensure the report is comprehensive yet concise

Format the output in Markdown with appropriate headers and structure."#,
            topic, iterations_summary, findings_summary
        );

        let messages = vec![siumai::prelude::ChatMessage::user(prompt).build()];

        let response = llm_client
            .chat_with_tools(messages, None)
            .await
            .map_err(|e| crate::ApplicationError::Research {
                message: format!("Failed to generate report with LLM: {}", e),
            })?;

        let content = response.content_text().unwrap_or_default();
        Ok(content.to_string())
    }

    /// Generate report using template-based approach (fallback)
    async fn generate_report_template_based(
        &self,
        topic: &str,
        findings: &[ResearchFinding],
        iterations: &[ResearchIteration],
    ) -> ApplicationResult<String> {
        let mut report = String::new();

        report.push_str(&format!("# Comprehensive Research Report: {}\n\n", topic));

        // Research overview
        report.push_str("## Research Overview\n\n");
        report.push_str(&format!(
            "This research was conducted over {} iterations, ",
            iterations.len()
        ));
        report.push_str(&format!(
            "examining {} sources and generating {} findings.\n\n",
            findings.len(),
            findings.len()
        ));

        // Methodology
        report.push_str("## Methodology\n\n");
        report.push_str("This research employed an iterative approach, progressively deepening understanding through:\n");
        report.push_str("- Systematic question decomposition\n");
        report.push_str("- Multi-source information gathering\n");
        report.push_str("- Iterative synthesis and validation\n\n");

        // Findings by iteration
        report.push_str("## Research Findings\n\n");
        for (i, iteration) in iterations.iter().enumerate() {
            report.push_str(&format!("### Iteration {} Findings\n\n", i + 1));
            report.push_str(&iteration.partial_synthesis);
            report.push('\n');
        }

        // Conclusions
        report.push_str("## Conclusions\n\n");
        report.push_str(&self.generate_conclusions(findings).await?);

        Ok(report)
    }

    /// Extract key findings
    async fn extract_key_findings(
        &self,
        findings: &[ResearchFinding],
    ) -> ApplicationResult<Vec<String>> {
        let mut key_findings = findings
            .iter()
            .filter(|f| f.confidence > self.config.confidence_threshold)
            .map(|f| f.content.clone())
            .collect::<Vec<_>>();

        // Sort by confidence and take top findings
        key_findings.sort_by_key(|b| std::cmp::Reverse(b.len())); // Simple heuristic: longer = more detailed
        key_findings.truncate(5); // Top 5 findings

        Ok(key_findings)
    }

    /// Generate recommendations
    async fn generate_recommendations(
        &self,
        topic: &str,
        findings: &[ResearchFinding],
    ) -> ApplicationResult<Vec<String>> {
        let mut recommendations = Vec::new();

        // Generate recommendations based on findings
        if findings.iter().any(|f| f.confidence < 0.5) {
            recommendations
                .push("Further investigation is needed in areas with low confidence".to_string());
        }

        if findings.len() < 5 {
            recommendations.push(
                "Additional sources should be consulted for comprehensive understanding"
                    .to_string(),
            );
        }

        // Add topic-specific recommendations
        recommendations.push(format!(
            "Consider practical applications of {} in your specific context",
            topic
        ));
        recommendations
            .push("Validate findings through hands-on experimentation where possible".to_string());

        Ok(recommendations)
    }

    /// Identify areas for further research
    async fn identify_further_research(
        &self,
        findings: &[ResearchFinding],
        iterations: &[ResearchIteration],
    ) -> ApplicationResult<Vec<String>> {
        let mut further_research = Vec::new();

        // Check for unanswered questions from last iteration
        if let Some(last_iteration) = iterations.last() {
            if !last_iteration.new_questions.is_empty() {
                further_research.push("Investigate remaining unanswered questions".to_string());
            }
        }

        // Check for low-confidence areas
        let low_confidence_areas: Vec<_> = findings
            .iter()
            .filter(|f| f.confidence < 0.6)
            .map(|f| format!("Verify: {}", f.content))
            .collect();

        further_research.extend(low_confidence_areas);

        // Check for contradictions
        if findings.iter().any(|f| !f.limitations.is_empty()) {
            further_research.push("Resolve identified limitations and contradictions".to_string());
        }

        Ok(further_research)
    }

    /// Calculate research quality metrics
    fn calculate_research_metrics(
        &self,
        findings: &[ResearchFinding],
        iterations: &[ResearchIteration],
    ) -> ResearchMetrics {
        let sources_consulted = findings
            .iter()
            .map(|f| &f.source.id)
            .collect::<std::collections::HashSet<_>>()
            .len();

        let questions_explored = iterations.iter().map(|i| i.questions.len()).sum();

        let average_confidence = self.calculate_overall_confidence(findings);

        // Simple heuristics for other metrics
        let coverage_score = (findings.len() as f64 / 10.0).min(1.0); // Assume 10 findings = full coverage
        let depth_score = (iterations.len() as f64 / self.config.max_iterations as f64).min(1.0);
        let coherence_score = average_confidence; // Use confidence as proxy for coherence

        ResearchMetrics {
            sources_consulted,
            questions_explored,
            findings_discovered: findings.len(),
            average_confidence,
            coverage_score,
            depth_score,
            coherence_score,
        }
    }

    /// Generate conclusions
    async fn generate_conclusions(
        &self,
        findings: &[ResearchFinding],
    ) -> ApplicationResult<String> {
        let mut conclusions = String::new();

        let confidence = self.calculate_overall_confidence(findings);

        if confidence > 0.8 {
            conclusions.push_str("The research provides a comprehensive understanding of the topic with high confidence. ");
        } else if confidence > 0.6 {
            conclusions.push_str("The research provides good insights with moderate confidence. ");
        } else {
            conclusions.push_str(
                "The research provides initial insights but requires further investigation. ",
            );
        }

        conclusions.push_str(&format!("Based on {} findings, ", findings.len()));
        conclusions.push_str("the key takeaways are documented above and should be considered in context of the identified limitations.");

        Ok(conclusions)
    }
}
