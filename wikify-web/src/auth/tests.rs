//! Tests for the authentication and authorization system

use super::*;
use axum::{
    body::Body,
    extract::FromRequestParts,
    http::{HeaderMap, HeaderValue, Method, Request},
};
use wikify_applications::Permission;

/// Helper function to create test request parts with AppState
async fn create_test_parts_with_headers(
    headers: HeaderMap,
) -> (axum::http::request::Parts, crate::AppState) {
    let mut request = Request::builder()
        .method(Method::GET)
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    *request.headers_mut() = headers;

    let (parts, _) = request.into_parts();
    let state = crate::AppState::new(crate::WebConfig::default())
        .await
        .unwrap();
    (parts, state)
}

/// Helper function to create headers with API key
fn headers_with_api_key(api_key: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", HeaderValue::from_str(api_key).unwrap());
    headers
}

/// Helper function to create headers with user ID
fn headers_with_user_id(user_id: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("x-user-id", HeaderValue::from_str(user_id).unwrap());
    headers
}

/// Helper function to create headers with Bearer token
fn headers_with_bearer_token(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let auth_value = format!("Bearer {}", token);
    headers.insert("authorization", HeaderValue::from_str(&auth_value).unwrap());
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_extraction_with_api_key() {
        let headers = headers_with_api_key("test-api-key");
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = User::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let user = result.unwrap();
        assert!(user.id.starts_with("api_user_"));
        assert!(user.has_permission(&Permission::Admin));
    }

    #[tokio::test]
    async fn test_user_extraction_with_user_id() {
        let headers = headers_with_user_id("test-user-123");
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = User::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.id, "test-user-123");
        assert!(user.has_permission(&Permission::Query));
        assert!(user.has_permission(&Permission::GenerateWiki));
    }

    #[tokio::test]
    async fn test_user_extraction_with_bearer_token() {
        let headers = headers_with_bearer_token("test-bearer-token");
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = User::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let user = result.unwrap();
        assert!(user.id.starts_with("api_user_"));
        assert!(user.has_permission(&Permission::Admin));
    }

    #[tokio::test]
    async fn test_user_extraction_without_auth() {
        let headers = HeaderMap::new();
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = User::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
        // Should return AuthRedirect for unauthenticated requests
    }

    #[tokio::test]
    async fn test_optional_user_extraction_without_auth() {
        let headers = HeaderMap::new();
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = OptionalUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let OptionalUser(maybe_user) = result.unwrap();
        assert!(maybe_user.is_none());
    }

    #[tokio::test]
    async fn test_optional_user_extraction_with_auth() {
        let headers = headers_with_api_key("test-key");
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = OptionalUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let OptionalUser(maybe_user) = result.unwrap();
        assert!(maybe_user.is_some());
        let user = maybe_user.unwrap();
        assert!(user.has_permission(&Permission::Admin));
    }

    #[tokio::test]
    async fn test_require_query_with_authenticated_user() {
        let headers = headers_with_user_id("test-user");
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = RequireQuery::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let RequireQuery(user) = result.unwrap();
        assert_eq!(user.id, "test-user");
        assert!(user.has_permission(&Permission::Query));
    }

    #[tokio::test]
    async fn test_require_query_with_anonymous_user() {
        let headers = HeaderMap::new();
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = RequireQuery::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let RequireQuery(user) = result.unwrap();
        assert_eq!(user.id, "anonymous");
        assert!(user.has_permission(&Permission::Query));
    }

    #[tokio::test]
    async fn test_require_generate_wiki_with_auth() {
        let headers = headers_with_user_id("test-user");
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = RequireGenerateWiki::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let RequireGenerateWiki(user) = result.unwrap();
        assert!(user.has_permission(&Permission::GenerateWiki));
    }

    #[tokio::test]
    async fn test_require_generate_wiki_without_auth() {
        let headers = HeaderMap::new();
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = RequireGenerateWiki::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
        // Should fail for unauthenticated requests
    }

    #[tokio::test]
    async fn test_admin_user_with_admin_permissions() {
        let headers = headers_with_api_key("admin-key");
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = AdminUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
        let AdminUser(user) = result.unwrap();
        assert!(user.is_admin);
        assert!(user.has_permission(&Permission::Admin));
    }

    #[tokio::test]
    async fn test_admin_user_without_admin_permissions() {
        let headers = headers_with_user_id("regular-user");
        let (mut parts, state) = create_test_parts_with_headers(headers).await;

        let result = AdminUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
        // Should fail for non-admin users
    }

    #[tokio::test]
    async fn test_user_permission_context_conversion() {
        let user = User::new(
            "test-user".to_string(),
            Some("Test User".to_string()),
            vec![Permission::Query, Permission::GenerateWiki],
        );

        let _context = user.to_permission_context();

        // Verify the context is created correctly
        // Note: We can't easily test the internal structure without exposing more methods
        // This test mainly ensures the conversion doesn't panic
    }

    #[tokio::test]
    async fn test_anonymous_user_permission_context() {
        let anonymous = AnonymousUser;
        let _context = anonymous.to_permission_context();

        // Verify anonymous context is created correctly
        // This test mainly ensures the conversion doesn't panic
    }

    #[tokio::test]
    async fn test_admin_permission_inheritance() {
        // Admin user should have all permissions even if not explicitly granted
        let admin_user = User::new(
            "admin-user".to_string(),
            Some("Admin User".to_string()),
            vec![Permission::Admin], // Only Admin permission explicitly granted
        );

        // Admin should have all permissions through inheritance
        assert!(admin_user.has_permission(&Permission::Query));
        assert!(admin_user.has_permission(&Permission::GenerateWiki));
        assert!(admin_user.has_permission(&Permission::DeepResearch));
        assert!(admin_user.has_permission(&Permission::Export));
        assert!(admin_user.has_permission(&Permission::ManageRepository));
        assert!(admin_user.has_permission(&Permission::Admin));
        assert!(admin_user.is_admin);
    }

    #[tokio::test]
    async fn test_regular_user_no_inheritance() {
        // Regular user should only have explicitly granted permissions
        let regular_user = User::new(
            "regular-user".to_string(),
            Some("Regular User".to_string()),
            vec![Permission::Query, Permission::GenerateWiki],
        );

        // Should have explicitly granted permissions
        assert!(regular_user.has_permission(&Permission::Query));
        assert!(regular_user.has_permission(&Permission::GenerateWiki));

        // Should NOT have other permissions
        assert!(!regular_user.has_permission(&Permission::DeepResearch));
        assert!(!regular_user.has_permission(&Permission::Export));
        assert!(!regular_user.has_permission(&Permission::ManageRepository));
        assert!(!regular_user.has_permission(&Permission::Admin));
        assert!(!regular_user.is_admin);
    }
}
