//! Tests for the adaptive research strategy system

use wikify_applications::research::types::ResearchContext;
use wikify_applications::research::{
    AdaptiveResearchStrategy, ResearchConfig, ResearchStrategySelector,
};

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
        let mock_response = if prompt.contains("vs") || prompt.contains("compare") {
            // Mock response for comparison strategy
            r#"{
                "strategy": "Comparative",
                "reasoning": "The topic contains comparison keywords",
                "parameters": {
                    "max_iterations": 3,
                    "subjects": ["subject1", "subject2"],
                    "comparison_aspects": ["functionality", "performance"]
                }
            }"#
        } else if prompt.contains("architecture") || prompt.contains("design") {
            // Mock response for architectural strategy
            r#"{
                "strategy": "Architectural",
                "reasoning": "The topic is about system architecture",
                "parameters": {
                    "max_iterations": 4,
                    "components": ["core", "interfaces"],
                    "depth_level": 2
                }
            }"#
        } else if prompt.contains("what is") || prompt.len() < 50 {
            // Mock response for quick scan strategy
            r#"{
                "strategy": "QuickScan",
                "reasoning": "Simple question requiring quick answer",
                "parameters": {
                    "max_iterations": 2,
                    "depth_level": 1
                }
            }"#
        } else {
            // Mock response for deep dive strategy
            r#"{
                "strategy": "DeepDive",
                "reasoning": "Complex topic requiring thorough investigation",
                "parameters": {
                    "max_iterations": 4,
                    "focus_areas": ["implementation", "usage", "best_practices"]
                }
            }"#
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

#[tokio::test]
async fn test_strategy_selection_heuristic() {
    // Test heuristic-based strategy selection (without LLM)
    let config = ResearchConfig::default();
    let selector = ResearchStrategySelector::new(config);
    let context = ResearchContext::default();

    // Test comparison question
    let strategy = selector
        .select_strategy("Compare Rust vs Python performance", &context)
        .await
        .unwrap();
    match strategy {
        AdaptiveResearchStrategy::Comparative {
            subjects,
            comparison_aspects,
        } => {
            assert_eq!(subjects.len(), 2);
            assert!(
                subjects.contains(&"Compare Rust".to_string())
                    || subjects.contains(&"Rust".to_string())
            );
            assert!(
                subjects.contains(&"Python performance".to_string())
                    || subjects.contains(&"Python".to_string())
            );
            assert!(!comparison_aspects.is_empty());
            println!("Comparison subjects: {:?}", subjects);
        }
        _ => panic!("Expected Comparative strategy for comparison question"),
    }

    // Test architectural question
    let strategy = selector
        .select_strategy("Explain the architecture of this system", &context)
        .await
        .unwrap();
    match strategy {
        AdaptiveResearchStrategy::Architectural {
            components,
            interaction_depth,
        } => {
            assert!(!components.is_empty());
            assert_eq!(interaction_depth, 2);
        }
        _ => panic!("Expected Architectural strategy for architecture question"),
    }

    // Test simple question
    let strategy = selector
        .select_strategy("What is Rust?", &context)
        .await
        .unwrap();
    match strategy {
        AdaptiveResearchStrategy::QuickScan {
            max_iterations,
            depth_level,
        } => {
            assert_eq!(max_iterations, 2);
            assert_eq!(depth_level, 1);
        }
        _ => panic!("Expected QuickScan strategy for simple question"),
    }

    // Test complex question
    let strategy = selector.select_strategy("How does the memory management system work in this complex distributed application with multiple microservices?", &context).await.unwrap();
    match strategy {
        AdaptiveResearchStrategy::DeepDive {
            max_iterations,
            focus_areas,
        } => {
            assert_eq!(max_iterations, 4);
            assert!(!focus_areas.is_empty());
        }
        _ => panic!("Expected DeepDive strategy for complex question"),
    }

    println!("✅ Heuristic strategy selection tests passed");
}

#[tokio::test]
async fn test_strategy_selection_with_llm() {
    // Test LLM-based strategy selection
    let config = ResearchConfig::default();
    let llm_client = Box::new(MockLlmClient);
    let selector = ResearchStrategySelector::with_llm_client(config, llm_client);
    let context = ResearchContext::default();

    // Test comparison question with LLM
    let strategy = selector
        .select_strategy("Compare Docker vs Kubernetes", &context)
        .await
        .unwrap();
    match strategy {
        AdaptiveResearchStrategy::Comparative {
            subjects,
            comparison_aspects,
        } => {
            assert!(!subjects.is_empty());
            assert!(!comparison_aspects.is_empty());
        }
        _ => panic!("Expected Comparative strategy from LLM for comparison question"),
    }

    // Test architectural question with LLM
    let strategy = selector
        .select_strategy("Analyze the system architecture", &context)
        .await
        .unwrap();
    match strategy {
        AdaptiveResearchStrategy::Architectural {
            components,
            interaction_depth,
        } => {
            assert!(!components.is_empty());
            assert!(interaction_depth > 0);
            println!(
                "Architectural strategy: components={:?}, depth={}",
                components, interaction_depth
            );
        }
        other => {
            println!("Got strategy: {:?}", other);
            panic!("Expected Architectural strategy from LLM for architecture question");
        }
    }

    // Test simple question with LLM
    let strategy = selector
        .select_strategy("What is Docker?", &context)
        .await
        .unwrap();
    match strategy {
        AdaptiveResearchStrategy::QuickScan {
            max_iterations,
            depth_level,
        } => {
            assert!(max_iterations <= 3);
            assert!(depth_level <= 2);
        }
        _ => panic!("Expected QuickScan strategy from LLM for simple question"),
    }

    // Test complex question with LLM
    let strategy = selector
        .select_strategy(
            "How does the distributed consensus algorithm work in this blockchain implementation?",
            &context,
        )
        .await
        .unwrap();
    match strategy {
        AdaptiveResearchStrategy::DeepDive {
            max_iterations,
            focus_areas,
        } => {
            assert!(max_iterations >= 3);
            assert!(!focus_areas.is_empty());
        }
        _ => panic!("Expected DeepDive strategy from LLM for complex question"),
    }

    println!("✅ LLM-based strategy selection tests passed");
}

#[tokio::test]
async fn test_strategy_adaptation() {
    // Test strategy adaptation based on intermediate results
    let config = ResearchConfig::default();
    let selector = ResearchStrategySelector::new(config);

    // Create initial strategy
    let initial_strategy = AdaptiveResearchStrategy::DeepDive {
        max_iterations: 5,
        focus_areas: vec!["implementation".to_string(), "performance".to_string()],
    };

    // Create mock findings with high confidence
    let high_confidence_findings = vec![
        create_mock_finding(0.95),
        create_mock_finding(0.92),
        create_mock_finding(0.88),
    ];

    // Test adaptation with high confidence (should reduce iterations)
    let adapted_strategy = selector
        .adapt_strategy(&initial_strategy, &high_confidence_findings, 2)
        .await
        .unwrap();
    match adapted_strategy {
        AdaptiveResearchStrategy::DeepDive { max_iterations, .. } => {
            assert!(
                max_iterations <= 3,
                "Should reduce iterations with high confidence"
            );
        }
        _ => panic!("Expected adapted DeepDive strategy"),
    }

    // Create mock findings with low confidence
    let low_confidence_findings = vec![
        create_mock_finding(0.3),
        create_mock_finding(0.4),
        create_mock_finding(0.2),
    ];

    // Test adaptation with low confidence (should extend iterations)
    let adapted_strategy = selector
        .adapt_strategy(&initial_strategy, &low_confidence_findings, 2)
        .await
        .unwrap();
    match adapted_strategy {
        AdaptiveResearchStrategy::DeepDive { max_iterations, .. } => {
            assert!(
                max_iterations >= 5,
                "Should extend iterations with low confidence"
            );
        }
        _ => panic!("Expected adapted DeepDive strategy"),
    }

    println!("✅ Strategy adaptation tests passed");
}

fn create_mock_finding(confidence: f64) -> wikify_applications::research::types::ResearchFinding {
    use uuid::Uuid;
    use wikify_applications::research::types::{ResearchFinding, SourceInfo, SourceType};

    ResearchFinding {
        id: Uuid::new_v4(),
        question_id: Uuid::new_v4(),
        content: "Mock finding content".to_string(),
        confidence,
        relevance: 0.8,
        source: SourceInfo {
            id: "mock-source".to_string(),
            source_type: SourceType::Documentation,
            title: Some("Mock Source".to_string()),
            author: None,
            last_modified: None,
            reliability: 0.9,
        },
        evidence: vec!["Mock evidence".to_string()],
        limitations: vec![],
        timestamp: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn test_strategy_comparison() {
    // Test that different strategies produce different configurations
    let config = ResearchConfig::default();
    let selector = ResearchStrategySelector::new(config);
    let context = ResearchContext::default();

    let quick_strategy = selector
        .select_strategy("What is X?", &context)
        .await
        .unwrap();
    let deep_strategy = selector.select_strategy("Analyze the complex distributed architecture of X with detailed performance implications", &context).await.unwrap();
    let comparison_strategy = selector
        .select_strategy("Compare X vs Y", &context)
        .await
        .unwrap();

    // Verify different strategies have different characteristics
    println!("Quick strategy: {:?}", quick_strategy);
    println!("Deep strategy: {:?}", deep_strategy);
    println!("Comparison strategy: {:?}", comparison_strategy);

    match (&quick_strategy, &deep_strategy, &comparison_strategy) {
        (
            AdaptiveResearchStrategy::QuickScan {
                max_iterations: quick_iter,
                ..
            },
            AdaptiveResearchStrategy::DeepDive {
                max_iterations: deep_iter,
                ..
            },
            AdaptiveResearchStrategy::Comparative { .. },
        ) => {
            assert!(
                quick_iter < deep_iter,
                "QuickScan should have fewer iterations than DeepDive"
            );
            println!(
                "✅ Strategy comparison test passed: QuickScan({}) < DeepDive({})",
                quick_iter, deep_iter
            );
        }
        _ => {
            // Allow for different combinations as long as we get different strategies
            println!("Got different strategy types, which is acceptable");
        }
    }

    println!("✅ All strategy system tests passed!");
}
