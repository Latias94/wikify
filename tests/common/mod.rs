//! Common test utilities for Repository Architecture tests
//!
//! Provides shared testing infrastructure including test app setup,
//! helper functions, and common assertions.

use reqwest::{Client, Response};
use serde_json::Value;
use std::sync::Once;
use std::time::Duration;
use tokio::time::sleep;
use tracing_subscriber;

static INIT: Once = Once::new();

/// Initialize logging for tests
pub fn init_logging() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("wikify=debug,info")
            .with_test_writer()
            .init();
    });
}

/// Test application wrapper providing convenient API methods
#[derive(Clone)]
pub struct TestApp {
    client: Client,
    base_url: String,
    port: u16,
}

impl TestApp {
    /// Spawn a new test application instance
    pub async fn spawn() -> Self {
        init_logging();
        
        // For now, assume the application is running on localhost:8080
        // In a real test setup, you would spawn the actual application
        let port = 8080;
        let base_url = format!("http://127.0.0.1:{}", port);
        
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        let app = Self {
            client,
            base_url,
            port,
        };
        
        // Wait for the application to be ready
        app.wait_for_ready().await;
        
        app
    }
    
    /// Get the port the test app is running on
    pub fn port(&self) -> u16 {
        self.port
    }
    
    /// Wait for the application to be ready
    async fn wait_for_ready(&self) {
        let max_attempts = 30;
        let mut attempts = 0;
        
        while attempts < max_attempts {
            if let Ok(response) = self.client.get(&format!("{}/health", self.base_url)).send().await {
                if response.status().is_success() {
                    return;
                }
            }
            
            attempts += 1;
            sleep(Duration::from_millis(1000)).await;
        }
        
        panic!("Test application failed to start within {} seconds", max_attempts);
    }
    
    // Repository API methods
    
    /// GET /api/repositories
    pub async fn get_repositories(&self) -> Response {
        self.client
            .get(&format!("{}/api/repositories", self.base_url))
            .send()
            .await
            .expect("Failed to send request")
    }
    
    /// POST /api/repositories
    pub async fn post_repositories(&self, body: &Value) -> Response {
        self.client
            .post(&format!("{}/api/repositories", self.base_url))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .expect("Failed to send request")
    }
    
    /// GET /api/repositories/{id}
    pub async fn get_repository_info(&self, repository_id: &str) -> Response {
        self.client
            .get(&format!("{}/api/repositories/{}", self.base_url, repository_id))
            .send()
            .await
            .expect("Failed to send request")
    }
    
    /// DELETE /api/repositories/{id}
    pub async fn delete_repository(&self, repository_id: &str) -> Response {
        self.client
            .delete(&format!("{}/api/repositories/{}", self.base_url, repository_id))
            .send()
            .await
            .expect("Failed to send request")
    }
    
    /// POST /api/repositories/{id}/reindex
    pub async fn post_reindex_repository(&self, repository_id: &str) -> Response {
        self.client
            .post(&format!("{}/api/repositories/{}/reindex", self.base_url, repository_id))
            .send()
            .await
            .expect("Failed to send request")
    }
    
    // Chat API methods
    
    /// POST /api/chat
    pub async fn post_chat(&self, body: &Value) -> Response {
        self.client
            .post(&format!("{}/api/chat", self.base_url))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .expect("Failed to send request")
    }
    
    // Wiki API methods
    
    /// GET /api/wiki/{repository_id}
    pub async fn get_wiki(&self, repository_id: &str) -> Response {
        self.client
            .get(&format!("{}/api/wiki/{}", self.base_url, repository_id))
            .send()
            .await
            .expect("Failed to send request")
    }
    
    /// POST /api/wiki/{repository_id}/export
    pub async fn post_export_wiki(&self, repository_id: &str, body: &Value) -> Response {
        self.client
            .post(&format!("{}/api/wiki/{}/export", self.base_url, repository_id))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .expect("Failed to send request")
    }
    
    // Research API methods
    
    /// POST /api/research/iterate/{repository_id}
    pub async fn post_research_iteration(&self, repository_id: &str) -> Response {
        self.client
            .post(&format!("{}/api/research/iterate/{}", self.base_url, repository_id))
            .send()
            .await
            .expect("Failed to send request")
    }
    
    /// GET /api/research/progress/{repository_id}
    pub async fn get_research_progress(&self, repository_id: &str) -> Response {
        self.client
            .get(&format!("{}/api/research/progress/{}", self.base_url, repository_id))
            .send()
            .await
            .expect("Failed to send request")
    }
    
    /// GET /api/research/history/{repository_id}
    pub async fn get_research_history(&self, repository_id: &str) -> Response {
        self.client
            .get(&format!("{}/api/research/history/{}", self.base_url, repository_id))
            .send()
            .await
            .expect("Failed to send request")
    }
    
    /// DELETE /api/research/history/{repository_id}
    pub async fn delete_research_history(&self, repository_id: &str) -> Response {
        self.client
            .delete(&format!("{}/api/research/history/{}", self.base_url, repository_id))
            .send()
            .await
            .expect("Failed to send request")
    }
}

/// Helper functions for common test operations

/// Create a test repository and return its ID
pub async fn create_test_repository(app: &TestApp, name: &str) -> String {
    let repo_request = serde_json::json!({
        "repository": format!("https://github.com/test-org/{}", name),
        "repo_type": "github",
        "auto_generate_wiki": false
    });
    
    let response = app.post_repositories(&repo_request).await;
    assert_eq!(response.status(), 200, "Failed to create test repository");
    
    let repo_response: Value = response.json().await.expect("Failed to parse repository response");
    repo_response["repository_id"].as_str().unwrap().to_string()
}

/// Wait for a repository to be ready for querying
pub async fn wait_for_repository_ready(app: &TestApp, repository_id: &str, max_wait_secs: u64) -> bool {
    let max_attempts = max_wait_secs;
    let mut attempts = 0;
    
    while attempts < max_attempts {
        let info_response = app.get_repository_info(repository_id).await;
        if info_response.status() == 200 {
            let info: Value = info_response.json().await.expect("Failed to parse repository info");
            if let Some(status) = info.get("status") {
                if status == "ready" || status == "indexed" {
                    return true;
                }
            }
        }
        
        attempts += 1;
        sleep(Duration::from_secs(1)).await;
    }
    
    false
}

/// Send a test chat message and verify the response format
pub async fn test_chat_message(app: &TestApp, repository_id: &str, question: &str) -> Value {
    let chat_request = serde_json::json!({
        "repository_id": repository_id,
        "question": question,
        "max_results": 3
    });
    
    let response = app.post_chat(&chat_request).await;
    assert_eq!(response.status(), 200, "Chat request failed");
    
    let chat_result: Value = response.json().await.expect("Failed to parse chat response");
    
    // Verify response format
    assert_eq!(chat_result["repository_id"], repository_id, "Incorrect repository_id in response");
    assert!(chat_result["answer"].is_string(), "Answer should be a string");
    assert!(chat_result["sources"].is_array(), "Sources should be an array");
    assert!(chat_result["timestamp"].is_string(), "Timestamp should be a string");
    
    chat_result
}

/// Clean up test repositories
pub async fn cleanup_repositories(app: &TestApp, repository_ids: &[String]) {
    for repo_id in repository_ids {
        let response = app.delete_repository(repo_id).await;
        if response.status() != 200 {
            eprintln!("Warning: Failed to delete repository {}", repo_id);
        }
    }
}

/// Assert that a response contains the expected repository_id
pub fn assert_repository_id_in_response(response: &Value, expected_repo_id: &str) {
    assert_eq!(
        response["repository_id"].as_str().unwrap(),
        expected_repo_id,
        "Response does not contain expected repository_id"
    );
}

/// Assert that a chat response has the expected structure
pub fn assert_valid_chat_response(response: &Value, repository_id: &str) {
    assert_repository_id_in_response(response, repository_id);
    assert!(response["answer"].is_string(), "Chat response missing answer");
    assert!(response["sources"].is_array(), "Chat response missing sources");
    assert!(response["timestamp"].is_string(), "Chat response missing timestamp");
}

/// Generate a unique test repository name
pub fn generate_test_repo_name(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}-{}", prefix, timestamp)
}

/// Test data generators

pub struct TestRepositories;

impl TestRepositories {
    pub fn rust_projects() -> Vec<(&'static str, &'static str)> {
        vec![
            ("https://github.com/rust-lang/rust", "The Rust Programming Language"),
            ("https://github.com/tokio-rs/tokio", "Async runtime for Rust"),
            ("https://github.com/serde-rs/serde", "Serialization framework"),
            ("https://github.com/clap-rs/clap", "Command line argument parser"),
            ("https://github.com/rust-lang/cargo", "Rust package manager"),
        ]
    }
    
    pub fn javascript_projects() -> Vec<(&'static str, &'static str)> {
        vec![
            ("https://github.com/microsoft/vscode", "Visual Studio Code"),
            ("https://github.com/facebook/react", "React JavaScript library"),
            ("https://github.com/nodejs/node", "Node.js runtime"),
            ("https://github.com/microsoft/TypeScript", "TypeScript language"),
            ("https://github.com/webpack/webpack", "Module bundler"),
        ]
    }
    
    pub fn test_questions() -> Vec<&'static str> {
        vec![
            "What is this repository about?",
            "How do I get started?",
            "What are the main features?",
            "What programming language is used?",
            "How do I contribute to this project?",
            "What are the system requirements?",
            "How do I install this software?",
            "What is the license?",
            "Who maintains this project?",
            "What are the recent changes?",
        ]
    }
}
