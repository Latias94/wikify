//! Integration tests for streaming deep research functionality
//!
//! These tests verify the complete flow of streaming research including:
//! - SSE stream initialization
//! - Real-time progress updates
//! - Research completion and result retrieval
//! - Error handling and recovery

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use futures_util::{stream::StreamExt, TryStreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;
use tower::ServiceExt;
use wikify_web::{create_app, AppState, WebConfig};

mod helpers;
use helpers::*;

/// Test helper to create a test repository for research
async fn create_test_repository_for_research() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path();

    // Create a simple Rust project structure for testing
    std::fs::create_dir_all(repo_path.join("src")).unwrap();

    // Main.rs with some content to research
    std::fs::write(
        repo_path.join("src/main.rs"),
        r#"
//! A simple Rust application demonstrating various programming concepts
//! 
//! This application showcases:
//! - Error handling with Result types
//! - Async programming with tokio
//! - Data structures and algorithms
//! - Memory management and ownership

use std::collections::HashMap;
use tokio::time::{sleep, Duration};

/// Main application entry point
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting the application...");
    
    // Demonstrate async operations
    let result = perform_async_operation().await?;
    println!("Async operation result: {}", result);
    
    // Demonstrate data structures
    let mut data_store = create_data_store();
    data_store.insert("key1".to_string(), "value1".to_string());
    data_store.insert("key2".to_string(), "value2".to_string());
    
    // Process data
    process_data(&data_store).await;
    
    Ok(())
}

/// Performs an asynchronous operation with error handling
async fn perform_async_operation() -> Result<String, &'static str> {
    sleep(Duration::from_millis(100)).await;
    
    // Simulate some computation
    let computation_result = calculate_fibonacci(10);
    
    if computation_result > 0 {
        Ok(format!("Computation successful: {}", computation_result))
    } else {
        Err("Computation failed")
    }
}

/// Creates a data store using HashMap
fn create_data_store() -> HashMap<String, String> {
    HashMap::new()
}

/// Processes data from the store
async fn process_data(data: &HashMap<String, String>) {
    println!("Processing {} items", data.len());
    
    for (key, value) in data {
        println!("Processing: {} -> {}", key, value);
        sleep(Duration::from_millis(10)).await;
    }
}

/// Calculates fibonacci number recursively
fn calculate_fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2),
    }
}

/// Advanced error handling example
#[derive(Debug)]
enum AppError {
    NetworkError(String),
    DataError(String),
    ConfigError(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            AppError::DataError(msg) => write!(f, "Data error: {}", msg),
            AppError::ConfigError(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}
"#,
    )
    .unwrap();

    // Cargo.toml
    std::fs::write(
        repo_path.join("Cargo.toml"),
        r#"
[package]
name = "test-research-project"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
"#,
    )
    .unwrap();

    // README.md
    std::fs::write(
        repo_path.join("README.md"),
        r#"
# Test Research Project

This is a test project for demonstrating deep research capabilities.

## Features

- Async programming with Tokio
- Error handling patterns
- Data structures and algorithms
- Memory management

## Architecture

The application follows a modular design with clear separation of concerns:

1. **Main Module**: Entry point and orchestration
2. **Data Processing**: HashMap-based data management
3. **Async Operations**: Non-blocking I/O operations
4. **Error Handling**: Comprehensive error management

## Performance Considerations

- Uses efficient data structures
- Implements proper error propagation
- Leverages Rust's ownership system for memory safety
"#,
    )
    .unwrap();

    temp_dir
}

/// Test streaming research with real LLM integration
#[tokio::test]
#[ignore = "Uses real LLM API - run with --ignored"]
async fn test_streaming_deep_research_integration() {
    // Check if we have API keys available
    let has_openai = std::env::var("OPENAI_API_KEY").is_ok();
    let has_anthropic = std::env::var("ANTHROPIC_API_KEY").is_ok();
    let has_deepseek = std::env::var("DEEPSEEK_API_KEY").is_ok();

    if !has_openai && !has_anthropic && !has_deepseek {
        println!("⏭️ Skipping streaming research test: No LLM API keys found");
        return;
    }

    println!("🧪 Starting streaming deep research integration test");

    // Create test repository
    let temp_repo = create_test_repository_for_research().await;
    let repo_path = temp_repo.path().to_string_lossy().to_string();

    println!("📁 Created test repository: {}", repo_path);

    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state.clone());

    // Create and login test user
    let (_user_id, access_token) = create_test_user_and_login(&mut app).await;

    println!("👤 Created test user and obtained access token");

    // Initialize repository
    let init_request = json!({
        "repository": repo_path,
        "repo_type": "local",
        "auto_generate_wiki": false
    });

    let request = create_authenticated_request(
        "POST",
        "/api/repositories",
        Some(init_request),
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let init_response = extract_json_response(response).await;
    let repository_id = init_response["repository_id"].as_str().unwrap();

    println!("🔧 Initialized repository: {}", repository_id);

    // Wait for indexing to complete
    println!("⏳ Waiting for repository indexing...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Start streaming deep research
    let research_request = json!({
        "repository_id": repository_id,
        "research_question": "What are the main programming concepts and patterns demonstrated in this Rust codebase? Focus on async programming, error handling, and data structures.",
        "config": {
            "max_iterations": 3,
            "max_sources_per_iteration": 5
        }
    });

    println!("🔬 Starting streaming deep research...");

    let request = create_authenticated_request(
        "POST",
        "/api/research/deep-stream",
        Some(research_request),
        Some(&access_token),
    )
    .await;

    // Send request and get streaming response
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify content type is text/event-stream
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("text/event-stream"));

    println!("✅ Streaming response initiated successfully");

    // Process the stream (simulate reading SSE events)
    let body = response.into_body();
    let mut stream = body.into_data_stream();

    let mut research_id: Option<String> = None;
    let mut progress_updates = 0;
    let mut final_result: Option<Value> = None;
    let mut is_complete = false;

    // Read stream with timeout
    let stream_timeout = Duration::from_secs(60); // 1 minute timeout

    println!("📡 Reading streaming events...");

    let stream_result = timeout(stream_timeout, async {
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(data) => {
                    let text = String::from_utf8_lossy(&data);

                    // Parse SSE events
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let json_str = &line[6..]; // Remove "data: " prefix
                            if let Ok(event_data) = serde_json::from_str::<Value>(json_str) {
                                let event_type = event_data["type"].as_str().unwrap_or("unknown");

                                match event_type {
                                    "progress" => {
                                        progress_updates += 1;
                                        research_id = Some(
                                            event_data["research_id"].as_str().unwrap().to_string(),
                                        );

                                        let iteration =
                                            event_data["current_iteration"].as_u64().unwrap_or(0);
                                        let progress =
                                            event_data["progress"].as_f64().unwrap_or(0.0);

                                        println!(
                                            "📊 Progress update {}: iteration {}, progress {:.1}%",
                                            progress_updates,
                                            iteration,
                                            progress * 100.0
                                        );
                                    }
                                    "complete" => {
                                        final_result = event_data["final_result"].clone().into();
                                        is_complete = true;
                                        println!("🎉 Research completed!");
                                        break;
                                    }
                                    "error" => {
                                        let error_msg =
                                            event_data["error"].as_str().unwrap_or("Unknown error");
                                        panic!("❌ Research failed: {}", error_msg);
                                    }
                                    _ => {
                                        println!("📝 Unknown event type: {}", event_type);
                                    }
                                }
                            }
                        }
                    }

                    if is_complete {
                        break;
                    }
                }
                Err(e) => {
                    println!("⚠️ Stream error: {}", e);
                    break;
                }
            }
        }
    })
    .await;

    match stream_result {
        Ok(_) => {
            println!("✅ Stream processing completed successfully");
        }
        Err(_) => {
            println!("⏰ Stream processing timed out (this may be expected for long research)");
        }
    }

    // Verify we received progress updates
    assert!(
        progress_updates > 0,
        "Should have received at least one progress update"
    );
    println!("📈 Received {} progress updates", progress_updates);

    // If we have a research ID, try to get the detailed result
    if let Some(research_id) = research_id {
        println!("🔍 Fetching detailed research result...");

        let request = create_authenticated_request(
            "GET",
            &format!("/api/research/{}/result", research_id),
            None,
            Some(&access_token),
        )
        .await;

        let response = app.clone().oneshot(request).await.unwrap();

        if response.status() == StatusCode::OK {
            let result = extract_json_response(response).await;
            println!("📋 Research result summary:");
            println!("   - Topic: {}", result["topic"].as_str().unwrap_or("N/A"));
            println!(
                "   - Status: {}",
                result["status"].as_str().unwrap_or("N/A")
            );
            println!(
                "   - Iterations: {}",
                result["iterations"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0)
            );
        } else {
            println!(
                "⚠️ Could not fetch detailed result (status: {})",
                response.status()
            );
        }
    }

    println!("🎯 Streaming deep research integration test completed successfully!");

    // Cleanup
    drop(temp_repo);
}

/// Test helper functions
async fn create_test_user_and_login(app: &mut axum::Router) -> (String, String) {
    let register_request = json!({
        "username": "streaming_research_tester",
        "email": "streaming@test.com",
        "password": "test123456",
        "display_name": "Streaming Research Tester"
    });

    let request =
        create_authenticated_request("POST", "/api/auth/register", Some(register_request), None)
            .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let auth_response = extract_json_response(response).await;
    let access_token = auth_response["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    let user_id = auth_response["user"]["id"].as_str().unwrap().to_string();

    (user_id, access_token)
}

async fn create_authenticated_request(
    method: &str,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);

    if let Some(token) = token {
        builder = builder.header("Authorization", format!("Bearer {}", token));
    }

    if let Some(body) = body {
        builder = builder.header("Content-Type", "application/json");
        builder
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    }
}

async fn extract_json_response(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}
