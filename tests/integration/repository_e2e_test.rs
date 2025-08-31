//! End-to-End Repository Architecture Integration Tests
//!
//! Tests the complete flow from repository creation to chat interaction
//! using the new Repository-based architecture.

use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

mod common;
use common::TestApp;

/// Test complete repository lifecycle
#[tokio::test]
async fn test_repository_lifecycle_e2e() {
    let app = TestApp::spawn().await;
    
    // Step 1: Create repository
    let repo_request = json!({
        "repository": "https://github.com/rust-lang/cargo",
        "repo_type": "github",
        "auto_generate_wiki": false
    });
    
    let response = app.post_repositories(&repo_request).await;
    assert_eq!(response.status(), 200);
    
    let repo_response: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let repository_id = repo_response["repository_id"].as_str().unwrap();
    
    println!("✅ Repository created: {}", repository_id);
    
    // Step 2: Wait for indexing to start
    sleep(Duration::from_secs(2)).await;
    
    // Step 3: Get repository info
    let info_response = app.get_repository_info(repository_id).await;
    assert_eq!(info_response.status(), 200);
    
    let info: serde_json::Value = info_response.json().await.expect("Failed to parse JSON");
    assert_eq!(info["repository_id"], repository_id);
    
    println!("✅ Repository info retrieved");
    
    // Step 4: Test chat with repository (should work even if indexing is pending)
    let chat_request = json!({
        "repository_id": repository_id,
        "question": "What is this repository about?",
        "max_results": 3
    });
    
    let chat_response = app.post_chat(&chat_request).await;
    assert_eq!(chat_response.status(), 200);
    
    let chat_result: serde_json::Value = chat_response.json().await.expect("Failed to parse JSON");
    assert_eq!(chat_result["repository_id"], repository_id);
    assert!(chat_result["answer"].is_string());
    
    println!("✅ Chat with repository successful");
    
    // Step 5: Test wiki endpoint
    let wiki_response = app.get_wiki(repository_id).await;
    // Wiki might not exist yet, so 404 is acceptable
    assert!(wiki_response.status() == 200 || wiki_response.status() == 404);
    
    println!("✅ Wiki endpoint accessible");
    
    // Step 6: Clean up - delete repository
    let delete_response = app.delete_repository(repository_id).await;
    assert_eq!(delete_response.status(), 200);
    
    println!("✅ Repository deleted successfully");
}

/// Test WebSocket chat functionality with repository_id
#[tokio::test]
async fn test_websocket_chat_with_repository_id() {
    let app = TestApp::spawn().await;
    
    // Create a repository first
    let repo_request = json!({
        "repository": "https://github.com/microsoft/vscode",
        "repo_type": "github",
        "auto_generate_wiki": false
    });
    
    let response = app.post_repositories(&repo_request).await;
    let repo_response: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let repository_id = repo_response["repository_id"].as_str().unwrap();
    
    // Test WebSocket connection and messaging
    let ws_url = format!("ws://127.0.0.1:{}/ws", app.port());
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    
    // Send chat message with repository_id
    let chat_message = json!({
        "type": "Chat",
        "repository_id": repository_id,
        "question": "What programming languages are used in this repository?",
        "context": null
    });
    
    use tokio_tungstenite::tungstenite::Message;
    ws_stream.send(Message::Text(chat_message.to_string())).await.unwrap();
    
    // Wait for response
    let response = tokio::time::timeout(Duration::from_secs(10), ws_stream.next())
        .await
        .expect("Timeout waiting for WebSocket response")
        .expect("WebSocket stream ended")
        .expect("WebSocket error");
    
    if let Message::Text(text) = response {
        let response_json: serde_json::Value = serde_json::from_str(&text).unwrap();
        
        // Verify response format
        assert_eq!(response_json["type"], "ChatResponse");
        assert_eq!(response_json["repository_id"], repository_id);
        assert!(response_json["answer"].is_string());
        assert!(response_json["sources"].is_array());
        
        println!("✅ WebSocket chat response received with correct repository_id");
    } else {
        panic!("Expected text message from WebSocket");
    }
    
    // Clean up
    app.delete_repository(repository_id).await;
}

/// Test multiple repositories isolation
#[tokio::test]
async fn test_multiple_repositories_isolation() {
    let app = TestApp::spawn().await;
    
    // Create two different repositories
    let repo1_request = json!({
        "repository": "https://github.com/rust-lang/rust",
        "repo_type": "github",
        "auto_generate_wiki": false
    });
    
    let repo2_request = json!({
        "repository": "https://github.com/microsoft/TypeScript",
        "repo_type": "github", 
        "auto_generate_wiki": false
    });
    
    let response1 = app.post_repositories(&repo1_request).await;
    let response2 = app.post_repositories(&repo2_request).await;
    
    let repo1_response: serde_json::Value = response1.json().await.unwrap();
    let repo2_response: serde_json::Value = response2.json().await.unwrap();
    
    let repo1_id = repo1_response["repository_id"].as_str().unwrap();
    let repo2_id = repo2_response["repository_id"].as_str().unwrap();
    
    assert_ne!(repo1_id, repo2_id);
    
    // Test chat with each repository separately
    let chat1_request = json!({
        "repository_id": repo1_id,
        "question": "What is Rust?",
        "max_results": 3
    });
    
    let chat2_request = json!({
        "repository_id": repo2_id,
        "question": "What is TypeScript?",
        "max_results": 3
    });
    
    let chat1_response = app.post_chat(&chat1_request).await;
    let chat2_response = app.post_chat(&chat2_request).await;
    
    let chat1_result: serde_json::Value = chat1_response.json().await.unwrap();
    let chat2_result: serde_json::Value = chat2_response.json().await.unwrap();
    
    // Verify each response has correct repository_id
    assert_eq!(chat1_result["repository_id"], repo1_id);
    assert_eq!(chat2_result["repository_id"], repo2_id);
    
    println!("✅ Multiple repositories properly isolated");
    
    // Clean up
    app.delete_repository(repo1_id).await;
    app.delete_repository(repo2_id).await;
}

/// Test error handling with invalid repository_id
#[tokio::test]
async fn test_invalid_repository_id_handling() {
    let app = TestApp::spawn().await;
    
    let invalid_repo_id = Uuid::new_v4().to_string();
    
    // Test chat with non-existent repository
    let chat_request = json!({
        "repository_id": invalid_repo_id,
        "question": "This should fail",
        "max_results": 3
    });
    
    let chat_response = app.post_chat(&chat_request).await;
    // Should return error but not crash
    assert!(chat_response.status() == 404 || chat_response.status() == 400);
    
    // Test wiki with non-existent repository
    let wiki_response = app.get_wiki(&invalid_repo_id).await;
    assert_eq!(wiki_response.status(), 404);
    
    // Test repository info with non-existent repository
    let info_response = app.get_repository_info(&invalid_repo_id).await;
    assert_eq!(info_response.status(), 404);
    
    println!("✅ Invalid repository_id properly handled");
}

/// Test repository reindexing
#[tokio::test]
async fn test_repository_reindexing() {
    let app = TestApp::spawn().await;
    
    // Create repository
    let repo_request = json!({
        "repository": "https://github.com/tokio-rs/tokio",
        "repo_type": "github",
        "auto_generate_wiki": false
    });
    
    let response = app.post_repositories(&repo_request).await;
    let repo_response: serde_json::Value = response.json().await.unwrap();
    let repository_id = repo_response["repository_id"].as_str().unwrap();
    
    // Wait a bit for initial indexing
    sleep(Duration::from_secs(2)).await;
    
    // Trigger reindexing
    let reindex_response = app.post_reindex_repository(repository_id).await;
    assert_eq!(reindex_response.status(), 200);
    
    let reindex_result: serde_json::Value = reindex_response.json().await.unwrap();
    assert_eq!(reindex_result["repository_id"], repository_id);
    assert_eq!(reindex_result["status"], "success");
    
    println!("✅ Repository reindexing successful");
    
    // Clean up
    app.delete_repository(repository_id).await;
}
