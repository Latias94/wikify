//! User Workflow Scenario Tests
//!
//! Tests realistic user workflows with the new Repository-based architecture
//! to ensure the system works as expected from a user perspective.

use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

mod common;
use common::TestApp;

/// Test: New user discovers and explores a repository
#[tokio::test]
async fn test_new_user_repository_discovery() {
    let app = TestApp::spawn().await;
    
    println!("🧪 Scenario: New user discovers and explores a repository");
    
    // Step 1: User adds a new repository they want to explore
    println!("📝 Step 1: User adds a repository");
    let repo_request = json!({
        "repository": "https://github.com/rust-lang/mdBook",
        "repo_type": "github",
        "auto_generate_wiki": true  // User wants automatic wiki generation
    });
    
    let response = app.post_repositories(&repo_request).await;
    assert_eq!(response.status(), 200);
    
    let repo_response: serde_json::Value = response.json().await.unwrap();
    let repository_id = repo_response["repository_id"].as_str().unwrap();
    
    println!("   ✅ Repository added: {}", repository_id);
    
    // Step 2: User checks repository status while indexing
    println!("📝 Step 2: User checks repository status");
    let info_response = app.get_repository_info(repository_id).await;
    assert_eq!(info_response.status(), 200);
    
    let info: serde_json::Value = info_response.json().await.unwrap();
    println!("   ✅ Repository status: {:?}", info.get("status"));
    
    // Step 3: User asks initial exploratory questions
    println!("📝 Step 3: User asks exploratory questions");
    let questions = vec![
        "What is this repository about?",
        "What programming language is used?",
        "How do I get started with this project?",
        "What are the main features?",
    ];
    
    for (i, question) in questions.iter().enumerate() {
        let chat_request = json!({
            "repository_id": repository_id,
            "question": question,
            "max_results": 5
        });
        
        let chat_response = app.post_chat(&chat_request).await;
        assert_eq!(chat_response.status(), 200);
        
        let chat_result: serde_json::Value = chat_response.json().await.unwrap();
        assert_eq!(chat_result["repository_id"], repository_id);
        assert!(chat_result["answer"].is_string());
        
        println!("   ✅ Question {}: {}", i + 1, question);
        println!("      Answer length: {} chars", chat_result["answer"].as_str().unwrap().len());
        
        // Small delay between questions (realistic user behavior)
        sleep(Duration::from_millis(500)).await;
    }
    
    // Step 4: User tries to access wiki (might not be ready yet)
    println!("📝 Step 4: User checks for generated wiki");
    let wiki_response = app.get_wiki(repository_id).await;
    if wiki_response.status() == 200 {
        println!("   ✅ Wiki is available");
    } else {
        println!("   ℹ️  Wiki not ready yet (status: {})", wiki_response.status());
    }
    
    // Step 5: User lists all their repositories
    println!("📝 Step 5: User lists all repositories");
    let list_response = app.get_repositories().await;
    assert_eq!(list_response.status(), 200);
    
    let repositories: serde_json::Value = list_response.json().await.unwrap();
    assert!(repositories["repositories"].is_array());
    
    let repo_count = repositories["repositories"].as_array().unwrap().len();
    println!("   ✅ User has {} repositories", repo_count);
    
    // Clean up
    app.delete_repository(repository_id).await;
    println!("🎉 Scenario completed successfully");
}

/// Test: Developer working with multiple projects
#[tokio::test]
async fn test_developer_multi_project_workflow() {
    let app = TestApp::spawn().await;
    
    println!("🧪 Scenario: Developer working with multiple projects");
    
    // Step 1: Developer adds multiple repositories for different projects
    println!("📝 Step 1: Developer adds multiple repositories");
    let repositories = vec![
        ("https://github.com/tokio-rs/tokio", "Async runtime"),
        ("https://github.com/serde-rs/serde", "Serialization library"),
        ("https://github.com/clap-rs/clap", "CLI argument parser"),
    ];
    
    let mut repo_ids = vec![];
    for (repo_url, description) in repositories {
        let repo_request = json!({
            "repository": repo_url,
            "repo_type": "github",
            "auto_generate_wiki": false
        });
        
        let response = app.post_repositories(&repo_request).await;
        let repo_response: serde_json::Value = response.json().await.unwrap();
        let repo_id = repo_response["repository_id"].as_str().unwrap().to_string();
        
        repo_ids.push((repo_id, description));
        println!("   ✅ Added {}: {}", description, repo_url);
    }
    
    // Step 2: Developer asks specific questions about each project
    println!("📝 Step 2: Developer asks project-specific questions");
    let project_questions = vec![
        ("How do I create a tokio runtime?", 0),
        ("What are the main serde traits?", 1),
        ("How do I define command line arguments with clap?", 2),
        ("What's the difference between spawn and spawn_blocking?", 0),
        ("How do I serialize a custom struct?", 1),
    ];
    
    for (question, project_idx) in project_questions {
        let (repo_id, project_name) = &repo_ids[project_idx];
        
        let chat_request = json!({
            "repository_id": repo_id,
            "question": question,
            "max_results": 3
        });
        
        let chat_response = app.post_chat(&chat_request).await;
        assert_eq!(chat_response.status(), 200);
        
        let chat_result: serde_json::Value = chat_response.json().await.unwrap();
        assert_eq!(chat_result["repository_id"], repo_id);
        
        println!("   ✅ Asked {} about {}: {}", project_name, question, 
                 if chat_result["answer"].as_str().unwrap().len() > 50 { "Got detailed answer" } else { "Got short answer" });
        
        sleep(Duration::from_millis(300)).await;
    }
    
    // Step 3: Developer compares features across projects
    println!("📝 Step 3: Developer compares features across projects");
    let comparison_question = "What are the main features and use cases?";
    
    for (repo_id, project_name) in &repo_ids {
        let chat_request = json!({
            "repository_id": repo_id,
            "question": comparison_question,
            "max_results": 5
        });
        
        let chat_response = app.post_chat(&chat_request).await;
        let chat_result: serde_json::Value = chat_response.json().await.unwrap();
        
        println!("   ✅ Got features overview for {}", project_name);
        assert_eq!(chat_result["repository_id"], repo_id);
    }
    
    // Step 4: Developer removes one project they no longer need
    println!("📝 Step 4: Developer removes unused project");
    let (repo_to_remove, project_name) = &repo_ids[2]; // Remove clap
    
    let delete_response = app.delete_repository(repo_to_remove).await;
    assert_eq!(delete_response.status(), 200);
    println!("   ✅ Removed {}", project_name);
    
    // Step 5: Verify remaining repositories still work
    println!("📝 Step 5: Verify remaining repositories still work");
    for (repo_id, project_name) in &repo_ids[0..2] {
        let chat_request = json!({
            "repository_id": repo_id,
            "question": "Is this project still active?",
            "max_results": 3
        });
        
        let chat_response = app.post_chat(&chat_request).await;
        assert_eq!(chat_response.status(), 200);
        
        let chat_result: serde_json::Value = chat_response.json().await.unwrap();
        assert_eq!(chat_result["repository_id"], repo_id);
        
        println!("   ✅ {} still working correctly", project_name);
    }
    
    // Clean up remaining repositories
    for (repo_id, _) in &repo_ids[0..2] {
        app.delete_repository(repo_id).await;
    }
    
    println!("🎉 Multi-project workflow completed successfully");
}

/// Test: Real-time collaboration scenario
#[tokio::test]
async fn test_realtime_collaboration_scenario() {
    let app = TestApp::spawn().await;
    
    println!("🧪 Scenario: Real-time collaboration with WebSocket");
    
    // Step 1: Set up shared repository
    println!("📝 Step 1: Set up shared repository");
    let repo_request = json!({
        "repository": "https://github.com/microsoft/vscode",
        "repo_type": "github",
        "auto_generate_wiki": false
    });
    
    let response = app.post_repositories(&repo_request).await;
    let repo_response: serde_json::Value = response.json().await.unwrap();
    let repository_id = repo_response["repository_id"].as_str().unwrap();
    
    // Step 2: Multiple users connect via WebSocket
    println!("📝 Step 2: Multiple users connect via WebSocket");
    let ws_url = format!("ws://127.0.0.1:{}/ws", app.port());
    
    let (mut ws1, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
    let (mut ws2, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
    
    use tokio_tungstenite::tungstenite::Message;
    use futures_util::{SinkExt, StreamExt};
    
    // Step 3: Users ask questions simultaneously
    println!("📝 Step 3: Users ask questions simultaneously");
    let user1_question = json!({
        "type": "Chat",
        "repository_id": repository_id,
        "question": "What is VS Code written in?",
        "context": null
    });
    
    let user2_question = json!({
        "type": "Chat", 
        "repository_id": repository_id,
        "question": "How do I create a VS Code extension?",
        "context": null
    });
    
    // Send questions simultaneously
    let send_task1 = ws1.send(Message::Text(user1_question.to_string()));
    let send_task2 = ws2.send(Message::Text(user2_question.to_string()));
    
    tokio::try_join!(send_task1, send_task2).unwrap();
    
    // Step 4: Receive responses
    println!("📝 Step 4: Users receive responses");
    let mut responses_received = 0;
    let timeout_duration = Duration::from_secs(20);
    
    // Collect responses from both WebSockets
    let response1_task = tokio::time::timeout(timeout_duration, ws1.next());
    let response2_task = tokio::time::timeout(timeout_duration, ws2.next());
    
    let (response1_result, response2_result) = tokio::join!(response1_task, response2_task);
    
    // Verify first user's response
    if let Ok(Some(Ok(Message::Text(text1)))) = response1_result {
        let response_json: serde_json::Value = serde_json::from_str(&text1).unwrap();
        if response_json["type"] == "ChatResponse" {
            assert_eq!(response_json["repository_id"], repository_id);
            responses_received += 1;
            println!("   ✅ User 1 received response");
        }
    }
    
    // Verify second user's response
    if let Ok(Some(Ok(Message::Text(text2)))) = response2_result {
        let response_json: serde_json::Value = serde_json::from_str(&text2).unwrap();
        if response_json["type"] == "ChatResponse" {
            assert_eq!(response_json["repository_id"], repository_id);
            responses_received += 1;
            println!("   ✅ User 2 received response");
        }
    }
    
    assert_eq!(responses_received, 2, "Both users should receive responses");
    
    // Clean up
    app.delete_repository(repository_id).await;
    println!("🎉 Real-time collaboration scenario completed successfully");
}

/// Test: Error recovery and resilience
#[tokio::test]
async fn test_error_recovery_scenario() {
    let app = TestApp::spawn().await;
    
    println!("🧪 Scenario: Error recovery and system resilience");
    
    // Step 1: User tries to access non-existent repository
    println!("📝 Step 1: User tries invalid repository");
    let invalid_repo_id = "00000000-0000-0000-0000-000000000000";
    
    let chat_request = json!({
        "repository_id": invalid_repo_id,
        "question": "This should fail gracefully",
        "max_results": 3
    });
    
    let chat_response = app.post_chat(&chat_request).await;
    assert!(chat_response.status() == 404 || chat_response.status() == 400);
    println!("   ✅ Invalid repository handled gracefully");
    
    // Step 2: User creates repository with invalid URL
    println!("📝 Step 2: User tries invalid repository URL");
    let invalid_repo_request = json!({
        "repository": "not-a-valid-url",
        "repo_type": "github",
        "auto_generate_wiki": false
    });
    
    let invalid_response = app.post_repositories(&invalid_repo_request).await;
    assert!(invalid_response.status() >= 400);
    println!("   ✅ Invalid repository URL rejected");
    
    // Step 3: User creates valid repository and system recovers
    println!("📝 Step 3: User creates valid repository after errors");
    let valid_repo_request = json!({
        "repository": "https://github.com/rust-lang/cargo",
        "repo_type": "github",
        "auto_generate_wiki": false
    });
    
    let valid_response = app.post_repositories(&valid_repo_request).await;
    assert_eq!(valid_response.status(), 200);
    
    let repo_response: serde_json::Value = valid_response.json().await.unwrap();
    let repository_id = repo_response["repository_id"].as_str().unwrap();
    
    println!("   ✅ System recovered, valid repository created");
    
    // Step 4: User successfully uses the valid repository
    println!("📝 Step 4: User successfully uses valid repository");
    let success_chat_request = json!({
        "repository_id": repository_id,
        "question": "What is Cargo?",
        "max_results": 3
    });
    
    let success_response = app.post_chat(&success_chat_request).await;
    assert_eq!(success_response.status(), 200);
    
    let success_result: serde_json::Value = success_response.json().await.unwrap();
    assert_eq!(success_result["repository_id"], repository_id);
    
    println!("   ✅ Repository working correctly after error recovery");
    
    // Clean up
    app.delete_repository(repository_id).await;
    println!("🎉 Error recovery scenario completed successfully");
}
