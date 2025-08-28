// Wikify Web Simple Database Service
// 简化的数据库实现，专注于基本功能

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};

use crate::{WebError, WebResult};

/// 简化的数据库服务
pub struct SimpleDatabaseService {
    pool: SqlitePool,
}

impl SimpleDatabaseService {
    /// 创建新的数据库服务
    pub async fn new(database_url: &str) -> WebResult<Self> {
        tracing::info!("🔗 Attempting to connect to database: {}", database_url);

        // 尝试使用连接选项进行更精细的控制
        let pool = if database_url.starts_with("sqlite:") && !database_url.contains(":memory:") {
            let db_path = database_url.strip_prefix("sqlite:").unwrap_or(database_url);
            tracing::debug!("🔧 Using SQLite connection options for file: {}", db_path);

            // 尝试创建父目录
            if let Some(parent) = std::path::Path::new(db_path).parent() {
                if !parent.exists() {
                    tracing::info!("📁 Creating parent directory: {}", parent.display());
                    std::fs::create_dir_all(parent).map_err(|e| {
                        tracing::error!("❌ Failed to create directory: {}", e);
                        WebError::Database(format!("Failed to create directory: {}", e))
                    })?;
                }
            }

            // 尝试创建空文件（如果不存在）
            let path = std::path::Path::new(db_path);
            if !path.exists() {
                tracing::debug!("📄 Creating empty database file: {}", db_path);
                match std::fs::File::create(path) {
                    Ok(_) => tracing::debug!("✅ Empty database file created"),
                    Err(e) => {
                        tracing::warn!("⚠️  Could not create empty file: {}", e);
                        // 继续尝试连接，SQLite 可能能够创建文件
                    }
                }
            }

            let options = SqliteConnectOptions::new()
                .filename(db_path)
                .create_if_missing(true);

            tracing::debug!("🔗 Attempting connection with options...");
            SqlitePool::connect_with(options).await.map_err(|e| {
                tracing::error!("❌ Database connection failed with options: {}", e);

                // 尝试使用标准连接作为后备
                tracing::info!("🔄 Trying standard connection as fallback...");
                WebError::Database(format!("Failed to connect to database: {}", e))
            })?
        } else {
            // 对于内存数据库，使用标准连接
            tracing::debug!("🔧 Using standard connection for: {}", database_url);
            SqlitePool::connect(database_url).await.map_err(|e| {
                tracing::error!("❌ Database connection failed: {}", e);
                WebError::Database(format!("Failed to connect to database: {}", e))
            })?
        };

        tracing::info!("✅ Database connection established successfully");

        // 创建表
        tracing::info!("🏗️  Creating database tables...");
        Self::create_tables(&pool).await?;
        tracing::info!("✅ Database tables created successfully");

        Ok(Self { pool })
    }

    /// 创建数据库表
    async fn create_tables(pool: &SqlitePool) -> WebResult<()> {
        tracing::debug!("📋 Creating repositories table...");
        // 创建仓库表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS repositories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                repo_path TEXT NOT NULL,
                repo_type TEXT NOT NULL,
                status TEXT DEFAULT 'created',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                last_indexed_at TEXT
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to create repositories table: {}", e);
            WebError::Database(format!("Failed to create repositories table: {}", e))
        })?;
        tracing::debug!("✅ Repositories table created successfully");

        tracing::debug!("📋 Creating sessions table...");
        // 创建会话表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                repository_id TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                last_activity TEXT DEFAULT CURRENT_TIMESTAMP,
                is_active INTEGER DEFAULT 1
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to create sessions table: {}", e);
            WebError::Database(format!("Failed to create sessions table: {}", e))
        })?;
        tracing::debug!("✅ Sessions table created successfully");

        tracing::debug!("📋 Creating query_history table...");
        // 创建查询历史表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS query_history (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                question TEXT NOT NULL,
                answer TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to create query_history table: {}", e);
            WebError::Database(format!("Failed to create query_history table: {}", e))
        })?;
        tracing::debug!("✅ Query history table created successfully");

        Ok(())
    }

    /// 保存仓库信息
    pub async fn save_repository(&self, repo: &SimpleRepository) -> WebResult<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO repositories (id, name, repo_path, repo_type, status, created_at, last_indexed_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&repo.id)
        .bind(&repo.name)
        .bind(&repo.repo_path)
        .bind(&repo.repo_type)
        .bind(&repo.status)
        .bind(&repo.created_at.to_rfc3339())
        .bind(repo.last_indexed_at.as_ref().map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await
        .map_err(|e| WebError::Database(format!("Failed to save repository: {}", e)))?;

        Ok(())
    }

    /// 获取仓库列表
    pub async fn get_repositories(&self) -> WebResult<Vec<SimpleRepository>> {
        let rows = sqlx::query("SELECT id, name, repo_path, repo_type, status, created_at, last_indexed_at FROM repositories ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WebError::Database(format!("Failed to get repositories: {}", e)))?;

        let mut repositories = Vec::new();
        for row in rows {
            let created_at_str: String = row
                .try_get("created_at")
                .unwrap_or_else(|_| Utc::now().to_rfc3339());
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let last_indexed_at = row
                .try_get::<Option<String>, _>("last_indexed_at")
                .unwrap_or(None)
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            repositories.push(SimpleRepository {
                id: row.try_get("id").unwrap_or_default(),
                name: row.try_get("name").unwrap_or_default(),
                repo_path: row.try_get("repo_path").unwrap_or_default(),
                repo_type: row.try_get("repo_type").unwrap_or_default(),
                status: row
                    .try_get("status")
                    .unwrap_or_else(|_| "created".to_string()),
                created_at,
                last_indexed_at,
            });
        }

        Ok(repositories)
    }

    /// 保存会话
    pub async fn save_session(&self, session: &SimpleSession) -> WebResult<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO sessions (id, repository_id, created_at, last_activity, is_active) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&session.id)
        .bind(&session.repository_id)
        .bind(&session.created_at.to_rfc3339())
        .bind(&session.last_activity.to_rfc3339())
        .bind(if session.is_active { 1 } else { 0 })
        .execute(&self.pool)
        .await
        .map_err(|e| WebError::Database(format!("Failed to save session: {}", e)))?;

        Ok(())
    }

    /// 获取会话列表
    pub async fn get_sessions(&self) -> WebResult<Vec<SimpleSession>> {
        let rows = sqlx::query("SELECT id, repository_id, created_at, last_activity, is_active FROM sessions WHERE is_active = 1 ORDER BY last_activity DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WebError::Database(format!("Failed to get sessions: {}", e)))?;

        let mut sessions = Vec::new();
        for row in rows {
            let created_at_str: String = row
                .try_get("created_at")
                .unwrap_or_else(|_| Utc::now().to_rfc3339());
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let last_activity_str: String = row
                .try_get("last_activity")
                .unwrap_or_else(|_| Utc::now().to_rfc3339());
            let last_activity = DateTime::parse_from_rfc3339(&last_activity_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            sessions.push(SimpleSession {
                id: row.try_get("id").unwrap_or_default(),
                repository_id: row.try_get("repository_id").unwrap_or_default(),
                created_at,
                last_activity,
                is_active: row.try_get::<i32, _>("is_active").unwrap_or(1) == 1,
            });
        }

        Ok(sessions)
    }

    /// 保存查询记录
    pub async fn save_query(&self, query: &SimpleQuery) -> WebResult<()> {
        sqlx::query(
            "INSERT INTO query_history (id, session_id, question, answer, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&query.id)
        .bind(&query.session_id)
        .bind(&query.question)
        .bind(&query.answer)
        .bind(&query.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WebError::Database(format!("Failed to save query: {}", e)))?;

        Ok(())
    }

    /// 获取查询历史
    pub async fn get_query_history(
        &self,
        session_id: Option<&str>,
        limit: i32,
    ) -> WebResult<Vec<SimpleQuery>> {
        let rows = if let Some(sid) = session_id {
            sqlx::query("SELECT id, session_id, question, answer, created_at FROM query_history WHERE session_id = ? ORDER BY created_at DESC LIMIT ?")
                .bind(sid)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query("SELECT id, session_id, question, answer, created_at FROM query_history ORDER BY created_at DESC LIMIT ?")
                .bind(limit)
                .fetch_all(&self.pool)
                .await
        }
        .map_err(|e| WebError::Database(format!("Failed to get query history: {}", e)))?;

        let mut queries = Vec::new();
        for row in rows {
            let created_at_str: String = row
                .try_get("created_at")
                .unwrap_or_else(|_| Utc::now().to_rfc3339());
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            queries.push(SimpleQuery {
                id: row.try_get("id").unwrap_or_default(),
                session_id: row.try_get("session_id").ok(),
                question: row.try_get("question").unwrap_or_default(),
                answer: row.try_get("answer").unwrap_or_default(),
                created_at,
            });
        }

        Ok(queries)
    }
}

/// 简化的仓库信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleRepository {
    pub id: String,
    pub name: String,
    pub repo_path: String,
    pub repo_type: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub last_indexed_at: Option<DateTime<Utc>>,
}

/// 简化的会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleSession {
    pub id: String,
    pub repository_id: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_active: bool,
}

/// 简化的查询记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleQuery {
    pub id: String,
    pub session_id: Option<String>,
    pub question: String,
    pub answer: String,
    pub created_at: DateTime<Utc>,
}
