//! Integration tests for research template system

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use wikify_applications::{ResearchCategory, ResearchTemplate};
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

/// Test helper to register and login a test user
async fn create_test_user_and_login(app: &mut axum::Router) -> (String, String) {
    // Register test user
    let register_request = json!({
        "username": "test_researcher",
        "email": "researcher@test.com",
        "password": "test123456",
        "display_name": "Test Researcher"
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

/// Test helper to create admin user and login
async fn create_admin_user_and_login(app: &mut axum::Router) -> (String, String) {
    // Login as default admin
    let login_request = json!({
        "username": "admin",
        "password": "admin123"
    });

    let request =
        create_authenticated_request("POST", "/api/auth/login", Some(login_request), None).await;

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
async fn test_list_research_templates() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Test listing all templates (public endpoint)
    let request = create_authenticated_request("GET", "/api/research/templates", None, None).await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let templates: Vec<ResearchTemplate> = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    // Should have at least the built-in templates
    assert!(!templates.is_empty());

    // Check for expected templates
    let template_ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
    assert!(template_ids.contains(&"technical-analysis"));
    assert!(template_ids.contains(&"architecture-assessment"));
    assert!(template_ids.contains(&"security-analysis"));
    assert!(template_ids.contains(&"documentation-extraction"));

    println!("✅ Found {} research templates", templates.len());
}

#[tokio::test]
async fn test_get_specific_research_template() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Test getting specific template
    let request = create_authenticated_request(
        "GET",
        "/api/research/templates/technical-analysis",
        None,
        None,
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let template: ResearchTemplate = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert_eq!(template.id, "technical-analysis");
    assert_eq!(template.name, "Technical Analysis");
    assert_eq!(template.category, ResearchCategory::Technical);
    assert!(!template.initial_questions.is_empty());

    println!("✅ Technical analysis template retrieved successfully");
}

#[tokio::test]
async fn test_get_nonexistent_template() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Test getting non-existent template
    let request = create_authenticated_request(
        "GET",
        "/api/research/templates/nonexistent-template",
        None,
        None,
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    println!("✅ Non-existent template correctly returns 404");
}

#[tokio::test]
async fn test_list_templates_by_category() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Test listing templates by category
    let request = create_authenticated_request(
        "GET",
        "/api/research/templates/category/technical",
        None,
        None,
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let templates: Vec<ResearchTemplate> = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    // Should have technical templates
    assert!(!templates.is_empty());

    // All templates should be technical category
    for template in &templates {
        assert_eq!(template.category, ResearchCategory::Technical);
    }

    println!("✅ Found {} technical templates", templates.len());
}

#[tokio::test]
async fn test_invalid_category() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Test invalid category
    let request = create_authenticated_request(
        "GET",
        "/api/research/templates/category/invalid-category",
        None,
        None,
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    println!("✅ Invalid category correctly returns 400");
}

#[tokio::test]
async fn test_start_research_from_template_authentication() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Test starting research without authentication
    let start_request = json!({
        "session_id": "test-session-001",
        "template_id": "technical-analysis",
        "topic": "Test Research Topic",
        "parameters": {}
    });

    let request = create_authenticated_request(
        "POST",
        "/api/research/start-from-template",
        Some(start_request),
        None, // No token
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    println!("✅ Unauthenticated research start correctly returns 401");
}

#[tokio::test]
async fn test_start_research_from_template_with_auth() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create and login test user
    let (_user_id, access_token) = create_test_user_and_login(&mut app).await;

    // Test starting research with authentication
    let start_request = json!({
        "session_id": "test-session-002",
        "template_id": "technical-analysis",
        "topic": "Authenticated Research Test",
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

    let progress_response = extract_json_response(response).await;
    assert!(progress_response["session_id"].is_string());
    assert!(progress_response["current_iteration"].is_number());

    println!("✅ Authenticated research start successful");
}

#[tokio::test]
async fn test_start_research_with_invalid_template() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create and login test user
    let (_user_id, access_token) = create_test_user_and_login(&mut app).await;

    // Test starting research with invalid template
    let start_request = json!({
        "session_id": "test-session-003",
        "template_id": "invalid-template",
        "topic": "Invalid Template Test",
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
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    println!("✅ Invalid template correctly returns 404");
}

#[tokio::test]
async fn test_research_history_access_control() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create two test users
    let (user1_id, user1_token) = create_test_user_and_login(&mut app).await;

    // Register second user
    let register_request = json!({
        "username": "test_researcher_2",
        "email": "researcher2@test.com",
        "password": "test123456",
        "display_name": "Test Researcher 2"
    });

    let request =
        create_authenticated_request("POST", "/api/auth/register", Some(register_request), None)
            .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let auth_response = extract_json_response(response).await;
    let user2_token = auth_response["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    // User 1 starts a research
    let start_request = json!({
        "session_id": "user1-research-001",
        "template_id": "technical-analysis",
        "topic": "User 1 Research",
        "parameters": {}
    });

    let request = create_authenticated_request(
        "POST",
        "/api/research/start-from-template",
        Some(start_request),
        Some(&user1_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // User 1 can see their own research history
    let request =
        create_authenticated_request("GET", "/api/research/history", None, Some(&user1_token))
            .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let history = extract_json_response(response).await;
    let records = history.as_array().unwrap();
    assert!(!records.is_empty());

    // User 2 should not see User 1's research
    let request =
        create_authenticated_request("GET", "/api/research/history", None, Some(&user2_token))
            .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let history = extract_json_response(response).await;
    let records = history.as_array().unwrap();
    assert!(records.is_empty()); // User 2 has no research history

    println!("✅ Research history access control working correctly");
}

#[tokio::test]
async fn test_admin_access_to_all_research() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create regular user and start research
    let (_user_id, user_token) = create_test_user_and_login(&mut app).await;

    let start_request = json!({
        "session_id": "user-research-for-admin-test",
        "template_id": "documentation-extraction",
        "topic": "User Research for Admin Test",
        "parameters": {}
    });

    let request = create_authenticated_request(
        "POST",
        "/api/research/start-from-template",
        Some(start_request),
        Some(&user_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Login as admin
    let (_admin_id, admin_token) = create_admin_user_and_login(&mut app).await;

    // Admin should see all research history
    let request =
        create_authenticated_request("GET", "/api/research/history", None, Some(&admin_token))
            .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let history = extract_json_response(response).await;
    let records = history.as_array().unwrap();
    assert!(!records.is_empty()); // Admin can see user's research

    println!("✅ Admin can access all research history");
}

#[tokio::test]
async fn test_research_statistics_admin_only() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create regular user
    let (_user_id, user_token) = create_test_user_and_login(&mut app).await;

    // Regular user should not access statistics
    let request =
        create_authenticated_request("GET", "/api/research/statistics", None, Some(&user_token))
            .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Admin should access statistics
    let (_admin_id, admin_token) = create_admin_user_and_login(&mut app).await;

    let request =
        create_authenticated_request("GET", "/api/research/statistics", None, Some(&admin_token))
            .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let stats = extract_json_response(response).await;
    assert!(stats["total_sessions"].is_number());
    assert!(stats["completed_sessions"].is_number());

    println!("✅ Research statistics access control working correctly");
}

#[tokio::test]
async fn test_complete_research_workflow() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create and login test user
    let (_user_id, access_token) = create_test_user_and_login(&mut app).await;

    // 1. List available templates
    let request = create_authenticated_request("GET", "/api/research/templates", None, None).await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let templates: Vec<ResearchTemplate> = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(!templates.is_empty());

    // 2. Get specific template details
    let template_id = &templates[0].id;
    let request = create_authenticated_request(
        "GET",
        &format!("/api/research/templates/{}", template_id),
        None,
        None,
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 3. Start research from template
    let session_id = format!("workflow-test-{}", uuid::Uuid::new_v4());
    let start_request = json!({
        "session_id": session_id,
        "template_id": template_id,
        "topic": "Complete Workflow Test",
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

    let progress_response = extract_json_response(response).await;
    assert_eq!(progress_response["session_id"], session_id);

    // 4. Check research progress
    let request = create_authenticated_request(
        "GET",
        &format!("/api/research/progress/{}", session_id),
        None,
        Some(&access_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Check research appears in history
    let request =
        create_authenticated_request("GET", "/api/research/history", None, Some(&access_token))
            .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let history = extract_json_response(response).await;
    let records = history.as_array().unwrap();

    // Should find our research session
    let found_session = records
        .iter()
        .any(|record| record["session_id"] == session_id);
    assert!(found_session);

    println!("✅ Complete research workflow test successful");
}

#[tokio::test]
async fn test_research_record_access_control() {
    // Create test app
    let config = WebConfig::default();
    let state = AppState::new(config).await.unwrap();
    let mut app = create_app(state);

    // Create two users
    let (_user1_id, user1_token) = create_test_user_and_login(&mut app).await;

    let register_request = json!({
        "username": "test_researcher_3",
        "email": "researcher3@test.com",
        "password": "test123456",
        "display_name": "Test Researcher 3"
    });

    let request =
        create_authenticated_request("POST", "/api/auth/register", Some(register_request), None)
            .await;

    let response = app.clone().oneshot(request).await.unwrap();
    let auth_response = extract_json_response(response).await;
    let user2_token = auth_response["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    // User 1 starts research
    let session_id = "access-control-test-session";
    let start_request = json!({
        "session_id": session_id,
        "template_id": "security-analysis",
        "topic": "Access Control Test Research",
        "parameters": {}
    });

    let request = create_authenticated_request(
        "POST",
        "/api/research/start-from-template",
        Some(start_request),
        Some(&user1_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // User 1 can access their own research record
    let request = create_authenticated_request(
        "GET",
        &format!("/api/research/history/{}", session_id),
        None,
        Some(&user1_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // User 2 cannot access User 1's research record
    let request = create_authenticated_request(
        "GET",
        &format!("/api/research/history/{}", session_id),
        None,
        Some(&user2_token),
    )
    .await;

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND); // Access denied returns 404

    println!("✅ Research record access control working correctly");
}
