//! Authentication handlers for user registration, login, and token management

use super::{
    api_keys::{ApiKeyService, CreateApiKeyRequest},
    jwt::AuthError,
    users::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest, UserService},
    User,
};
use crate::AppState;
use axum::{extract::State, http::StatusCode, response::Json, Json as JsonExtractor};
use serde_json::{json, Value};
use tracing::info;

/// User registration endpoint
///
/// Register a new user account with username, email, and password.
/// Returns user information and JWT tokens on success.
pub async fn register_user(
    State(app_state): State<AppState>,
    JsonExtractor(request): JsonExtractor<RegisterRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    info!("User registration attempt: {}", request.username);

    let response = app_state.user_service.register(request).await?;

    info!("User registered successfully: {}", response.user.username);
    Ok(Json(response))
}

/// User login endpoint
///
/// Authenticate user with username and password.
/// Returns user information and JWT tokens on success.
pub async fn login_user(
    State(app_state): State<AppState>,
    JsonExtractor(request): JsonExtractor<LoginRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    info!("User login attempt: {}", request.username);

    let response = app_state.user_service.login(request).await?;

    info!("User logged in successfully: {}", response.user.username);
    Ok(Json(response))
}

/// Token refresh endpoint
///
/// Refresh access token using a valid refresh token.
/// Returns new token pair on success.
pub async fn refresh_token(
    State(app_state): State<AppState>,
    JsonExtractor(request): JsonExtractor<RefreshRequest>,
) -> Result<Json<Value>, AuthError> {
    info!("Token refresh attempt");

    let tokens = app_state.user_service.refresh_token(request).await?;

    info!("Token refreshed successfully");
    Ok(Json(json!(tokens)))
}

/// Get current user information
///
/// Returns information about the currently authenticated user.
/// Requires valid JWT token in Authorization header.
pub async fn get_current_user(user: User) -> Result<Json<Value>, StatusCode> {
    info!("Getting current user info: {}", user.id);

    Ok(Json(json!({
        "id": user.id,
        "display_name": user.display_name,
        "permissions": user.permissions.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>(),
        "is_admin": user.is_admin,
    })))
}

/// Logout endpoint (client-side token invalidation)
///
/// This endpoint doesn't actually invalidate tokens server-side (stateless JWT),
/// but provides a standard logout endpoint for clients to call.
/// Clients should discard their tokens after calling this endpoint.
pub async fn logout_user(user: User) -> Result<Json<Value>, StatusCode> {
    info!("User logout: {}", user.id);

    Ok(Json(json!({
        "message": "Logged out successfully",
        "user_id": user.id
    })))
}

/// Change password endpoint
///
/// Allows authenticated users to change their password.
/// Requires current password for verification.
pub async fn change_password(
    user: User,
    JsonExtractor(request): JsonExtractor<ChangePasswordRequest>,
) -> Result<Json<Value>, AuthError> {
    info!("Password change attempt for user: {}", user.id);

    // TODO: Implement password change logic
    // This would require:
    // 1. Verify current password
    // 2. Hash new password
    // 3. Update user in store
    // 4. Optionally invalidate existing tokens

    // For now, return not implemented
    Err(AuthError::TokenCreation) // Placeholder error
}

/// Password change request
#[derive(serde::Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use serde_json::json;
    use tower::ServiceExt;

    async fn create_test_app() -> Router {
        use crate::WebConfig;

        let app_state = crate::AppState::new(WebConfig::default()).await.unwrap();

        Router::new()
            .route("/auth/register", axum::routing::post(register_user))
            .route("/auth/login", axum::routing::post(login_user))
            .route("/auth/refresh", axum::routing::post(refresh_token))
            .route("/auth/me", axum::routing::get(get_current_user))
            .route("/auth/logout", axum::routing::post(logout_user))
            .with_state(app_state)
    }

    #[tokio::test]
    async fn test_user_registration() {
        let app = create_test_app().await;

        let request_body = json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "password123",
            "display_name": "Test User"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_user_login() {
        let app = create_test_app().await;

        // First register a user
        let register_body = json!({
            "username": "logintest",
            "email": "login@example.com",
            "password": "password123"
        });

        let _register_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(register_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then try to login
        let login_body = json!({
            "username": "logintest",
            "password": "password123"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(login_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_login() {
        let app = create_test_app().await;

        let login_body = json!({
            "username": "nonexistent",
            "password": "wrongpassword"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(login_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

/// Create a new API key for the authenticated user
///
/// This endpoint allows authenticated users to create new API keys
/// for programmatic access to the API.
pub async fn create_api_key(
    State(state): State<AppState>,
    user: User,
    JsonExtractor(request): JsonExtractor<CreateApiKeyRequest>,
) -> Result<Json<Value>, StatusCode> {
    match state
        .api_key_service
        .create_api_key(&user.id, request)
        .await
    {
        Ok(api_key_response) => {
            info!(
                "Created API key '{}' for user: {}",
                api_key_response.name, user.id
            );
            Ok(Json(json!({
                "success": true,
                "api_key": api_key_response
            })))
        }
        Err(e) => {
            tracing::error!("Failed to create API key: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List API keys for the authenticated user
///
/// Returns a list of all API keys owned by the authenticated user.
/// The actual key values are not returned for security reasons.
pub async fn list_api_keys(
    State(state): State<AppState>,
    user: User,
) -> Result<Json<Value>, StatusCode> {
    match state
        .api_key_service
        .storage()
        .list_user_api_keys(&user.id)
        .await
    {
        Ok(api_keys) => {
            let api_key_responses: Vec<_> = api_keys
                .into_iter()
                .map(|key| super::api_keys::ApiKeyResponse {
                    id: key.id,
                    key: None, // Never return the actual key
                    name: key.name,
                    permissions: key.permissions,
                    created_at: key.created_at,
                    expires_at: key.expires_at,
                    last_used_at: key.last_used_at,
                    is_active: key.is_active,
                })
                .collect();

            Ok(Json(json!({
                "success": true,
                "api_keys": api_key_responses
            })))
        }
        Err(e) => {
            tracing::error!("Failed to list API keys: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Delete an API key
///
/// Allows users to delete their own API keys.
pub async fn delete_api_key(
    State(state): State<AppState>,
    user: User,
    axum::extract::Path(key_id): axum::extract::Path<String>,
) -> Result<Json<Value>, StatusCode> {
    // First check if the API key belongs to the user
    match state
        .api_key_service
        .storage()
        .get_api_key_by_id(&key_id)
        .await
    {
        Ok(Some(api_key)) => {
            if api_key.user_id != user.id {
                return Err(StatusCode::FORBIDDEN);
            }

            // Delete the API key
            match state
                .api_key_service
                .storage()
                .delete_api_key(&key_id)
                .await
            {
                Ok(()) => {
                    info!("Deleted API key '{}' for user: {}", api_key.name, user.id);
                    Ok(Json(json!({
                        "success": true,
                        "message": "API key deleted successfully"
                    })))
                }
                Err(e) => {
                    tracing::error!("Failed to delete API key: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get API key: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
