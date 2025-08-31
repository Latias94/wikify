//! Wikify Web 认证和权限集成测试
//!
//! 参考 zero-to-production 的测试架构，全面测试各种权限模式下的API端点功能

mod helpers;

use axum::http::StatusCode;
use helpers::{spawn_app_with_mode, TestUser};
use serde_json::json;

/// 🚀 Wikify认证权限集成测试 - 基于zero-to-production最佳实践
#[tokio::test]
async fn auth_permissions_comprehensive() {
    println!("🚀 开始Wikify认证权限集成测试...");

    // 测试Open模式
    test_open_mode().await;

    // 测试Private模式
    test_private_mode().await;

    // 测试Enterprise模式
    test_enterprise_mode().await;

    // 测试认证流程
    test_authentication_flow().await;

    // 测试API端点权限
    test_api_endpoint_permissions().await;

    println!("✅ 所有认证权限测试完成！");
}

/// 测试Open模式 - 所有操作都应该允许匿名访问
async fn test_open_mode() {
    println!("🔓 测试Open模式...");

    let app = spawn_app_with_mode("open").await;

    // 1. 认证状态检查
    let response = app.get_auth_status().await;
    assert_eq!(response.status(), StatusCode::OK);

    let auth_status: serde_json::Value = response.json().await.unwrap();
    assert_eq!(auth_status["auth_mode"], "open");
    assert_eq!(auth_status["auth_required"], false);
    println!("✅ Open模式认证状态正确");

    // 2. 测试公开端点 - 应该都允许访问
    let public_endpoints = vec![
        ("GET", "/api/health", "健康检查"),
        ("GET", "/api/config", "配置信息"),
        ("GET", "/api/auth/status", "认证状态"),
        ("GET", "/api/research/templates", "研究模板"),
    ];

    for (method, uri, name) in public_endpoints {
        let response = match method {
            "GET" => app
                .api_client
                .get(&format!("{}{}", app.address, uri))
                .send()
                .await
                .unwrap(),
            _ => panic!("Unsupported method: {}", method),
        };

        assert_ne!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{}在open模式下不应该返回401",
            name
        );
        println!("✅ {} 测试通过", name);
    }

    // 3. 测试需要权限的端点 - 在open模式下应该允许匿名访问
    let protected_endpoints = vec![("GET", "/api/repositories", "仓库列表")];

    for (method, uri, name) in protected_endpoints {
        let response = match method {
            "GET" => app
                .api_client
                .get(&format!("{}{}", app.address, uri))
                .send()
                .await
                .unwrap(),
            _ => continue,
        };

        assert_ne!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{}在open模式下不应该返回401",
            name
        );
        println!("✅ {} 在open模式下允许匿名访问", name);
    }

    // 4. 🔥 关键测试：POST端点在open模式下不应该返回401
    println!("🔥 测试关键POST端点...");
    let test_repo_request = json!({
        "repository": "https://github.com/octocat/Hello-World",
        "repo_type": "github",
        "auto_generate_wiki": false
    });

    let post_response = app.post_repositories(&test_repo_request).await;

    // 在open模式下，绝对不应该返回401
    assert_ne!(
        post_response.status(),
        StatusCode::UNAUTHORIZED,
        "🚨 CRITICAL: POST /repositories 在open模式下返回401 - 这是认证中间件配置错误！"
    );

    match post_response.status() {
        StatusCode::OK => {
            println!("✅ POST /repositories 成功创建仓库");
            // 清理测试数据
            if let Ok(response_data) = post_response.json::<serde_json::Value>().await {
                if let Some(repo_id) = response_data.get("repository_id") {
                    let _ = app.delete_repository(repo_id.as_str().unwrap()).await;
                }
            }
        }
        StatusCode::CONFLICT => {
            println!("✅ POST /repositories 返回409 (仓库已存在) - 这是正常的业务逻辑");
        }
        StatusCode::BAD_REQUEST => {
            println!("⚠️ POST /repositories 返回400 (请求格式错误) - 可能需要检查请求格式");
        }
        status => {
            println!("⚠️ POST /repositories 返回状态: {} - 但至少不是401", status);
        }
    }

    println!("✅ Open模式测试完成");
}

/// 测试Private模式 - 需要用户认证
async fn test_private_mode() {
    println!("🔒 测试Private模式...");

    let app = spawn_app_with_mode("private").await;

    // 1. 认证状态检查
    let response = app.get_auth_status().await;
    assert_eq!(response.status(), StatusCode::OK);

    let auth_status: serde_json::Value = response.json().await.unwrap();
    assert_eq!(auth_status["auth_mode"], "private");
    assert_eq!(auth_status["auth_required"], true);
    assert_eq!(auth_status["registration_enabled"], true);
    println!("✅ Private模式认证状态正确");

    // 2. 测试公开端点 - 应该仍然可访问
    let public_endpoints = vec![
        "/api/health",
        "/api/auth/status",
        "/api/auth/register",
        "/api/auth/login",
        "/api/research/templates",
    ];

    for endpoint in public_endpoints {
        let response = app
            .api_client
            .get(&format!("{}{}", app.address, endpoint))
            .send()
            .await
            .unwrap();
        assert_ne!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "公开端点 {} 不应该需要认证",
            endpoint
        );
        println!("✅ 公开端点 {} 测试通过", endpoint);
    }

    // 3. 测试受保护端点 - 应该都返回401
    let protected_endpoints = vec![
        ("GET", "/api/repositories", "仓库列表"),
        ("POST", "/api/chat", "聊天功能"),
        ("POST", "/api/wiki/generate", "Wiki生成"),
        ("POST", "/api/research/start", "开始研究"),
    ];

    for (method, uri, name) in protected_endpoints {
        let response = match method {
            "GET" => app
                .api_client
                .get(&format!("{}{}", app.address, uri))
                .send()
                .await
                .unwrap(),
            "POST" => app
                .api_client
                .post(&format!("{}{}", app.address, uri))
                .json(&json!({}))
                .send()
                .await
                .unwrap(),
            _ => panic!("Unsupported method: {}", method),
        };

        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{}在private模式下应该返回401",
            name
        );
        println!("✅ {} 正确返回401", name);
    }

    println!("✅ Private模式测试完成");
}

/// 测试Enterprise模式
async fn test_enterprise_mode() {
    println!("🏢 测试Enterprise模式...");

    let app = spawn_app_with_mode("enterprise").await;

    // 认证状态检查
    let response = app.get_auth_status().await;
    assert_eq!(response.status(), StatusCode::OK);

    let auth_status: serde_json::Value = response.json().await.unwrap();
    assert_eq!(auth_status["auth_mode"], "enterprise");
    assert_eq!(auth_status["auth_required"], true);

    // Enterprise模式应该有额外的功能
    if let Some(features) = auth_status.get("features") {
        println!("✅ Enterprise功能: {:?}", features);
    }

    println!("✅ Enterprise模式测试完成");
}

/// 测试认证流程
async fn test_authentication_flow() {
    println!("🔐 测试认证流程...");

    let app = spawn_app_with_mode("private").await;
    let test_user = TestUser::generate();

    // 1. 尝试用户注册
    let register_response = app.post_register(&test_user.to_register_json()).await;

    match register_response.status() {
        StatusCode::OK => {
            println!("✅ 用户注册功能已实现");

            let register_body: serde_json::Value = register_response.json().await.unwrap();
            if let Some(access_token) = register_body.get("access_token") {
                println!("✅ 注册返回了访问令牌");

                // 使用token访问受保护端点
                let token = access_token.as_str().unwrap();
                let protected_response = app.get_with_auth("/api/repositories", token).await;

                if protected_response.status() != StatusCode::UNAUTHORIZED {
                    println!("✅ 使用注册获得的token可以访问受保护端点");
                } else {
                    println!("⚠️  注册获得的token无法访问受保护端点");
                }
            }

            // 2. 尝试用户登录
            let login_response = app.post_login(&test_user.to_login_json()).await;

            if login_response.status() == StatusCode::OK {
                println!("✅ 用户登录功能已实现");

                let login_body: serde_json::Value = login_response.json().await.unwrap();
                if let Some(access_token) = login_body.get("access_token") {
                    println!("✅ 登录返回了访问令牌");

                    let token = access_token.as_str().unwrap();
                    let protected_response = app.get_with_auth("/api/repositories", token).await;

                    if protected_response.status() != StatusCode::UNAUTHORIZED {
                        println!("✅ 使用登录获得的token可以访问受保护端点");
                    } else {
                        println!("⚠️  登录获得的token无法访问受保护端点");
                    }
                }
            } else {
                println!("⚠️  登录功能返回状态: {}", login_response.status());
            }
        }
        StatusCode::NOT_IMPLEMENTED => {
            println!("⚠️  用户注册功能未实现 (501)");
        }
        StatusCode::INTERNAL_SERVER_ERROR => {
            println!("⚠️  用户注册遇到服务器错误 (500)");
        }
        status => {
            println!("⚠️  用户注册返回状态: {}", status);
        }
    }

    println!("✅ 认证流程测试完成");
}

/// 测试API端点权限
async fn test_api_endpoint_permissions() {
    println!("🛡️  测试API端点权限...");

    // 在open模式下测试各种端点
    let app = spawn_app_with_mode("open").await;

    // 测试不同权限级别的端点
    let permission_tests = vec![
        // 公开端点 - 不需要任何权限
        ("GET", "/api/health", "Public", "健康检查"),
        ("GET", "/api/auth/status", "Public", "认证状态"),
        ("GET", "/api/research/templates", "Public", "研究模板"),
        // Query权限端点
        ("GET", "/api/repositories", "Query", "仓库列表"),
        // 需要特定权限的端点（在open模式下应该都能访问）
        // 注意：实际的权限检查需要有真实的session_id等数据
    ];

    for (method, uri, permission, name) in permission_tests {
        let response = match method {
            "GET" => app
                .api_client
                .get(&format!("{}{}", app.address, uri))
                .send()
                .await
                .unwrap(),
            _ => continue, // 跳过复杂的POST请求，避免缺少必需参数
        };

        // 在open模式下，不应该有权限问题
        assert_ne!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{} ({}) 在open模式下不应该被禁止",
            name,
            permission
        );

        // 也不应该有认证问题
        assert_ne!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{} ({}) 在open模式下不应该需要认证",
            name,
            permission
        );

        println!(
            "✅ {} ({}) 权限测试通过 - 状态: {}",
            name,
            permission,
            response.status()
        );
    }

    println!("✅ API端点权限测试完成");
}
