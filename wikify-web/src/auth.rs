//! Authentication and authorization using Axum best practices

pub mod api_keys;
pub mod database;
pub mod handlers;
pub mod jwt;
pub mod users;

#[cfg(test)]
mod tests;

use crate::AppState;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Redirect, Response},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use wikify_applications::{Permission, PermissionContext, UserIdentity};

/// Authenticated user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: String,
    /// Display name
    pub display_name: Option<String>,
    /// User permissions
    pub permissions: Vec<Permission>,
    /// Whether user is admin
    pub is_admin: bool,
}

impl User {
    /// Create a new user
    pub fn new(id: String, display_name: Option<String>, permissions: Vec<Permission>) -> Self {
        let is_admin = permissions.contains(&Permission::Admin);
        Self {
            id,
            display_name,
            permissions,
            is_admin,
        }
    }

    /// Check if user has specific permission
    /// Admin users automatically have all permissions
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.is_admin || self.permissions.contains(permission)
    }

    /// Convert to PermissionContext for application layer
    pub fn to_permission_context(&self) -> PermissionContext {
        let identity = UserIdentity::registered(
            self.id.clone(),
            self.display_name.clone(),
            None, // email not stored in context
        );
        PermissionContext::user(identity)
    }
}

/// Anonymous user context for public endpoints
#[derive(Debug, Clone)]
pub struct AnonymousUser;

impl AnonymousUser {
    /// Convert to PermissionContext for application layer
    pub fn to_permission_context(&self) -> PermissionContext {
        let permissions = vec![Permission::Query].into_iter().collect();
        let limits = wikify_applications::auth::permissions::ResourceLimits::anonymous();
        PermissionContext::anonymous(permissions, limits)
    }
}

/// Authentication redirect for failed auth
#[derive(Debug)]
pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        Redirect::temporary("/login").into_response()
    }
}

/// Permission denied error with detailed message
#[derive(Debug)]
pub struct PermissionDenied {
    pub required_permission: String,
    pub user_id: Option<String>,
}

impl PermissionDenied {
    pub fn new(required_permission: &str, user_id: Option<String>) -> Self {
        Self {
            required_permission: required_permission.to_string(),
            user_id,
        }
    }
}

impl IntoResponse for PermissionDenied {
    fn into_response(self) -> Response {
        let message = if let Some(user_id) = &self.user_id {
            format!(
                "User '{}' does not have required permission: {}",
                user_id, self.required_permission
            )
        } else {
            format!("Required permission: {}", self.required_permission)
        };

        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "permission_denied",
                "message": message,
                "required_permission": self.required_permission,
                "user_id": self.user_id
            })),
        )
            .into_response()
    }
}

/// Implement FromRequestParts for User (authenticated users only)
impl<S> FromRequestParts<S> for User
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthRedirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let _app_state = AppState::from_ref(state);

        // First try JWT authentication
        if let Ok(claims) = jwt::Claims::from_request_parts(parts, state).await {
            if let Ok(user) = claims.to_user() {
                return Ok(user);
            }
        }

        // Fallback to header-based authentication (for backward compatibility)
        if let Some(user) = authenticate_from_headers(&parts.headers).await {
            Ok(user)
        } else {
            // Authentication failed, redirect to login
            Err(AuthRedirect)
        }
    }
}

/// Optional user extractor - doesn't fail if user is not authenticated
pub struct OptionalUser(pub Option<User>);

impl<S> FromRequestParts<S> for OptionalUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let _app_state = AppState::from_ref(state);

        // First try JWT authentication
        if let Ok(claims) = jwt::Claims::from_request_parts(parts, state).await {
            if let Ok(user) = claims.to_user() {
                return Ok(OptionalUser(Some(user)));
            }
        }

        // Fallback to header-based authentication
        Ok(OptionalUser(
            authenticate_from_headers(&parts.headers).await,
        ))
    }
}

/// Authenticate user from request headers
async fn authenticate_from_headers(headers: &HeaderMap) -> Option<User> {
    // Check for API key authentication
    if let Some(api_key) = extract_api_key(headers) {
        if let Some(user) = authenticate_api_key(&api_key).await {
            return Some(user);
        }
    }

    // Check for user ID header (simple auth for development)
    if let Some(user_id) = extract_user_id(headers) {
        return Some(User::new(
            user_id,
            Some("Header User".to_string()),
            vec![Permission::Query, Permission::GenerateWiki],
        ));
    }

    None
}

/// Extract API key from headers
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-api-key")
        .or_else(|| headers.get("authorization"))
        .and_then(|value| value.to_str().ok())
        .and_then(|auth| {
            if auth.starts_with("Bearer ") {
                Some(auth[7..].to_string())
            } else {
                Some(auth.to_string())
            }
        })
}

/// Extract user ID from headers (simple development auth)
fn extract_user_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-user-id")
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string())
}

/// Authenticate using API key (placeholder implementation)
async fn authenticate_api_key(api_key: &str) -> Option<User> {
    debug!(
        "Authenticating with API key: {}...",
        &api_key[..api_key.len().min(8)]
    );

    // TODO: Implement actual API key validation
    // For now, accept any non-empty API key
    if !api_key.is_empty() {
        Some(User::new(
            format!("api_user_{}", &api_key[..api_key.len().min(8)]),
            Some("API User".to_string()),
            vec![
                Permission::Query,
                Permission::GenerateWiki,
                Permission::Admin,
            ],
        ))
    } else {
        None
    }
}

/// Admin user extractor - requires admin permissions
pub struct AdminUser(pub User);

impl<S> FromRequestParts<S> for AdminUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = User::from_request_parts(parts, state)
            .await
            .map_err(|auth_redirect| auth_redirect.into_response())?;

        if user.is_admin {
            Ok(AdminUser(user))
        } else {
            warn!("Admin access required but user '{}' is not admin", user.id);
            Err(PermissionDenied::new("Admin", Some(user.id)).into_response())
        }
    }
}

/// Generic permission extractor
pub struct RequirePermission<const P: u8>(pub User);

// Note: Generic permission extractor with const generics is complex in current Rust
// For now, we'll use specific extractors for each permission type

/// Specific permission extractors for common use cases
pub struct RequireQuery(pub User);
pub struct RequireGenerateWiki(pub User);
pub struct RequireDeepResearch(pub User);
pub struct RequireExport(pub User);
pub struct RequireManageSession(pub User);

impl<S> FromRequestParts<S> for RequireQuery
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // For Query permission, allow both authenticated users and anonymous users
        let OptionalUser(maybe_user) = OptionalUser::from_request_parts(parts, state)
            .await
            .unwrap();

        if let Some(user) = maybe_user {
            if user.has_permission(&Permission::Query) {
                Ok(RequireQuery(user))
            } else {
                warn!(
                    "Query permission required but user '{}' doesn't have it",
                    user.id
                );
                Err(PermissionDenied::new("Query", Some(user.id)).into_response())
            }
        } else {
            // Anonymous users have query permission by default
            // Create a temporary user for this request
            let anonymous_user = User::new("anonymous".to_string(), None, vec![Permission::Query]);
            Ok(RequireQuery(anonymous_user))
        }
    }
}

impl<S> FromRequestParts<S> for RequireGenerateWiki
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = User::from_request_parts(parts, state)
            .await
            .map_err(|auth_redirect| auth_redirect.into_response())?;

        if user.has_permission(&Permission::GenerateWiki) {
            Ok(RequireGenerateWiki(user))
        } else {
            warn!(
                "GenerateWiki permission required but user '{}' doesn't have it",
                user.id
            );
            Err(PermissionDenied::new("GenerateWiki", Some(user.id)).into_response())
        }
    }
}

impl<S> FromRequestParts<S> for RequireDeepResearch
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = User::from_request_parts(parts, state)
            .await
            .map_err(|auth_redirect| auth_redirect.into_response())?;

        if user.has_permission(&Permission::DeepResearch) {
            Ok(RequireDeepResearch(user))
        } else {
            warn!(
                "DeepResearch permission required but user '{}' doesn't have it",
                user.id
            );
            Err(PermissionDenied::new("DeepResearch", Some(user.id)).into_response())
        }
    }
}

impl<S> FromRequestParts<S> for RequireExport
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = User::from_request_parts(parts, state)
            .await
            .map_err(|auth_redirect| auth_redirect.into_response())?;

        if user.has_permission(&Permission::Export) {
            Ok(RequireExport(user))
        } else {
            warn!(
                "Export permission required but user '{}' doesn't have it",
                user.id
            );
            Err(PermissionDenied::new("Export", Some(user.id)).into_response())
        }
    }
}

impl<S> FromRequestParts<S> for RequireManageSession
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = User::from_request_parts(parts, state)
            .await
            .map_err(|auth_redirect| auth_redirect.into_response())?;

        if user.has_permission(&Permission::ManageSession) {
            Ok(RequireManageSession(user))
        } else {
            warn!(
                "ManageSession permission required but user '{}' doesn't have it",
                user.id
            );
            Err(PermissionDenied::new("ManageSession", Some(user.id)).into_response())
        }
    }
}
