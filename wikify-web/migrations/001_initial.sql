-- Wikify v0.1.1 数据库初始化
-- 简化的数据库设计，支持零配置启动和可选用户隔离

-- 仓库表
CREATE TABLE repositories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    repo_path TEXT NOT NULL,
    repo_type TEXT NOT NULL CHECK (repo_type IN ('local', 'git', 'github')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_indexed_at DATETIME,
    status TEXT DEFAULT 'created' CHECK (status IN ('created', 'indexing', 'indexed', 'failed', 'archived')),
    metadata TEXT DEFAULT '{}' -- JSON 格式的元数据
);

-- 会话表
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT DEFAULT 'default', -- 用户标识，默认为 'default'
    repository_id TEXT NOT NULL,
    name TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_activity DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
);

-- 查询历史表
CREATE TABLE query_history (
    id TEXT PRIMARY KEY,
    user_id TEXT DEFAULT 'default',
    repository_id TEXT NOT NULL,
    session_id TEXT,
    question TEXT NOT NULL,
    answer TEXT NOT NULL,
    sources TEXT DEFAULT '[]', -- JSON 格式的源文档
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    response_time_ms INTEGER,
    similarity_threshold REAL,
    chunks_retrieved INTEGER,
    FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL
);

-- 用户表 (可选，仅在简单多用户模式下使用)
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 索引优化
CREATE INDEX idx_repositories_status ON repositories(status);
CREATE INDEX idx_repositories_created ON repositories(created_at DESC);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_repository_id ON sessions(repository_id);
CREATE INDEX idx_sessions_last_activity ON sessions(last_activity DESC);

CREATE INDEX idx_query_history_user_id ON query_history(user_id);
CREATE INDEX idx_query_history_repository_id ON query_history(repository_id);
CREATE INDEX idx_query_history_session_id ON query_history(session_id);
CREATE INDEX idx_query_history_created ON query_history(created_at DESC);

CREATE INDEX idx_users_last_seen ON users(last_seen DESC);

-- 插入默认用户 (单用户模式)
INSERT INTO users (id, display_name) VALUES ('default', 'Default User');
