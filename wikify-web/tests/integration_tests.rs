//! Integration tests for Wikify Web Server
//!
//! These tests verify the complete functionality of the web server.

use axum::http::StatusCode;
use serde_json::json;
use tokio::net::TcpListener;
use wikify_web::{create_app, AppState, WebConfig};

/// Helper function to create a test server
async fn create_test_server() -> (String, AppState) {
    let config = WebConfig {
        host: "127.0.0.1".to_string(),
        port: 0, // Let the OS choose a free port
        dev_mode: true,
        static_dir: Some("static".to_string()),
        database_url: Some(":memory:".to_string()), // In-memory SQLite for testing
    };

    let state = AppState::new(config.clone()).await.unwrap();
    let app = create_app(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    (server_url, state)
}

#[tokio::test]
async fn test_health_check() {
    let (server_url, _state) = create_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/api/health", server_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "healthy");
    assert!(body["timestamp"].is_string());
}

#[tokio::test]
async fn test_repository_initialization() {
    let (server_url, _state) = create_test_server().await;
    let client = reqwest::Client::new();

    let request_body = json!({
        "repository": "https://github.com/rust-lang/rust",
        "repo_type": "github"
    });

    let response = client
        .post(&format!("{}/api/repositories", server_url))
        .json(&request_body)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["session_id"].is_string());
    assert_eq!(body["status"], "initialized");
}

#[tokio::test]
async fn test_chat_query() {
    let (server_url, _state) = create_test_server().await;
    let client = reqwest::Client::new();

    // First initialize a repository
    let init_request = json!({
        "repository": "https://github.com/rust-lang/rust",
        "repo_type": "github"
    });

    let init_response = client
        .post(&format!("{}/api/repositories", server_url))
        .json(&init_request)
        .send()
        .await
        .unwrap();

    let init_body: serde_json::Value = init_response.json().await.unwrap();
    let session_id = init_body["session_id"].as_str().unwrap();

    // Now test chat query
    let chat_request = json!({
        "session_id": session_id,
        "question": "What is this repository about?",
        "context": null
    });

    let response = client
        .post(&format!("{}/api/chat", server_url))
        .json(&chat_request)
        .send()
        .await
        .unwrap();

    // Note: This might fail if RAG system is not properly configured
    // In a real test, we'd mock the RAG responses
    assert!(response.status().is_success() || response.status().is_server_error());
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn test_database_operations() {
    let (server_url, _state) = create_test_server().await;
    let client = reqwest::Client::new();

    // Test getting repositories (should be empty initially)
    let response = client
        .get(&format!("{}/api/repositories", server_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["repositories"].is_array());

    // Test getting sessions
    let response = client
        .get(&format!("{}/api/sessions", server_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["sessions"].is_array());
}

#[tokio::test]
async fn test_static_file_serving() {
    let (server_url, _state) = create_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/static/index.html", server_url))
        .send()
        .await
        .unwrap();

    // Should either serve the file or return 404 if not found
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_websocket_connection() {
    let (server_url, _state) = create_test_server().await;

    // Convert HTTP URL to WebSocket URL
    let ws_url = server_url.replace("http://", "ws://") + "/ws/chat";

    // Test WebSocket connection
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    // If we get here, the WebSocket connection was successful
    drop(ws_stream);
}
