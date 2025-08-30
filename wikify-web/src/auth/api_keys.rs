//! API Key management system for Wikify
//!
//! This module provides a complete API Key management system including:
//! - API Key generation and validation
//! - Storage and retrieval
//! - Permission management
//! - Expiration handling

use crate::auth::{users::UserData, Permission};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// API Key data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique API key ID
    pub id: String,
    /// The actual API key (hashed for security)
    pub key_hash: String,
    /// User ID that owns this API key
    pub user_id: String,
    /// Human-readable name for the API key
    pub name: String,
    /// Permissions granted to this API key
    pub permissions: Vec<Permission>,
    /// When the API key was created
    pub created_at: DateTime<Utc>,
    /// When the API key expires (None = never expires)
    pub expires_at: Option<DateTime<Utc>>,
    /// When the API key was last used
    pub last_used_at: Option<DateTime<Utc>>,
    /// Whether the API key is active
    pub is_active: bool,
}

/// API Key creation request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    /// Human-readable name for the API key
    pub name: String,
    /// Permissions to grant to this API key
    pub permissions: Vec<Permission>,
    /// Optional expiration time (in days from now)
    pub expires_in_days: Option<u32>,
}

/// API Key response (includes the raw key only on creation)
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    /// API key ID
    pub id: String,
    /// The raw API key (only returned on creation)
    pub key: Option<String>,
    /// Human-readable name
    pub name: String,
    /// Permissions
    pub permissions: Vec<Permission>,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Expiration time
    pub expires_at: Option<DateTime<Utc>>,
    /// Last used time
    pub last_used_at: Option<DateTime<Utc>>,
    /// Whether active
    pub is_active: bool,
}

/// API Key storage trait
#[async_trait::async_trait]
pub trait ApiKeyStorage: Send + Sync {
    /// Store an API key
    async fn store_api_key(&self, api_key: &ApiKey) -> Result<(), ApiKeyError>;

    /// Get API key by hash
    async fn get_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKey>, ApiKeyError>;

    /// Get API key by ID
    async fn get_api_key_by_id(&self, id: &str) -> Result<Option<ApiKey>, ApiKeyError>;

    /// List API keys for a user
    async fn list_user_api_keys(&self, user_id: &str) -> Result<Vec<ApiKey>, ApiKeyError>;

    /// Update API key
    async fn update_api_key(&self, api_key: &ApiKey) -> Result<(), ApiKeyError>;

    /// Delete API key
    async fn delete_api_key(&self, id: &str) -> Result<(), ApiKeyError>;
}

/// API Key service errors
#[derive(Debug, thiserror::Error)]
pub enum ApiKeyError {
    #[error("API key not found")]
    NotFound,
    #[error("API key expired")]
    Expired,
    #[error("API key inactive")]
    Inactive,
    #[error("Invalid API key format")]
    InvalidFormat,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Storage error: {0}")]
    Storage(String),
}

/// In-memory API Key storage implementation
#[derive(Debug, Default)]
pub struct MemoryApiKeyStorage {
    keys: Arc<RwLock<HashMap<String, ApiKey>>>,
    key_hash_index: Arc<RwLock<HashMap<String, String>>>, // hash -> id
}

impl MemoryApiKeyStorage {
    /// Create new memory storage
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl ApiKeyStorage for MemoryApiKeyStorage {
    async fn store_api_key(&self, api_key: &ApiKey) -> Result<(), ApiKeyError> {
        let mut keys = self.keys.write().await;
        let mut hash_index = self.key_hash_index.write().await;

        keys.insert(api_key.id.clone(), api_key.clone());
        hash_index.insert(api_key.key_hash.clone(), api_key.id.clone());

        debug!(
            "Stored API key: {} for user: {}",
            api_key.id, api_key.user_id
        );
        Ok(())
    }

    async fn get_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKey>, ApiKeyError> {
        let hash_index = self.key_hash_index.read().await;
        let keys = self.keys.read().await;

        if let Some(id) = hash_index.get(key_hash) {
            Ok(keys.get(id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn get_api_key_by_id(&self, id: &str) -> Result<Option<ApiKey>, ApiKeyError> {
        let keys = self.keys.read().await;
        Ok(keys.get(id).cloned())
    }

    async fn list_user_api_keys(&self, user_id: &str) -> Result<Vec<ApiKey>, ApiKeyError> {
        let keys = self.keys.read().await;
        let user_keys: Vec<ApiKey> = keys
            .values()
            .filter(|key| key.user_id == user_id)
            .cloned()
            .collect();
        Ok(user_keys)
    }

    async fn update_api_key(&self, api_key: &ApiKey) -> Result<(), ApiKeyError> {
        let mut keys = self.keys.write().await;
        if keys.contains_key(&api_key.id) {
            keys.insert(api_key.id.clone(), api_key.clone());
            debug!("Updated API key: {}", api_key.id);
            Ok(())
        } else {
            Err(ApiKeyError::NotFound)
        }
    }

    async fn delete_api_key(&self, id: &str) -> Result<(), ApiKeyError> {
        let mut keys = self.keys.write().await;
        let mut hash_index = self.key_hash_index.write().await;

        if let Some(api_key) = keys.remove(id) {
            hash_index.remove(&api_key.key_hash);
            info!("Deleted API key: {} for user: {}", id, api_key.user_id);
            Ok(())
        } else {
            Err(ApiKeyError::NotFound)
        }
    }
}

/// API Key service for managing API keys
#[derive(Clone)]
pub struct ApiKeyService {
    storage: Arc<dyn ApiKeyStorage>,
}

impl ApiKeyService {
    /// Create new API key service
    pub fn new(storage: Arc<dyn ApiKeyStorage>) -> Self {
        Self { storage }
    }

    /// Create new API key service with memory storage
    pub fn memory() -> Self {
        Self::new(Arc::new(MemoryApiKeyStorage::new()))
    }

    /// Get storage reference for direct access
    pub fn storage(&self) -> &Arc<dyn ApiKeyStorage> {
        &self.storage
    }

    /// Generate a new API key
    pub async fn create_api_key(
        &self,
        user_id: &str,
        request: CreateApiKeyRequest,
    ) -> Result<ApiKeyResponse, ApiKeyError> {
        // Generate raw API key
        let raw_key = self.generate_raw_key();
        let key_hash = self.hash_key(&raw_key);

        // Calculate expiration
        let expires_at = request
            .expires_in_days
            .map(|days| Utc::now() + chrono::Duration::days(days as i64));

        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            key_hash,
            user_id: user_id.to_string(),
            name: request.name.clone(),
            permissions: request.permissions.clone(),
            created_at: Utc::now(),
            expires_at,
            last_used_at: None,
            is_active: true,
        };

        // Store the API key
        self.storage.store_api_key(&api_key).await?;

        info!("Created API key '{}' for user: {}", request.name, user_id);

        Ok(ApiKeyResponse {
            id: api_key.id,
            key: Some(raw_key), // Only return raw key on creation
            name: api_key.name,
            permissions: api_key.permissions,
            created_at: api_key.created_at,
            expires_at: api_key.expires_at,
            last_used_at: api_key.last_used_at,
            is_active: api_key.is_active,
        })
    }

    /// Validate and authenticate an API key
    pub async fn authenticate_api_key(
        &self,
        raw_key: &str,
    ) -> Result<Option<UserData>, ApiKeyError> {
        let key_hash = self.hash_key(raw_key);

        match self.storage.get_api_key_by_hash(&key_hash).await? {
            Some(mut api_key) => {
                // Check if key is active
                if !api_key.is_active {
                    return Err(ApiKeyError::Inactive);
                }

                // Check if key is expired
                if let Some(expires_at) = api_key.expires_at {
                    if Utc::now() > expires_at {
                        return Err(ApiKeyError::Expired);
                    }
                }

                // Update last used time
                api_key.last_used_at = Some(Utc::now());
                let _ = self.storage.update_api_key(&api_key).await;

                // Create user data from API key
                let is_admin = api_key.permissions.contains(&Permission::Admin);
                let user_data = UserData {
                    id: format!("apikey_{}", api_key.id),
                    username: format!("apikey_{}", api_key.name),
                    email: format!("apikey_{}@wikify.local", api_key.id),
                    display_name: Some(format!("API Key: {}", api_key.name)),
                    password_hash: String::new(), // API keys don't have passwords
                    permissions: api_key.permissions,
                    is_admin,
                    created_at: api_key.created_at,
                };

                debug!(
                    "Authenticated API key: {} for user: {}",
                    api_key.name, api_key.user_id
                );
                Ok(Some(user_data))
            }
            None => {
                debug!("API key not found");
                Ok(None)
            }
        }
    }

    /// Generate a raw API key
    fn generate_raw_key(&self) -> String {
        format!("wk_{}", Uuid::new_v4().simple())
    }

    /// Hash an API key for storage
    fn hash_key(&self, key: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
