// Wikify Web Middleware
// 简单的用户上下文中间件，支持可选的用户隔离

use crate::{auth::jwt::JwtService, AppState};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// 用户上下文信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// 用户ID，默认为 "default"
    pub user_id: String,
    /// 显示名称
    pub display_name: Option<String>,
}

impl Default for UserContext {
    fn default() -> Self {
        Self {
            user_id: "default".to_string(),
            display_name: Some("Default User".to_string()),
        }
    }
}

impl UserContext {
    /// 从请求中提取用户上下文
    pub fn from_request(headers: &HeaderMap) -> Self {
        // 1. 检查 X-User-ID Header
        if let Some(user_id) = headers.get("X-User-ID") {
            if let Ok(user_id_str) = user_id.to_str() {
                return Self {
                    user_id: user_id_str.to_string(),
                    display_name: headers
                        .get("X-User-Name")
                        .and_then(|name| name.to_str().ok())
                        .map(|s| s.to_string()),
                };
            }
        }

        // 2. 检查 Cookie (未来扩展)
        if let Some(cookie_header) = headers.get("Cookie") {
            if let Ok(cookie_str) = cookie_header.to_str() {
                if let Some(user_id) = Self::extract_user_from_cookie(cookie_str) {
                    return Self {
                        user_id,
                        display_name: None,
                    };
                }
            }
        }

        // 3. 默认用户
        Self::default()
    }

    /// 从 Cookie 中提取用户ID (简单实现)
    fn extract_user_from_cookie(cookie_str: &str) -> Option<String> {
        for cookie in cookie_str.split(';') {
            let cookie = cookie.trim();
            if let Some(value) = cookie.strip_prefix("wikify_user_id=") {
                return Some(value.to_string());
            }
        }
        None
    }

    /// 检查是否为默认用户
    pub fn is_default_user(&self) -> bool {
        self.user_id == "default"
    }

    /// 获取用户显示名称
    pub fn display_name(&self) -> String {
        self.display_name.clone().unwrap_or_else(|| {
            if self.is_default_user() {
                "Default User".to_string()
            } else {
                self.user_id.clone()
            }
        })
    }
}

/// 用户上下文中间件
pub async fn user_context_middleware(mut request: Request, next: Next) -> Response {
    // 从请求中提取用户上下文
    let user_context = UserContext::from_request(request.headers());

    // 将用户上下文添加到请求扩展中
    request.extensions_mut().insert(user_context);

    // 继续处理请求
    next.run(request).await
}

/// 从请求扩展中获取用户上下文的辅助函数
pub fn get_user_context(request: &Request) -> UserContext {
    request
        .extensions()
        .get::<UserContext>()
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderName, HeaderValue};

    #[test]
    fn test_default_user_context() {
        let headers = HeaderMap::new();
        let context = UserContext::from_request(&headers);

        assert_eq!(context.user_id, "default");
        assert!(context.is_default_user());
        assert_eq!(context.display_name(), "Default User");
    }

    #[test]
    fn test_user_context_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-user-id"),
            HeaderValue::from_static("john_doe"),
        );
        headers.insert(
            HeaderName::from_static("x-user-name"),
            HeaderValue::from_static("John Doe"),
        );

        let context = UserContext::from_request(&headers);

        assert_eq!(context.user_id, "john_doe");
        assert!(!context.is_default_user());
        assert_eq!(context.display_name(), "John Doe");
    }

    #[test]
    fn test_user_context_from_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("cookie"),
            HeaderValue::from_static("wikify_user_id=jane_doe; other_cookie=value"),
        );

        let context = UserContext::from_request(&headers);

        assert_eq!(context.user_id, "jane_doe");
        assert!(!context.is_default_user());
    }

    #[test]
    fn test_extract_user_from_cookie() {
        let cookie_str = "session_id=abc123; wikify_user_id=test_user; theme=dark";
        let user_id = UserContext::extract_user_from_cookie(cookie_str);

        assert_eq!(user_id, Some("test_user".to_string()));
    }

    #[test]
    fn test_extract_user_from_cookie_not_found() {
        let cookie_str = "session_id=abc123; theme=dark";
        let user_id = UserContext::extract_user_from_cookie(cookie_str);

        assert_eq!(user_id, None);
    }
}

/// Authentication middleware that requires valid JWT token or API key
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 检查权限模式
    let permission_mode = state
        .config
        .permission_mode
        .as_ref()
        .unwrap_or(&"open".to_string())
        .clone();

    // 在open模式下，跳过认证检查
    if permission_mode == "open" {
        return Ok(next.run(request).await);
    }

    let headers = request.headers();

    // Extract Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(auth_str) if auth_str.starts_with("Bearer ") => {
            let token = &auth_str[7..]; // Remove "Bearer " prefix

            // Try JWT token first
            match JwtService::verify_token(token) {
                Ok(claims) => {
                    debug!("Valid JWT token for user: {}", claims.sub);

                    // Try to get user from user service
                    match state.user_service.get_user_by_id(&claims.sub).await {
                        Some(user_data) => {
                            // Convert to our User type and add to request extensions
                            let user = crate::auth::User::new(
                                user_data.id.clone(),
                                user_data.display_name.clone(),
                                user_data.permissions.clone(),
                            );
                            request.extensions_mut().insert(user);
                            return Ok(next.run(request).await);
                        }
                        None => {
                            warn!("User not found for valid token: {}", claims.sub);
                            return Err(StatusCode::UNAUTHORIZED);
                        }
                    }
                }
                Err(_) => {
                    // JWT validation failed, try as API key
                    debug!("JWT validation failed, trying as API key");
                }
            }

            // Try API key authentication
            match state.api_key_service.authenticate_api_key(token).await {
                Ok(Some(user_data)) => {
                    debug!(
                        "Valid API key authentication for: {:?}",
                        user_data.display_name
                    );

                    // Convert to our User type and add to request extensions
                    let user = crate::auth::User::new(
                        user_data.id.clone(),
                        user_data.display_name.clone(),
                        user_data.permissions.clone(),
                    );
                    request.extensions_mut().insert(user);
                    Ok(next.run(request).await)
                }
                Ok(None) => {
                    debug!("API key not found");
                    Err(StatusCode::UNAUTHORIZED)
                }
                Err(e) => {
                    debug!("API key authentication failed: {}", e);
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
        }
        Some(auth_str) if auth_str.starts_with("ApiKey ") => {
            let api_key = &auth_str[7..]; // Remove "ApiKey " prefix

            // Try API key authentication
            match state.api_key_service.authenticate_api_key(api_key).await {
                Ok(Some(user_data)) => {
                    debug!(
                        "Valid API key authentication for: {:?}",
                        user_data.display_name
                    );

                    // Convert to our User type and add to request extensions
                    let user = crate::auth::User::new(
                        user_data.id.clone(),
                        user_data.display_name.clone(),
                        user_data.permissions.clone(),
                    );
                    request.extensions_mut().insert(user);
                    Ok(next.run(request).await)
                }
                Ok(None) => {
                    debug!("API key not found");
                    Err(StatusCode::UNAUTHORIZED)
                }
                Err(e) => {
                    debug!("API key authentication failed: {}", e);
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
        }
        _ => {
            debug!("Missing or invalid Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
