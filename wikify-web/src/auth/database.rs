//! Database-backed user storage implementation

use super::{
    jwt::AuthError,
    users::{UserData, UserStore},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use wikify_applications::Permission;

/// Database user record
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct UserRecord {
    id: String,
    username: String,
    email: String,
    display_name: Option<String>,
    password_hash: String,
    permissions: String, // JSON array
    is_admin: bool,
    created_at: String, // ISO 8601 string
}

impl UserRecord {
    /// Convert to UserData
    fn to_user_data(&self) -> Result<UserData, AuthError> {
        let permissions: Vec<String> =
            serde_json::from_str(&self.permissions).map_err(|_| AuthError::InvalidPermissions)?;

        let permissions: Result<Vec<Permission>, _> = permissions
            .iter()
            .map(|p| p.parse::<Permission>())
            .collect();

        let permissions = permissions.map_err(|_| AuthError::InvalidPermissions)?;

        let created_at: DateTime<Utc> = self
            .created_at
            .parse()
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(UserData {
            id: self.id.clone(),
            username: self.username.clone(),
            email: self.email.clone(),
            display_name: self.display_name.clone(),
            password_hash: self.password_hash.clone(),
            permissions,
            is_admin: self.is_admin,
            created_at,
        })
    }

    /// Create from UserData
    fn from_user_data(user: &UserData) -> Self {
        let permissions: Vec<String> = user
            .permissions
            .iter()
            .map(|p| format!("{:?}", p))
            .collect();

        let permissions_json =
            serde_json::to_string(&permissions).unwrap_or_else(|_| "[]".to_string());

        Self {
            id: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            display_name: user.display_name.clone(),
            password_hash: user.password_hash.clone(),
            permissions: permissions_json,
            is_admin: user.is_admin,
            created_at: user.created_at.to_rfc3339(),
        }
    }
}

/// Database-backed user store
#[derive(Debug, Clone)]
pub struct DatabaseUserStore {
    pool: SqlitePool,
    // Cache for frequently accessed users
    cache: Arc<RwLock<HashMap<String, UserData>>>,
}

impl DatabaseUserStore {
    /// Create new database user store
    pub async fn new(pool: SqlitePool) -> Result<Self, AuthError> {
        let store = Self {
            pool,
            cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Create users table
        store.create_tables().await?;

        // Create default admin user if not exists
        store.ensure_default_admin().await?;

        Ok(store)
    }

    /// Create database tables
    async fn create_tables(&self) -> Result<(), AuthError> {
        let query = r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                email TEXT UNIQUE NOT NULL,
                display_name TEXT,
                password_hash TEXT NOT NULL,
                permissions TEXT NOT NULL DEFAULT '[]',
                is_admin BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
            CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
        "#;

        sqlx::query(query).execute(&self.pool).await.map_err(|e| {
            error!("Failed to create users table: {}", e);
            AuthError::TokenCreation
        })?;

        info!("Users table created successfully");
        Ok(())
    }

    /// Ensure default admin user exists
    async fn ensure_default_admin(&self) -> Result<(), AuthError> {
        // Check if admin user already exists
        let existing = sqlx::query("SELECT COUNT(*) as count FROM users WHERE username = ?")
            .bind("admin")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to check for admin user: {}", e);
                AuthError::TokenCreation
            })?;

        let count: i64 = existing.get("count");
        if count > 0 {
            debug!("Admin user already exists");
            return Ok(());
        }

        // Create default admin user
        let admin_user = UserData::new(
            "admin".to_string(),
            "admin@wikify.local".to_string(),
            "admin123", // Default password - should be changed in production
            Some("Administrator".to_string()),
            vec![Permission::Admin],
            true,
        )?;

        self.insert_user(&admin_user).await?;
        info!("Created default admin user");
        Ok(())
    }

    /// Insert user into database
    async fn insert_user(&self, user: &UserData) -> Result<(), AuthError> {
        let record = UserRecord::from_user_data(user);

        let query = r#"
            INSERT INTO users (id, username, email, display_name, password_hash, permissions, is_admin, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(&record.id)
            .bind(&record.username)
            .bind(&record.email)
            .bind(&record.display_name)
            .bind(&record.password_hash)
            .bind(&record.permissions)
            .bind(record.is_admin)
            .bind(&record.created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to insert user: {}", e);
                AuthError::InvalidCredentials
            })?;

        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(user.username.clone(), user.clone());

        debug!("User inserted successfully: {}", user.username);
        Ok(())
    }

    /// Get user by username
    pub async fn get_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserData>, AuthError> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(user) = cache.get(username) {
                return Ok(Some(user.clone()));
            }
        }

        // Query database
        let query = "SELECT * FROM users WHERE username = ?";
        let row = sqlx::query(query)
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to query user by username: {}", e);
                AuthError::TokenCreation
            })?;

        if let Some(row) = row {
            let record = UserRecord {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                display_name: row.get("display_name"),
                password_hash: row.get("password_hash"),
                permissions: row.get("permissions"),
                is_admin: row.get("is_admin"),
                created_at: row.get("created_at"),
            };

            let user_data = record.to_user_data()?;

            // Update cache
            let mut cache = self.cache.write().await;
            cache.insert(username.to_string(), user_data.clone());

            Ok(Some(user_data))
        } else {
            Ok(None)
        }
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<UserData>, AuthError> {
        let query = "SELECT * FROM users WHERE id = ?";
        let row = sqlx::query(query)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to query user by ID: {}", e);
                AuthError::TokenCreation
            })?;

        if let Some(row) = row {
            let record = UserRecord {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                display_name: row.get("display_name"),
                password_hash: row.get("password_hash"),
                permissions: row.get("permissions"),
                is_admin: row.get("is_admin"),
                created_at: row.get("created_at"),
            };

            let user_data = record.to_user_data()?;
            Ok(Some(user_data))
        } else {
            Ok(None)
        }
    }

    /// Check if username exists
    pub async fn username_exists(&self, username: &str) -> Result<bool, AuthError> {
        let query = "SELECT COUNT(*) as count FROM users WHERE username = ?";
        let row = sqlx::query(query)
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to check username existence: {}", e);
                AuthError::TokenCreation
            })?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }

    /// Check if email exists
    pub async fn email_exists(&self, email: &str) -> Result<bool, AuthError> {
        let query = "SELECT COUNT(*) as count FROM users WHERE email = ?";
        let row = sqlx::query(query)
            .bind(email)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to check email existence: {}", e);
                AuthError::TokenCreation
            })?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }

    /// Register new user
    pub async fn register_user(&self, user: UserData) -> Result<UserData, AuthError> {
        // Check if username or email already exists
        if self.username_exists(&user.username).await? {
            return Err(AuthError::InvalidCredentials);
        }

        if self.email_exists(&user.email).await? {
            return Err(AuthError::InvalidCredentials);
        }

        // Insert user
        self.insert_user(&user).await?;
        Ok(user)
    }

    /// Update user permissions
    pub async fn update_user_permissions(
        &self,
        user_id: &str,
        permissions: Vec<Permission>,
    ) -> Result<(), AuthError> {
        let permissions_json = serde_json::to_string(
            &permissions
                .iter()
                .map(|p| format!("{:?}", p))
                .collect::<Vec<_>>(),
        )
        .map_err(|_| AuthError::InvalidPermissions)?;

        let query = "UPDATE users SET permissions = ? WHERE id = ?";
        sqlx::query(query)
            .bind(&permissions_json)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to update user permissions: {}", e);
                AuthError::TokenCreation
            })?;

        // Clear cache for this user
        let mut cache = self.cache.write().await;
        cache.retain(|_, user| user.id != user_id);

        debug!("Updated permissions for user: {}", user_id);
        Ok(())
    }

    /// Get user statistics
    pub async fn get_user_stats(&self) -> Result<UserStats, AuthError> {
        let query = r#"
            SELECT 
                COUNT(*) as total_users,
                COUNT(CASE WHEN is_admin = 1 THEN 1 END) as admin_users,
                COUNT(CASE WHEN created_at > datetime('now', '-7 days') THEN 1 END) as recent_users
            FROM users
        "#;

        let row = sqlx::query(query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to get user statistics: {}", e);
                AuthError::TokenCreation
            })?;

        Ok(UserStats {
            total_users: row.get::<i64, _>("total_users") as u64,
            admin_users: row.get::<i64, _>("admin_users") as u64,
            recent_users: row.get::<i64, _>("recent_users") as u64,
        })
    }
}

/// User statistics
#[derive(Debug, Serialize)]
pub struct UserStats {
    pub total_users: u64,
    pub admin_users: u64,
    pub recent_users: u64,
}
