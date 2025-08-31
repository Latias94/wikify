//! Simple WebSocket Message Format Test

#[tokio::test]
async fn test_websocket_message_serialization() {
    println!("🧪 Testing WebSocket message serialization");

    // Test IndexProgress message serialization
    let ws_message = wikify_web::websocket::WsMessage::IndexProgress {
        session_id: "test-session-123".to_string(),
        progress: 0.5,
        files_processed: 10,
        total_files: 20,
        current_file: Some("test.rs".to_string()),
    };

    let serialized = serde_json::to_string(&ws_message).unwrap();
    println!("📤 Serialized message: {}", serialized);

    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    // Verify the message format matches frontend expectations
    assert_eq!(parsed["type"], "index_progress");
    assert_eq!(parsed["session_id"], "test-session-123");
    assert_eq!(parsed["progress"], 0.5);
    assert_eq!(parsed["files_processed"], 10);
    assert_eq!(parsed["total_files"], 20);
    assert_eq!(parsed["current_file"], "test.rs");

    println!("✅ WebSocket message format is compatible with frontend!");
}

#[tokio::test]
async fn test_websocket_error_serialization() {
    println!("🧪 Testing WebSocket error message serialization");

    let ws_message = wikify_web::websocket::WsMessage::Error {
        message: "Test error message".to_string(),
        code: Some("INDEXING_ERROR".to_string()),
    };

    let serialized = serde_json::to_string(&ws_message).unwrap();
    println!("📤 Serialized error: {}", serialized);

    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    // Verify error message format
    assert_eq!(parsed["type"], "error");
    assert_eq!(parsed["message"], "Test error message");
    assert_eq!(parsed["code"], "INDEXING_ERROR");

    println!("✅ WebSocket error message format is correct!");
}
