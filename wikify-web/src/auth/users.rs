//! User management and authentication

use super::{
    database::DatabaseUserStore,
    jwt::{AuthError, JwtService, TokenPair},
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;
use wikify_applications::Permission;

/// User registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

/// User login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Token refresh request
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// User registration/login response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserInfo,
    #[serde(flatten)]
    pub tokens: TokenPair,
}

/// Public user information
#[derive(Debug, Serialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub permissions: Vec<String>,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Internal user data with password hash
#[derive(Debug, Clone)]
pub struct UserData {
    pub id: String,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub password_hash: String,
    pub permissions: Vec<Permission>,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl UserData {
    /// Create new user with hashed password
    pub fn new(
        username: String,
        email: String,
        password: &str,
        display_name: Option<String>,
        permissions: Vec<Permission>,
        is_admin: bool,
    ) -> Result<Self, AuthError> {
        let password_hash = hash_password(password)?;

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            username,
            email,
            display_name,
            password_hash,
            permissions,
            is_admin,
            created_at: chrono::Utc::now(),
        })
    }

    /// Verify password
    pub fn verify_password(&self, password: &str) -> bool {
        verify_password(password, &self.password_hash).unwrap_or(false)
    }

    /// Convert to public user info
    pub fn to_user_info(&self) -> UserInfo {
        UserInfo {
            id: self.id.clone(),
            username: self.username.clone(),
            email: self.email.clone(),
            display_name: self.display_name.clone(),
            permissions: self
                .permissions
                .iter()
                .map(|p| format!("{:?}", p))
                .collect(),
            is_admin: self.is_admin,
            created_at: self.created_at,
        }
    }
}

/// User store abstraction supporting both in-memory and database storage
#[derive(Debug, Clone)]
pub enum UserStore {
    /// In-memory storage (for development and testing)
    Memory {
        users: Arc<RwLock<HashMap<String, UserData>>>,
        users_by_email: Arc<RwLock<HashMap<String, String>>>, // email -> user_id
    },
    /// Database storage (for production)
    Database(DatabaseUserStore),
}

impl Default for UserStore {
    fn default() -> Self {
        Self::memory()
    }
}

impl UserStore {
    /// Create in-memory user store
    pub fn memory() -> Self {
        let store = Self::Memory {
            users: Arc::new(RwLock::new(HashMap::new())),
            users_by_email: Arc::new(RwLock::new(HashMap::new())),
        };

        // Create default admin user
        if let Err(e) = store.create_default_admin_sync() {
            warn!("Failed to create default admin user: {}", e);
        }

        store
    }

    /// Create database user store
    pub async fn database(database_store: DatabaseUserStore) -> Self {
        Self::Database(database_store)
    }

    /// Create default admin user (synchronous for memory store)
    fn create_default_admin_sync(&self) -> Result<(), AuthError> {
        match self {
            Self::Memory {
                users,
                users_by_email,
            } => {
                let admin_user = UserData::new(
                    "admin".to_string(),
                    "admin@wikify.local".to_string(),
                    "admin123", // Default password - should be changed in production
                    Some("Administrator".to_string()),
                    vec![Permission::Admin],
                    true,
                )?;

                info!("Creating default admin user: {}", admin_user.username);

                let mut users = users.write().unwrap();
                let mut users_by_email = users_by_email.write().unwrap();

                users.insert(admin_user.username.clone(), admin_user.clone());
                users_by_email.insert(admin_user.email.clone(), admin_user.id.clone());

                Ok(())
            }
            Self::Database(_) => {
                // Database store handles admin user creation internally
                Ok(())
            }
        }
    }

    /// Register new user
    pub async fn register_user(&self, request: RegisterRequest) -> Result<UserData, AuthError> {
        debug!("Starting user registration for: {}", request.username);

        // Validate input
        if request.username.is_empty() || request.email.is_empty() || request.password.is_empty() {
            debug!("Registration failed: missing credentials");
            return Err(AuthError::MissingCredentials);
        }

        if request.password.len() < 6 {
            debug!("Registration failed: password too short");
            return Err(AuthError::InvalidCredentials);
        }

        match self {
            Self::Memory {
                users,
                users_by_email,
            } => {
                let users_read = users.read().unwrap();
                let users_by_email_read = users_by_email.read().unwrap();

                // Check if username already exists
                if users_read.contains_key(&request.username) {
                    debug!(
                        "Registration failed: username '{}' already exists",
                        request.username
                    );
                    return Err(AuthError::InvalidCredentials);
                }

                // Check if email already exists
                if users_by_email_read.contains_key(&request.email) {
                    debug!(
                        "Registration failed: email '{}' already exists",
                        request.email
                    );
                    return Err(AuthError::InvalidCredentials);
                }

                drop(users_read);
                drop(users_by_email_read);

                // Create new user with default permissions
                debug!("Creating new user data for: {}", request.username);
                let user_data = UserData::new(
                    request.username,
                    request.email,
                    &request.password,
                    request.display_name,
                    vec![Permission::Query, Permission::GenerateWiki], // Default permissions
                    false,                                             // Not admin by default
                )?;
                debug!("User data created successfully for: {}", user_data.username);

                // Store user
                let mut users_write = users.write().unwrap();
                let mut users_by_email_write = users_by_email.write().unwrap();

                users_write.insert(user_data.username.clone(), user_data.clone());
                users_by_email_write.insert(user_data.email.clone(), user_data.id.clone());

                info!("Registered new user: {}", user_data.username);
                Ok(user_data)
            }
            Self::Database(db_store) => {
                // Create new user with default permissions
                let user_data = UserData::new(
                    request.username,
                    request.email,
                    &request.password,
                    request.display_name,
                    vec![Permission::Query, Permission::GenerateWiki], // Default permissions
                    false,                                             // Not admin by default
                )?;

                db_store.register_user(user_data).await
            }
        }
    }

    /// Authenticate user
    pub async fn authenticate_user(&self, request: LoginRequest) -> Result<UserData, AuthError> {
        match self {
            Self::Memory { users, .. } => {
                let users = users.read().unwrap();

                let user = users
                    .get(&request.username)
                    .ok_or(AuthError::InvalidCredentials)?;

                if !user.verify_password(&request.password) {
                    warn!("Invalid password for user: {}", request.username);
                    return Err(AuthError::InvalidCredentials);
                }

                debug!("User authenticated: {}", request.username);
                Ok(user.clone())
            }
            Self::Database(db_store) => {
                let user = db_store
                    .get_user_by_username(&request.username)
                    .await?
                    .ok_or(AuthError::InvalidCredentials)?;

                if !user.verify_password(&request.password) {
                    warn!("Invalid password for user: {}", request.username);
                    return Err(AuthError::InvalidCredentials);
                }

                debug!("User authenticated: {}", request.username);
                Ok(user)
            }
        }
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> Option<UserData> {
        match self {
            Self::Memory { users, .. } => {
                let users = users.read().unwrap();
                users.values().find(|u| u.id == user_id).cloned()
            }
            Self::Database(db_store) => db_store.get_user_by_id(user_id).await.unwrap_or(None),
        }
    }

    /// Get user by username
    pub async fn get_user_by_username(&self, username: &str) -> Option<UserData> {
        match self {
            Self::Memory { users, .. } => {
                let users = users.read().unwrap();
                users.get(username).cloned()
            }
            Self::Database(db_store) => db_store
                .get_user_by_username(username)
                .await
                .unwrap_or(None),
        }
    }
}

/// User service for authentication operations
#[derive(Debug, Clone)]
pub struct UserService {
    store: UserStore,
}

impl Default for UserService {
    fn default() -> Self {
        Self {
            store: UserStore::default(),
        }
    }
}

impl UserService {
    /// Create new user service with custom store
    pub fn new(store: UserStore) -> Self {
        Self { store }
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> Option<UserData> {
        self.store.get_user_by_id(user_id).await
    }

    /// Get user by username
    pub async fn get_user_by_username(&self, username: &str) -> Option<UserData> {
        self.store.get_user_by_username(username).await
    }

    /// Register new user
    pub async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, AuthError> {
        let user_data = self.store.register_user(request).await?;

        let tokens = JwtService::generate_token_pair(
            user_data.id.clone(),
            user_data.display_name.clone(),
            Some(user_data.email.clone()),
            user_data.permissions.clone(),
            user_data.is_admin,
        )?;

        Ok(AuthResponse {
            user: user_data.to_user_info(),
            tokens,
        })
    }

    /// Login user
    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, AuthError> {
        let user_data = self.store.authenticate_user(request).await?;

        let tokens = JwtService::generate_token_pair(
            user_data.id.clone(),
            user_data.display_name.clone(),
            Some(user_data.email.clone()),
            user_data.permissions.clone(),
            user_data.is_admin,
        )?;

        Ok(AuthResponse {
            user: user_data.to_user_info(),
            tokens,
        })
    }

    /// Refresh access token
    pub async fn refresh_token(&self, request: RefreshRequest) -> Result<TokenPair, AuthError> {
        let claims = JwtService::verify_token(&request.refresh_token)?;

        // Ensure it's a refresh token
        if claims.token_type != super::jwt::TokenType::Refresh {
            return Err(AuthError::InvalidTokenType);
        }

        // Get current user data
        let user_data = self
            .store
            .get_user_by_id(&claims.sub)
            .await
            .ok_or(AuthError::InvalidCredentials)?;

        // Generate new token pair
        JwtService::generate_token_pair(
            user_data.id,
            user_data.display_name,
            Some(user_data.email),
            user_data.permissions,
            user_data.is_admin,
        )
    }

    /// Get user store (for testing)
    pub fn store(&self) -> &UserStore {
        &self.store
    }
}

/// Hash password using Argon2
fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthError::TokenCreation)
}

/// Verify password against hash
fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| AuthError::InvalidToken)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
