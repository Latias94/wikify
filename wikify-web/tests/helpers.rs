//! 集成测试辅助工具
//!
//! 参考 zero-to-production 的测试架构，提供完整的应用测试环境

use serde_json::json;
use std::sync::LazyLock;
use tokio::net::TcpListener;
use tracing::info;
use uuid::Uuid;
use wikify_web::WebConfig;

// 确保tracing只初始化一次
static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .finish();
        tracing::subscriber::set_global_default(subscriber).ok();
    } else {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::WARN)
            .with_writer(std::io::sink)
            .finish();
        tracing::subscriber::set_global_default(subscriber).ok();
    }
});

/// 测试应用实例
pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub api_client: reqwest::Client,
}

impl TestApp {
    /// 发送POST请求到认证状态端点
    pub async fn get_auth_status(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/api/auth/status", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 用户注册
    pub async fn post_register<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/api/auth/register", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 用户登录
    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/api/auth/login", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 健康检查
    pub async fn get_health(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/api/health", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 获取配置
    pub async fn get_config(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/api/config", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 列出仓库
    pub async fn get_repositories(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/api/repositories", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 初始化仓库
    pub async fn post_repositories<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/api/repositories", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 获取仓库信息
    pub async fn get_repository(&self, repository_id: &str) -> reqwest::Response {
        self.api_client
            .get(&format!(
                "{}/api/repositories/{}",
                &self.address, repository_id
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 删除仓库
    pub async fn delete_repository(&self, repository_id: &str) -> reqwest::Response {
        self.api_client
            .delete(&format!(
                "{}/api/repositories/{}",
                &self.address, repository_id
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 重新索引仓库
    pub async fn post_reindex(&self, repository_id: &str) -> reqwest::Response {
        self.api_client
            .post(&format!(
                "{}/api/repositories/{}/reindex",
                &self.address, repository_id
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 聊天查询
    pub async fn post_chat<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/api/chat", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 生成Wiki
    pub async fn post_wiki_generate<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/api/wiki/generate", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 获取Wiki
    pub async fn get_wiki(&self, session_id: &str) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/api/wiki/{}", &self.address, session_id))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 开始研究
    pub async fn post_research_start<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/api/research/start", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 获取研究进度
    pub async fn get_research_progress(&self, session_id: &str) -> reqwest::Response {
        self.api_client
            .get(&format!(
                "{}/api/research/progress/{}",
                &self.address, session_id
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 获取研究模板
    pub async fn get_research_templates(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/api/research/templates", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 带认证头的请求
    pub async fn get_with_auth(&self, path: &str, token: &str) -> reqwest::Response {
        self.api_client
            .get(&format!("{}{}", &self.address, path))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 带认证头的POST请求
    pub async fn post_with_auth<Body>(
        &self,
        path: &str,
        token: &str,
        body: &Body,
    ) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}{}", &self.address, path))
            .header("Authorization", format!("Bearer {}", token))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

/// 启动测试应用
pub async fn spawn_app() -> TestApp {
    spawn_app_with_mode("open").await
}

/// 创建指定权限模式的测试应用
pub async fn spawn_app_with_mode(permission_mode: &str) -> TestApp {
    LazyLock::force(&TRACING);

    // 使用更简单的方法创建测试应用，类似于现有的integration_tests.rs
    let config = WebConfig {
        host: "127.0.0.1".to_string(),
        port: 0, // Let the OS choose a free port
        dev_mode: true,
        static_dir: Some("static".to_string()),
        database_url: Some(":memory:".to_string()), // In-memory SQLite for testing
        permission_mode: Some(permission_mode.to_string()),
    };

    info!(
        "Building test application with permission mode: {}",
        permission_mode
    );

    let state = wikify_web::AppState::new(config.clone()).await.unwrap();
    let app = wikify_web::create_app(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        port,
        api_client: client,
    }
}

/// 测试用户数据
pub struct TestUser {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

impl TestUser {
    pub fn generate() -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            username: format!("test_user_{}", &id[..8]),
            email: format!("test_{}@example.com", &id[..8]),
            password: "test_password_123".to_string(),
            display_name: Some(format!("Test User {}", &id[..8])),
        }
    }

    pub fn to_register_json(&self) -> serde_json::Value {
        json!({
            "username": self.username,
            "email": self.email,
            "password": self.password,
            "display_name": self.display_name
        })
    }

    pub fn to_login_json(&self) -> serde_json::Value {
        json!({
            "username": self.username,
            "password": self.password
        })
    }
}

/// 断言响应是重定向
pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
