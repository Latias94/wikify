//! Tests for the enhanced research engine with LLM integration

use uuid::Uuid;
use wikify_applications::research::types::{
    ResearchConfig, ResearchFinding, SourceInfo, SourceType,
};
use wikify_applications::research::{ResearchPlanner, ResearchSynthesizer};

/// Mock LLM client for testing
struct MockLlmClient;

#[async_trait::async_trait]
impl siumai::prelude::ChatCapability for MockLlmClient {
    async fn chat_with_tools<'a>(
        &'a self,
        messages: Vec<siumai::prelude::ChatMessage>,
        _tools: Option<Vec<siumai::prelude::Tool>>,
    ) -> Result<siumai::prelude::ChatResponse, siumai::prelude::LlmError> {
        // Extract the prompt from the first message
        let prompt = messages
            .first()
            .and_then(|m| m.content_text())
            .unwrap_or("");

        // Generate mock response based on prompt content
        let mock_response = if prompt.contains("research questions") {
            // Mock response for question generation
            r#"[
                {
                    "text": "What is the main purpose and architecture of this system?",
                    "type": "conceptual",
                    "priority": 8,
                    "complexity": 5,
                    "keywords": ["architecture", "purpose", "system"]
                },
                {
                    "text": "How is the system implemented technically?",
                    "type": "technical", 
                    "priority": 7,
                    "complexity": 6,
                    "keywords": ["implementation", "technical", "system"]
                },
                {
                    "text": "What are the key components and their interactions?",
                    "type": "architectural",
                    "priority": 6,
                    "complexity": 7,
                    "keywords": ["components", "interactions", "architecture"]
                }
            ]"#
        } else if prompt.contains("research report") {
            // Mock response for report generation
            r#"# Comprehensive Research Report: Test Topic

## Executive Summary
This research has successfully analyzed the test topic through systematic investigation.

## Research Overview and Methodology
The research was conducted using an iterative approach with multiple sources and findings.

## Detailed Findings Analysis
Key findings have been identified and analyzed for their significance and implications.

## Key Insights and Patterns
Several important patterns emerged from the research data.

## Conclusions and Implications
The research provides valuable insights into the topic under investigation.

## Recommendations for Further Research
Additional research areas have been identified for future investigation."#
        } else {
            "Mock LLM response for testing purposes."
        };

        Ok(siumai::prelude::ChatResponse {
            id: Some("mock-response".to_string()),
            content: siumai::prelude::MessageContent::Text(mock_response.to_string()),
            model: Some("mock-model".to_string()),
            usage: None,
            finish_reason: Some(siumai::prelude::FinishReason::Stop),
            tool_calls: None,
            thinking: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn chat_stream<'a>(
        &'a self,
        _messages: Vec<siumai::prelude::ChatMessage>,
        _tools: Option<Vec<siumai::prelude::Tool>>,
    ) -> Result<siumai::prelude::ChatStream, siumai::prelude::LlmError> {
        Err(siumai::prelude::LlmError::UnsupportedOperation(
            "Streaming not supported in mock".to_string(),
        ))
    }
}

// Removed create_test_context function as it's not needed for these tests

#[tokio::test]
async fn test_research_planner_with_llm() {
    // Create research planner with mock LLM
    let config = ResearchConfig::default();
    let llm_client = Box::new(MockLlmClient);
    let planner = ResearchPlanner::with_llm_client(config, llm_client);

    // Test question generation
    let questions = planner.plan_initial_research("Test Topic").await.unwrap();

    assert!(!questions.is_empty(), "Should generate questions");
    assert!(
        questions.len() <= 10,
        "Should not generate too many questions"
    );

    // Verify question structure
    for question in &questions {
        assert!(
            !question.text.is_empty(),
            "Question text should not be empty"
        );
        assert!(question.priority > 0, "Question should have priority");
        assert!(question.complexity > 0, "Question should have complexity");
    }

    println!("Generated {} questions with LLM", questions.len());
    for question in &questions {
        println!("  - {} ({})", question.text, question.question_type);
    }
}

#[tokio::test]
async fn test_research_synthesizer_with_llm() {
    // Create research synthesizer with mock LLM
    let config = ResearchConfig::default();
    let llm_client = Box::new(MockLlmClient);
    let synthesizer = ResearchSynthesizer::with_llm_client(config, llm_client);

    // Create mock findings
    let findings = vec![
        ResearchFinding {
            id: Uuid::new_v4(),
            question_id: Uuid::new_v4(),
            content: "Test finding 1".to_string(),
            confidence: 0.8,
            relevance: 0.9,
            source: SourceInfo {
                id: "source1".to_string(),
                source_type: SourceType::Documentation,
                title: Some("Test Source 1".to_string()),
                author: None,
                last_modified: None,
                reliability: 0.9,
            },
            evidence: vec!["Evidence 1".to_string()],
            limitations: vec![],
            timestamp: chrono::Utc::now(),
        },
        ResearchFinding {
            id: Uuid::new_v4(),
            question_id: Uuid::new_v4(),
            content: "Test finding 2".to_string(),
            confidence: 0.9,
            relevance: 0.8,
            source: SourceInfo {
                id: "source2".to_string(),
                source_type: SourceType::Documentation,
                title: Some("Test Source 2".to_string()),
                author: None,
                last_modified: None,
                reliability: 0.8,
            },
            evidence: vec!["Evidence 2".to_string()],
            limitations: vec![],
            timestamp: chrono::Utc::now(),
        },
    ];

    // Test report generation
    let report = synthesizer
        .create_final_report("Test Topic", &findings, &[])
        .await
        .unwrap();

    assert!(
        !report.final_report.is_empty(),
        "Should generate final report"
    );
    assert!(!report.summary.is_empty(), "Should generate summary");
    assert!(
        !report.key_findings.is_empty(),
        "Should extract key findings"
    );

    println!("Generated report summary: {}", report.summary);
    println!("Report contains {} key findings", report.key_findings.len());
}

#[tokio::test]
async fn test_fallback_without_llm() {
    // Test that the system works without LLM (fallback mode)
    let config = ResearchConfig::default();
    let planner = ResearchPlanner::new(config.clone());
    let synthesizer = ResearchSynthesizer::new(config);

    // Test question generation fallback
    let questions = planner.plan_initial_research("Test Topic").await.unwrap();
    assert!(
        !questions.is_empty(),
        "Should generate questions even without LLM"
    );

    // Test report generation fallback
    let findings = vec![];
    let report = synthesizer
        .create_final_report("Test Topic", &findings, &[])
        .await
        .unwrap();
    assert!(
        !report.final_report.is_empty(),
        "Should generate report even without LLM"
    );

    println!("Fallback mode works correctly");
}
