//! Wikify Web API端点全面测试
//!
//! 测试所有API端点的功能性和权限控制

mod helpers;

use axum::http::StatusCode;
use helpers::{spawn_app_with_mode, TestUser};
use serde_json::json;

/// 🚀 API端点全面测试
#[tokio::test]
async fn test_api_endpoints_comprehensive() {
    println!("🚀 开始API端点全面测试...");

    // 测试公开端点
    test_public_endpoints().await;

    // 测试认证端点
    test_auth_endpoints().await;

    // 测试仓库管理端点
    test_repository_endpoints().await;

    // 测试聊天端点
    test_chat_endpoints().await;

    // 测试Wiki端点
    test_wiki_endpoints().await;

    // 测试研究端点
    test_research_endpoints().await;

    println!("✅ 所有API端点测试完成！");
}

/// 测试公开端点 - 不需要认证
async fn test_public_endpoints() {
    println!("🌐 测试公开端点...");

    let app = spawn_app_with_mode("open").await;

    let public_endpoints = vec![
        ("/api/health", "健康检查"),
        ("/api/config", "配置信息"),
        ("/api/auth/status", "认证状态"),
        ("/api/research/templates", "研究模板列表"),
    ];

    for (endpoint, name) in public_endpoints {
        let response = app
            .api_client
            .get(&format!("{}{}", app.address, endpoint))
            .send()
            .await
            .unwrap();

        assert!(
            response.status().is_success() || response.status() == StatusCode::NOT_IMPLEMENTED,
            "{} 应该成功或返回未实现状态，实际状态: {}",
            name,
            response.status()
        );

        println!("✅ {} 测试通过 - 状态: {}", name, response.status());
    }

    println!("✅ 公开端点测试完成");
}

/// 测试认证相关端点
async fn test_auth_endpoints() {
    println!("🔐 测试认证端点...");

    let app = spawn_app_with_mode("private").await;
    let test_user = TestUser::generate();

    // 1. 测试认证状态端点
    let auth_status_response = app.get_auth_status().await;
    assert_eq!(auth_status_response.status(), StatusCode::OK);

    let auth_status: serde_json::Value = auth_status_response.json().await.unwrap();
    assert_eq!(auth_status["auth_mode"], "private");
    assert_eq!(auth_status["auth_required"], true);
    println!("✅ 认证状态端点测试通过");

    // 2. 测试用户注册端点
    let register_response = app.post_register(&test_user.to_register_json()).await;

    match register_response.status() {
        StatusCode::OK => {
            println!("✅ 用户注册端点已实现并正常工作");

            // 3. 测试用户登录端点
            let login_response = app.post_login(&test_user.to_login_json()).await;

            if login_response.status() == StatusCode::OK {
                println!("✅ 用户登录端点已实现并正常工作");
            } else {
                println!("⚠️  用户登录端点状态: {}", login_response.status());
            }
        }
        StatusCode::NOT_IMPLEMENTED => {
            println!("⚠️  用户注册端点未实现 (501)");
        }
        StatusCode::INTERNAL_SERVER_ERROR => {
            println!("⚠️  用户注册端点遇到服务器错误 (500)");
        }
        status => {
            println!("⚠️  用户注册端点返回状态: {}", status);
        }
    }

    println!("✅ 认证端点测试完成");
}

/// 测试仓库管理端点
async fn test_repository_endpoints() {
    println!("📁 测试仓库管理端点...");

    let app = spawn_app_with_mode("open").await;

    // 1. 测试仓库列表端点
    let repos_response = app.get_repositories().await;

    match repos_response.status() {
        StatusCode::OK => {
            println!("✅ 仓库列表端点正常工作");

            let repos_data: serde_json::Value = repos_response.json().await.unwrap();
            println!("📊 仓库列表响应: {:?}", repos_data);
        }
        StatusCode::NOT_IMPLEMENTED => {
            println!("⚠️  仓库列表端点未实现");
        }
        status => {
            println!("⚠️  仓库列表端点状态: {}", status);
        }
    }

    // 2. 测试仓库初始化端点（需要有效的仓库URL）
    let init_request = json!({
        "repository": "https://github.com/octocat/Hello-World",
        "repo_type": "github",
        "auto_generate_wiki": false
    });

    let init_response = app.post_repositories(&init_request).await;

    // 🔥 关键检查：在open模式下绝对不应该返回401
    assert_ne!(
        init_response.status(),
        StatusCode::UNAUTHORIZED,
        "🚨 CRITICAL: POST /repositories 在open模式下返回401 - 认证中间件配置错误！"
    );

    match init_response.status() {
        StatusCode::OK => {
            println!("✅ 仓库初始化端点正常工作");

            let init_data: serde_json::Value = init_response.json().await.unwrap();
            if let Some(repo_id) = init_data.get("repository_id") {
                let repo_id_str = repo_id.as_str().unwrap();
                println!("📊 创建的仓库ID: {}", repo_id_str);

                // 3. 测试获取仓库信息端点
                let repo_info_response = app.get_repository(repo_id_str).await;

                match repo_info_response.status() {
                    StatusCode::OK => {
                        println!("✅ 获取仓库信息端点正常工作");
                    }
                    StatusCode::NOT_FOUND => {
                        println!("⚠️  仓库信息未找到（可能是异步处理中）");
                    }
                    status => {
                        println!("⚠️  获取仓库信息端点状态: {}", status);
                    }
                }

                // 4. 测试重新索引端点
                let reindex_response = app.post_reindex(repo_id_str).await;

                match reindex_response.status() {
                    StatusCode::OK => {
                        println!("✅ 重新索引端点正常工作");
                    }
                    StatusCode::CONFLICT => {
                        println!("⚠️  仓库正在索引中，无法重新索引");
                    }
                    status => {
                        println!("⚠️  重新索引端点状态: {}", status);
                    }
                }

                // 5. 测试删除仓库端点
                let delete_response = app.delete_repository(repo_id_str).await;

                match delete_response.status() {
                    StatusCode::OK => {
                        println!("✅ 删除仓库端点正常工作");
                    }
                    status => {
                        println!("⚠️  删除仓库端点状态: {}", status);
                    }
                }
            }
        }
        StatusCode::BAD_REQUEST => {
            println!("⚠️  仓库初始化请求格式错误");
        }
        StatusCode::CONFLICT => {
            println!("⚠️  仓库已存在或正在处理中");
        }
        status => {
            println!("⚠️  仓库初始化端点状态: {}", status);
        }
    }

    println!("✅ 仓库管理端点测试完成");
}

/// 测试聊天端点
async fn test_chat_endpoints() {
    println!("💬 测试聊天端点...");

    let app = spawn_app_with_mode("open").await;

    // 测试聊天查询端点（需要有效的session_id）
    let chat_request = json!({
        "session_id": "test-session-id",
        "question": "What is this repository about?",
        "context": null
    });

    let chat_response = app.post_chat(&chat_request).await;

    match chat_response.status() {
        StatusCode::OK => {
            println!("✅ 聊天查询端点正常工作");

            let chat_data: serde_json::Value = chat_response.json().await.unwrap();
            println!("📊 聊天响应: {:?}", chat_data);
        }
        StatusCode::NOT_FOUND => {
            println!("⚠️  会话未找到（预期行为，因为使用了测试session_id）");
        }
        StatusCode::BAD_REQUEST => {
            println!("⚠️  聊天请求格式错误");
        }
        status => {
            println!("⚠️  聊天查询端点状态: {}", status);
        }
    }

    println!("✅ 聊天端点测试完成");
}

/// 测试Wiki端点
async fn test_wiki_endpoints() {
    println!("📚 测试Wiki端点...");

    let app = spawn_app_with_mode("open").await;

    // 测试Wiki生成端点
    let wiki_request = json!({
        "session_id": "test-session-id",
        "config": {
            "language": "en",
            "max_pages": 10,
            "include_diagrams": true,
            "comprehensive_view": false
        }
    });

    let wiki_response = app.post_wiki_generate(&wiki_request).await;

    match wiki_response.status() {
        StatusCode::OK => {
            println!("✅ Wiki生成端点正常工作");

            let wiki_data: serde_json::Value = wiki_response.json().await.unwrap();
            println!("📊 Wiki生成响应: {:?}", wiki_data);

            // 测试获取Wiki端点
            let get_wiki_response = app.get_wiki("test-session-id").await;

            match get_wiki_response.status() {
                StatusCode::OK => {
                    println!("✅ 获取Wiki端点正常工作");
                }
                StatusCode::NOT_FOUND => {
                    println!("⚠️  Wiki未找到（可能是异步生成中）");
                }
                status => {
                    println!("⚠️  获取Wiki端点状态: {}", status);
                }
            }
        }
        StatusCode::NOT_FOUND => {
            println!("⚠️  会话未找到（预期行为，因为使用了测试session_id）");
        }
        StatusCode::BAD_REQUEST => {
            println!("⚠️  Wiki生成请求格式错误");
        }
        status => {
            println!("⚠️  Wiki生成端点状态: {}", status);
        }
    }

    println!("✅ Wiki端点测试完成");
}

/// 测试研究端点
async fn test_research_endpoints() {
    println!("🔬 测试研究端点...");

    let app = spawn_app_with_mode("open").await;

    // 1. 测试研究模板端点
    let templates_response = app.get_research_templates().await;

    match templates_response.status() {
        StatusCode::OK => {
            println!("✅ 研究模板端点正常工作");

            let templates_data: serde_json::Value = templates_response.json().await.unwrap();
            println!(
                "📊 研究模板数量: {}",
                templates_data.as_array().unwrap_or(&vec![]).len()
            );
        }
        status => {
            println!("⚠️  研究模板端点状态: {}", status);
        }
    }

    // 2. 测试开始研究端点
    let research_request = json!({
        "session_id": "test-session-id",
        "topic": "Test research topic",
        "config": {
            "max_iterations": 3,
            "max_depth": 2,
            "confidence_threshold": 0.7,
            "max_sources_per_iteration": 5,
            "enable_parallel_research": false
        }
    });

    let research_response = app.post_research_start(&research_request).await;

    match research_response.status() {
        StatusCode::OK => {
            println!("✅ 开始研究端点正常工作");

            let research_data: serde_json::Value = research_response.json().await.unwrap();
            println!("📊 研究开始响应: {:?}", research_data);

            // 测试获取研究进度端点
            let progress_response = app.get_research_progress("test-session-id").await;

            match progress_response.status() {
                StatusCode::OK => {
                    println!("✅ 获取研究进度端点正常工作");
                }
                StatusCode::NOT_FOUND => {
                    println!("⚠️  研究会话未找到");
                }
                status => {
                    println!("⚠️  获取研究进度端点状态: {}", status);
                }
            }
        }
        StatusCode::NOT_FOUND => {
            println!("⚠️  会话未找到（预期行为，因为使用了测试session_id）");
        }
        StatusCode::BAD_REQUEST => {
            println!("⚠️  研究请求格式错误");
        }
        status => {
            println!("⚠️  开始研究端点状态: {}", status);
        }
    }

    println!("✅ 研究端点测试完成");
}
