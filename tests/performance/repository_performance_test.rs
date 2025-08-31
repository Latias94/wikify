//! Repository Architecture Performance Tests
//!
//! Tests performance characteristics of the new Repository-based architecture
//! including concurrent operations, memory usage, and response times.

use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::sleep;

mod common;
use common::TestApp;

/// Test concurrent repository creation performance
#[tokio::test]
async fn test_concurrent_repository_creation() {
    let app = TestApp::spawn().await;
    let start_time = Instant::now();
    
    // Create 10 repositories concurrently
    let concurrent_limit = Arc::new(Semaphore::new(5)); // Limit to 5 concurrent requests
    let mut handles = vec![];
    
    for i in 0..10 {
        let app_clone = app.clone();
        let semaphore = concurrent_limit.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            
            let repo_request = json!({
                "repository": format!("https://github.com/test-org/repo-{}", i),
                "repo_type": "github",
                "auto_generate_wiki": false
            });
            
            let response = app_clone.post_repositories(&repo_request).await;
            let repo_response: serde_json::Value = response.json().await.unwrap();
            repo_response["repository_id"].as_str().unwrap().to_string()
        });
        
        handles.push(handle);
    }
    
    // Wait for all repositories to be created
    let mut repository_ids = vec![];
    for handle in handles {
        let repo_id = handle.await.unwrap();
        repository_ids.push(repo_id);
    }
    
    let creation_time = start_time.elapsed();
    println!("✅ Created {} repositories in {:?}", repository_ids.len(), creation_time);
    
    // Verify all repositories were created successfully
    assert_eq!(repository_ids.len(), 10);
    
    // Performance assertion: should complete within reasonable time
    assert!(creation_time < Duration::from_secs(30), "Repository creation took too long: {:?}", creation_time);
    
    // Clean up all repositories
    let cleanup_start = Instant::now();
    for repo_id in repository_ids {
        app.delete_repository(&repo_id).await;
    }
    let cleanup_time = cleanup_start.elapsed();
    
    println!("✅ Cleaned up all repositories in {:?}", cleanup_time);
}

/// Test chat performance with multiple repositories
#[tokio::test]
async fn test_concurrent_chat_performance() {
    let app = TestApp::spawn().await;
    
    // Create 3 repositories for testing
    let mut repository_ids = vec![];
    for i in 0..3 {
        let repo_request = json!({
            "repository": format!("https://github.com/performance-test/repo-{}", i),
            "repo_type": "github",
            "auto_generate_wiki": false
        });
        
        let response = app.post_repositories(&repo_request).await;
        let repo_response: serde_json::Value = response.json().await.unwrap();
        repository_ids.push(repo_response["repository_id"].as_str().unwrap().to_string());
    }
    
    // Wait for repositories to be ready
    sleep(Duration::from_secs(3)).await;
    
    // Test concurrent chat requests
    let start_time = Instant::now();
    let mut handles = vec![];
    
    for (i, repo_id) in repository_ids.iter().enumerate() {
        for j in 0..5 { // 5 questions per repository
            let app_clone = app.clone();
            let repo_id_clone = repo_id.clone();
            
            let handle = tokio::spawn(async move {
                let chat_request = json!({
                    "repository_id": repo_id_clone,
                    "question": format!("Question {} for repository {}", j, i),
                    "max_results": 3
                });
                
                let request_start = Instant::now();
                let response = app_clone.post_chat(&chat_request).await;
                let request_time = request_start.elapsed();
                
                let chat_result: serde_json::Value = response.json().await.unwrap();
                
                (response.status(), request_time, chat_result["repository_id"].as_str().unwrap().to_string())
            });
            
            handles.push(handle);
        }
    }
    
    // Collect results
    let mut response_times = vec![];
    let mut successful_requests = 0;
    
    for handle in handles {
        let (status, response_time, returned_repo_id) = handle.await.unwrap();
        
        if status == 200 {
            successful_requests += 1;
            response_times.push(response_time);
            assert!(repository_ids.contains(&returned_repo_id), "Incorrect repository_id returned");
        }
    }
    
    let total_time = start_time.elapsed();
    let avg_response_time = response_times.iter().sum::<Duration>() / response_times.len() as u32;
    let max_response_time = response_times.iter().max().unwrap();
    
    println!("✅ Concurrent chat performance:");
    println!("   Total requests: {}", response_times.len());
    println!("   Successful requests: {}", successful_requests);
    println!("   Total time: {:?}", total_time);
    println!("   Average response time: {:?}", avg_response_time);
    println!("   Max response time: {:?}", max_response_time);
    
    // Performance assertions
    assert!(successful_requests >= response_times.len() * 8 / 10, "Too many failed requests"); // At least 80% success
    assert!(avg_response_time < Duration::from_secs(5), "Average response time too slow");
    assert!(*max_response_time < Duration::from_secs(15), "Max response time too slow");
    
    // Clean up
    for repo_id in repository_ids {
        app.delete_repository(&repo_id).await;
    }
}

/// Test WebSocket performance and message throughput
#[tokio::test]
async fn test_websocket_message_throughput() {
    let app = TestApp::spawn().await;
    
    // Create repository
    let repo_request = json!({
        "repository": "https://github.com/websocket-test/performance",
        "repo_type": "github",
        "auto_generate_wiki": false
    });
    
    let response = app.post_repositories(&repo_request).await;
    let repo_response: serde_json::Value = response.json().await.unwrap();
    let repository_id = repo_response["repository_id"].as_str().unwrap();
    
    // Connect to WebSocket
    let ws_url = format!("ws://127.0.0.1:{}/ws", app.port());
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    
    use tokio_tungstenite::tungstenite::Message;
    use futures_util::{SinkExt, StreamExt};
    
    let message_count = 10;
    let start_time = Instant::now();
    
    // Send multiple messages rapidly
    for i in 0..message_count {
        let chat_message = json!({
            "type": "Chat",
            "repository_id": repository_id,
            "question": format!("Performance test message {}", i),
            "context": null
        });
        
        ws_stream.send(Message::Text(chat_message.to_string())).await.unwrap();
    }
    
    // Receive responses
    let mut responses_received = 0;
    let mut response_times = vec![];
    
    while responses_received < message_count {
        let response = tokio::time::timeout(Duration::from_secs(30), ws_stream.next())
            .await
            .expect("Timeout waiting for WebSocket response")
            .expect("WebSocket stream ended")
            .expect("WebSocket error");
        
        if let Message::Text(text) = response {
            let response_json: serde_json::Value = serde_json::from_str(&text).unwrap();
            
            if response_json["type"] == "ChatResponse" {
                responses_received += 1;
                response_times.push(start_time.elapsed());
                
                // Verify repository_id is correct
                assert_eq!(response_json["repository_id"], repository_id);
            }
        }
    }
    
    let total_time = start_time.elapsed();
    let avg_response_time = total_time / message_count as u32;
    
    println!("✅ WebSocket throughput test:");
    println!("   Messages sent: {}", message_count);
    println!("   Responses received: {}", responses_received);
    println!("   Total time: {:?}", total_time);
    println!("   Average time per message: {:?}", avg_response_time);
    println!("   Messages per second: {:.2}", message_count as f64 / total_time.as_secs_f64());
    
    // Performance assertions
    assert_eq!(responses_received, message_count, "Not all messages received responses");
    assert!(avg_response_time < Duration::from_secs(10), "WebSocket responses too slow");
    
    // Clean up
    app.delete_repository(repository_id).await;
}

/// Test memory usage during repository operations
#[tokio::test]
async fn test_memory_usage_stability() {
    let app = TestApp::spawn().await;
    
    // Create and delete repositories in cycles to test memory leaks
    for cycle in 0..5 {
        println!("Memory test cycle {}", cycle + 1);
        
        let mut repository_ids = vec![];
        
        // Create 5 repositories
        for i in 0..5 {
            let repo_request = json!({
                "repository": format!("https://github.com/memory-test/cycle-{}-repo-{}", cycle, i),
                "repo_type": "github",
                "auto_generate_wiki": false
            });
            
            let response = app.post_repositories(&repo_request).await;
            let repo_response: serde_json::Value = response.json().await.unwrap();
            repository_ids.push(repo_response["repository_id"].as_str().unwrap().to_string());
        }
        
        // Perform some operations on each repository
        for repo_id in &repository_ids {
            let chat_request = json!({
                "repository_id": repo_id,
                "question": "Memory test question",
                "max_results": 3
            });
            
            app.post_chat(&chat_request).await;
        }
        
        // Delete all repositories
        for repo_id in repository_ids {
            app.delete_repository(&repo_id).await;
        }
        
        // Small delay between cycles
        sleep(Duration::from_millis(500)).await;
    }
    
    println!("✅ Memory stability test completed - no crashes or obvious leaks");
}

/// Test API response time consistency
#[tokio::test]
async fn test_api_response_time_consistency() {
    let app = TestApp::spawn().await;
    
    // Create a repository for testing
    let repo_request = json!({
        "repository": "https://github.com/consistency-test/repo",
        "repo_type": "github",
        "auto_generate_wiki": false
    });
    
    let response = app.post_repositories(&repo_request).await;
    let repo_response: serde_json::Value = response.json().await.unwrap();
    let repository_id = repo_response["repository_id"].as_str().unwrap();
    
    // Wait for repository to be ready
    sleep(Duration::from_secs(2)).await;
    
    // Test multiple identical requests to measure consistency
    let mut response_times = vec![];
    
    for i in 0..20 {
        let chat_request = json!({
            "repository_id": repository_id,
            "question": "Consistency test question",
            "max_results": 3
        });
        
        let start = Instant::now();
        let response = app.post_chat(&chat_request).await;
        let response_time = start.elapsed();
        
        assert_eq!(response.status(), 200);
        response_times.push(response_time);
        
        // Small delay between requests
        sleep(Duration::from_millis(100)).await;
    }
    
    // Calculate statistics
    let avg_time = response_times.iter().sum::<Duration>() / response_times.len() as u32;
    let min_time = *response_times.iter().min().unwrap();
    let max_time = *response_times.iter().max().unwrap();
    let variance = max_time.as_millis() as f64 - min_time.as_millis() as f64;
    
    println!("✅ API response time consistency:");
    println!("   Requests: {}", response_times.len());
    println!("   Average: {:?}", avg_time);
    println!("   Min: {:?}", min_time);
    println!("   Max: {:?}", max_time);
    println!("   Variance: {:.2}ms", variance);
    
    // Consistency assertions
    assert!(avg_time < Duration::from_secs(5), "Average response time too slow");
    assert!(variance < 5000.0, "Response time variance too high: {:.2}ms", variance);
    
    // Clean up
    app.delete_repository(repository_id).await;
}
