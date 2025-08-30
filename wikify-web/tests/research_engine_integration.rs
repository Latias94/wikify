//! Integration tests for research engine functionality

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use wikify_web::{create_app, AppState, WebConfig};

/// Test helper to create authenticated request
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

/// Test helper to extract JSON response
async fn extract_json_response(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

/// Test helper to create and login test user
async fn create_test_user_and_login(app: &mut axum::Router) -> (String, String) {
    let register_request = json!({
        "username": "research_tester",
        "email": "research@test.com",
        "password": "test123456",
        "display_name": "Research Tester"
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

#[tokio::test]
async fn test_template_to_research_engine_flow() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create and login test user
    let (_user_id, access_token) = create_test_user_and_login(&mut app).await;

    // 1. Start research from template
    let session_id = format!("engine-test-{}", uuid::Uuid::new_v4());
    let start_request = json!({
        "session_id": session_id,
        "template_id": "technical-analysis",
        "topic": "Research Engine Integration Test",
        "parameters": {
            "focus_language": "rust"
        }
    });

    let request = create_authenticated_request(
        "POST",
        "/api/research/start-from-template",
        Some(start_request),
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let start_response = extract_json_response(response).await;
    assert_eq!(start_response["session_id"], session_id);
    assert!(start_response["current_iteration"].as_u64().unwrap() >= 1);

    // 2. Check research progress
    let request = create_authenticated_request(
        "GET",
        &format!("/api/research/progress/{}", session_id),
        None,
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let progress_response = extract_json_response(response).await;
    assert_eq!(progress_response["session_id"], session_id);
    assert!(progress_response["questions"].as_array().unwrap().len() > 0);

    // 3. Continue research iteration
    let request = create_authenticated_request(
        "POST",
        &format!("/api/research/iterate/{}", session_id),
        None,
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let iteration_response = extract_json_response(response).await;
    assert_eq!(iteration_response["session_id"], session_id);

    // 4. Verify research appears in history
    let request = create_authenticated_request(
        "GET",
        &format!("/api/research/history/{}", session_id),
        None,
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let record = extract_json_response(response).await;
    assert_eq!(record["session_id"], session_id);
    assert_eq!(record["template_id"], "technical-analysis");
    assert_eq!(record["topic"], "Research Engine Integration Test");

    println!("✅ Template to research engine flow working correctly");
}

#[tokio::test]
async fn test_research_history_filtering() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create and login test user
    let (_user_id, access_token) = create_test_user_and_login(&mut app).await;

    // Start multiple research sessions with different templates
    let sessions = vec![
        ("filter-test-1", "technical-analysis", "Technical Test 1"),
        ("filter-test-2", "security-analysis", "Security Test 1"),
        ("filter-test-3", "technical-analysis", "Technical Test 2"),
    ];

    for (session_id, template_id, topic) in &sessions {
        let start_request = json!({
            "session_id": session_id,
            "template_id": template_id,
            "topic": topic,
            "parameters": {}
        });

        let request = create_authenticated_request(
            "POST",
            "/api/research/start-from-template",
            Some(start_request),
            Some(&access_token),
        )
        .await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // Test filtering by template
    let request = create_authenticated_request(
        "GET",
        "/api/research/history?template_id=technical-analysis",
        None,
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let history = extract_json_response(response).await;
    let records = history.as_array().unwrap();

    // Should find 2 technical analysis sessions
    assert_eq!(records.len(), 2);
    for record in records {
        assert_eq!(record["template_id"], "technical-analysis");
    }

    // Test limiting results
    let request = create_authenticated_request(
        "GET",
        "/api/research/history?limit=1",
        None,
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let history = extract_json_response(response).await;
    let records = history.as_array().unwrap();
    assert_eq!(records.len(), 1);

    println!("✅ Research history filtering working correctly");
}

#[tokio::test]
async fn test_research_record_deletion() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create and login test user
    let (_user_id, access_token) = create_test_user_and_login(&mut app).await;

    // Start research session
    let session_id = "deletion-test-session";
    let start_request = json!({
        "session_id": session_id,
        "template_id": "documentation-extraction",
        "topic": "Deletion Test Research",
        "parameters": {}
    });

    let request = create_authenticated_request(
        "POST",
        "/api/research/start-from-template",
        Some(start_request),
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify record exists
    let request = create_authenticated_request(
        "GET",
        &format!("/api/research/history/{}", session_id),
        None,
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Delete the record
    let request = create_authenticated_request(
        "DELETE",
        &format!("/api/research/history/{}", session_id),
        None,
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify record is deleted
    let request = create_authenticated_request(
        "GET",
        &format!("/api/research/history/{}", session_id),
        None,
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    println!("✅ Research record deletion working correctly");
}

#[tokio::test]
async fn test_template_parameter_validation() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create and login test user
    let (_user_id, access_token) = create_test_user_and_login(&mut app).await;

    // Test with valid parameters
    let session_id = "param-test-valid";
    let start_request = json!({
        "session_id": session_id,
        "template_id": "technical-analysis",
        "topic": "Parameter Validation Test",
        "parameters": {
            "focus_language": "rust"
        }
    });

    let request = create_authenticated_request(
        "POST",
        "/api/research/start-from-template",
        Some(start_request),
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test with empty parameters (should still work)
    let session_id = "param-test-empty";
    let start_request = json!({
        "session_id": session_id,
        "template_id": "architecture-assessment",
        "topic": "Empty Parameters Test",
        "parameters": {}
    });

    let request = create_authenticated_request(
        "POST",
        "/api/research/start-from-template",
        Some(start_request),
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    println!("✅ Template parameter validation working correctly");
}
