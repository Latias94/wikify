//! JWT authentication implementation based on Axum official examples

use super::{PermissionDenied, User};
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::LazyLock;
use tracing::{debug, warn};
use wikify_applications::Permission;

/// JWT signing keys - initialized from environment variable
static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "wikify-default-secret-change-in-production".to_string());
    Keys::new(secret.as_bytes())
});

/// JWT signing and verification keys
struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// User display name
    pub name: Option<String>,
    /// User email
    pub email: Option<String>,
    /// User permissions
    pub permissions: Vec<String>,
    /// Is admin user
    pub is_admin: bool,
    /// Issued at (timestamp)
    pub iat: i64,
    /// Expiration time (timestamp)
    pub exp: i64,
    /// Token type (access or refresh)
    pub token_type: TokenType,
}

/// Token type enumeration
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

impl Claims {
    /// Create new access token claims
    pub fn new_access_token(
        user_id: String,
        name: Option<String>,
        email: Option<String>,
        permissions: Vec<Permission>,
        is_admin: bool,
    ) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(1); // Access token expires in 1 hour

        Self {
            sub: user_id,
            name,
            email,
            permissions: permissions.iter().map(|p| format!("{:?}", p)).collect(),
            is_admin,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            token_type: TokenType::Access,
        }
    }

    /// Create new refresh token claims
    pub fn new_refresh_token(user_id: String) -> Self {
        let now = Utc::now();
        let exp = now + Duration::days(30); // Refresh token expires in 30 days

        Self {
            sub: user_id,
            name: None,
            email: None,
            permissions: vec![],
            is_admin: false,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            token_type: TokenType::Refresh,
        }
    }

    /// Convert claims to User
    pub fn to_user(&self) -> Result<User, AuthError> {
        if self.token_type != TokenType::Access {
            return Err(AuthError::InvalidTokenType);
        }

        let permissions: Result<Vec<Permission>, _> = self
            .permissions
            .iter()
            .map(|p| p.parse::<Permission>())
            .collect();

        let permissions = permissions.map_err(|_| AuthError::InvalidPermissions)?;

        Ok(User::new(self.sub.clone(), self.name.clone(), permissions))
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}

/// JWT token pair (access + refresh)
#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

impl TokenPair {
    pub fn new(access_token: String, refresh_token: String) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600, // 1 hour in seconds
        }
    }
}

/// JWT authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("Token creation failed")]
    TokenCreation,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token type")]
    InvalidTokenType,
    #[error("Invalid permissions")]
    InvalidPermissions,
    #[error("Missing authorization header")]
    MissingAuthHeader,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            AuthError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Invalid username or password",
            ),
            AuthError::MissingCredentials => (
                StatusCode::BAD_REQUEST,
                "missing_credentials",
                "Username and password are required",
            ),
            AuthError::TokenCreation => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "token_creation_failed",
                "Failed to create authentication token",
            ),
            AuthError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "invalid_token",
                "Invalid or malformed token",
            ),
            AuthError::TokenExpired => (
                StatusCode::UNAUTHORIZED,
                "token_expired",
                "Token has expired",
            ),
            AuthError::InvalidTokenType => (
                StatusCode::UNAUTHORIZED,
                "invalid_token_type",
                "Invalid token type for this operation",
            ),
            AuthError::InvalidPermissions => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "invalid_permissions",
                "Invalid permission format",
            ),
            AuthError::MissingAuthHeader => (
                StatusCode::UNAUTHORIZED,
                "missing_auth_header",
                "Authorization header is required",
            ),
        };

        let body = Json(json!({
            "error": error_code,
            "message": message,
        }));

        (status, body).into_response()
    }
}

/// JWT token utilities
pub struct JwtService;

impl JwtService {
    /// Generate access token
    pub fn generate_access_token(
        user_id: String,
        name: Option<String>,
        email: Option<String>,
        permissions: Vec<Permission>,
        is_admin: bool,
    ) -> Result<String, AuthError> {
        let claims = Claims::new_access_token(user_id, name, email, permissions, is_admin);
        encode(&Header::default(), &claims, &KEYS.encoding).map_err(|e| {
            warn!("Failed to encode JWT token: {}", e);
            AuthError::TokenCreation
        })
    }

    /// Generate refresh token
    pub fn generate_refresh_token(user_id: String) -> Result<String, AuthError> {
        let claims = Claims::new_refresh_token(user_id);
        encode(&Header::default(), &claims, &KEYS.encoding).map_err(|e| {
            warn!("Failed to encode refresh token: {}", e);
            AuthError::TokenCreation
        })
    }

    /// Generate token pair
    pub fn generate_token_pair(
        user_id: String,
        name: Option<String>,
        email: Option<String>,
        permissions: Vec<Permission>,
        is_admin: bool,
    ) -> Result<TokenPair, AuthError> {
        let access_token =
            Self::generate_access_token(user_id.clone(), name, email, permissions, is_admin)?;
        let refresh_token = Self::generate_refresh_token(user_id)?;

        Ok(TokenPair::new(access_token, refresh_token))
    }

    /// Verify and decode token
    pub fn verify_token(token: &str) -> Result<Claims, AuthError> {
        let token_data =
            decode::<Claims>(token, &KEYS.decoding, &Validation::default()).map_err(|e| {
                debug!("Token verification failed: {}", e);
                AuthError::InvalidToken
            })?;

        let claims = token_data.claims;

        if claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        Ok(claims)
    }
}

/// FromRequestParts implementation for Claims (JWT extraction)
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .ok_or(AuthError::MissingAuthHeader)?
            .to_str()
            .map_err(|_| AuthError::InvalidToken)?;

        // Parse Bearer token
        let token = if auth_header.starts_with("Bearer ") {
            &auth_header[7..]
        } else {
            return Err(AuthError::InvalidToken);
        };

        // Verify and decode the token
        JwtService::verify_token(token)
    }
}
