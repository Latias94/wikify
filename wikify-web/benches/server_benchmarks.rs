//! Performance benchmarks for Wikify Web Server
//!
//! These benchmarks measure the performance of key server operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;
use tokio::runtime::Runtime;
use wikify_web::{AppState, WebConfig};

/// Benchmark database operations
fn benchmark_database_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let config = WebConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        dev_mode: true,
        static_dir: None,
        database_url: Some(":memory:".to_string()),
    };

    let state = rt.block_on(async { AppState::new(config).await.unwrap() });

    c.bench_function("create_session", |b| {
        b.to_async(&rt).iter(|| async {
            let session_id = uuid::Uuid::new_v4().to_string();
            let repo_info = wikify_core::types::RepoInfo {
                owner: "test".to_string(),
                name: "test-repo".to_string(),
                repo_type: wikify_core::types::RepoType::GitHub,
                url: "https://github.com/test/repo".to_string(),
                access_token: None,
                local_path: Some("/tmp/test".to_string()),
            };

            black_box(state.create_session(session_id, repo_info).await)
        })
    });

    #[cfg(feature = "sqlite")]
    c.bench_function("database_query_repositories", |b| {
        b.to_async(&rt).iter(|| async {
            if let Some(db) = &state.database {
                black_box(db.get_repositories().await)
            } else {
                Ok(vec![])
            }
        })
    });
}

/// Benchmark JSON serialization/deserialization
fn benchmark_json_operations(c: &mut Criterion) {
    let sample_data = json!({
        "session_id": "test-session-123",
        "question": "What is the purpose of this repository?",
        "context": "This is a test context with some sample text that might be used in a real query.",
        "metadata": {
            "timestamp": "2024-01-01T00:00:00Z",
            "user_id": "user-123",
            "repository": "test/repo"
        }
    });

    c.bench_function("json_serialize", |b| {
        b.iter(|| black_box(serde_json::to_string(&sample_data).unwrap()))
    });

    let json_string = serde_json::to_string(&sample_data).unwrap();
    c.bench_function("json_deserialize", |b| {
        b.iter(|| black_box(serde_json::from_str::<serde_json::Value>(&json_string).unwrap()))
    });
}

/// Benchmark session management operations
fn benchmark_session_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let config = WebConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        dev_mode: true,
        static_dir: None,
        database_url: Some(":memory:".to_string()),
    };

    let state = rt.block_on(async { AppState::new(config).await.unwrap() });

    // Create some test sessions
    rt.block_on(async {
        for i in 0..10 {
            let session_id = format!("test-session-{}", i);
            let repo_info = wikify_core::types::RepoInfo {
                owner: "test".to_string(),
                name: format!("test-repo-{}", i),
                repo_type: wikify_core::types::RepoType::GitHub,
                url: format!("https://github.com/test/repo-{}", i),
                access_token: None,
                local_path: Some(format!("/tmp/test-{}", i)),
            };
            let _ = state.create_session(session_id, repo_info).await;
        }
    });

    c.bench_function("get_session", |b| {
        b.to_async(&rt)
            .iter(|| async { black_box(state.get_session("test-session-5").await) })
    });

    c.bench_function("update_session_activity", |b| {
        b.to_async(&rt)
            .iter(|| async { black_box(state.update_session_activity("test-session-5").await) })
    });

    c.bench_function("cleanup_old_sessions", |b| {
        b.to_async(&rt)
            .iter(|| async { black_box(state.cleanup_old_sessions().await) })
    });
}

/// Benchmark configuration loading
fn benchmark_config_operations(c: &mut Criterion) {
    c.bench_function("config_from_env", |b| {
        b.iter(|| black_box(WebConfig::from_env()))
    });

    let config = WebConfig::default();
    c.bench_function("config_address", |b| b.iter(|| black_box(config.address())));
}

criterion_group!(
    benches,
    benchmark_database_operations,
    benchmark_json_operations,
    benchmark_session_operations,
    benchmark_config_operations
);
criterion_main!(benches);
