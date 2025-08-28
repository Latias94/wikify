// Wikify Web Middleware
// 简单的用户上下文中间件，支持可选的用户隔离

use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};
use serde::{Deserialize, Serialize};

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
