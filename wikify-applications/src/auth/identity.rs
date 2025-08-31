//! User Identity Management
//!
//! Defines user identity types and management for flexible authentication scenarios.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// User type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UserType {
    /// Anonymous user (temporary session)
    Anonymous,
    /// Registered user with persistent identity
    Registered,
    /// Premium user with enhanced features
    Premium,
    /// System administrator
    Admin,
}

impl UserType {
    /// Get default permissions for this user type
    pub fn default_permissions(&self) -> HashSet<super::Permission> {
        use super::Permission::*;

        match self {
            UserType::Anonymous => [Query].into_iter().collect(),
            UserType::Registered => [Query, GenerateWiki].into_iter().collect(),
            UserType::Premium => [Query, GenerateWiki, DeepResearch, Export]
                .into_iter()
                .collect(),
            UserType::Admin => [Query, GenerateWiki, DeepResearch, Export, ManageRepository]
                .into_iter()
                .collect(),
        }
    }

    /// Get default resource limits for this user type
    pub fn default_limits(&self) -> super::ResourceLimits {
        match self {
            UserType::Anonymous => super::ResourceLimits {
                queries_per_hour: 50,
                concurrent_sessions: 1,
                max_session_duration_hours: 1,
                storage_limit_mb: 10,
            },
            UserType::Registered => super::ResourceLimits {
                queries_per_hour: 200,
                concurrent_sessions: 3,
                max_session_duration_hours: 8,
                storage_limit_mb: 100,
            },
            UserType::Premium => super::ResourceLimits {
                queries_per_hour: 1000,
                concurrent_sessions: 10,
                max_session_duration_hours: 24,
                storage_limit_mb: 1000,
            },
            UserType::Admin => super::ResourceLimits {
                queries_per_hour: u32::MAX,
                concurrent_sessions: u32::MAX,
                max_session_duration_hours: u32::MAX,
                storage_limit_mb: u32::MAX,
            },
        }
    }
}

impl std::fmt::Display for UserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserType::Anonymous => write!(f, "anonymous"),
            UserType::Registered => write!(f, "registered"),
            UserType::Premium => write!(f, "premium"),
            UserType::Admin => write!(f, "admin"),
        }
    }
}

impl std::str::FromStr for UserType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anonymous" => Ok(UserType::Anonymous),
            "registered" => Ok(UserType::Registered),
            "premium" => Ok(UserType::Premium),
            "admin" => Ok(UserType::Admin),
            _ => Err(format!("Unknown user type: {}", s)),
        }
    }
}

/// User identity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    /// Unique user identifier
    pub user_id: String,
    /// User type classification
    pub user_type: UserType,
    /// Display name (optional)
    pub display_name: Option<String>,
    /// User email (optional)
    pub email: Option<String>,
    /// Additional user metadata
    pub metadata: HashMap<String, String>,
    /// Custom permissions (overrides defaults)
    pub custom_permissions: Option<HashSet<super::Permission>>,
    /// Custom resource limits (overrides defaults)
    pub custom_limits: Option<super::ResourceLimits>,
}

impl UserIdentity {
    /// Create a new anonymous user
    pub fn anonymous() -> Self {
        Self {
            user_id: format!("anon_{}", uuid::Uuid::new_v4()),
            user_type: UserType::Anonymous,
            display_name: Some("Anonymous User".to_string()),
            email: None,
            metadata: HashMap::new(),
            custom_permissions: None,
            custom_limits: None,
        }
    }

    /// Create a new registered user
    pub fn registered(
        user_id: String,
        display_name: Option<String>,
        email: Option<String>,
    ) -> Self {
        Self {
            user_id,
            user_type: UserType::Registered,
            display_name,
            email,
            metadata: HashMap::new(),
            custom_permissions: None,
            custom_limits: None,
        }
    }

    /// Create a local user (for CLI usage)
    pub fn local() -> Self {
        Self {
            user_id: "local".to_string(),
            user_type: UserType::Admin, // Local users have admin privileges
            display_name: Some("Local User".to_string()),
            email: None,
            metadata: HashMap::new(),
            custom_permissions: None,
            custom_limits: None,
        }
    }

    /// Get effective permissions for this user
    pub fn effective_permissions(&self) -> HashSet<super::Permission> {
        self.custom_permissions
            .clone()
            .unwrap_or_else(|| self.user_type.default_permissions())
    }

    /// Get effective resource limits for this user
    pub fn effective_limits(&self) -> super::ResourceLimits {
        self.custom_limits
            .clone()
            .unwrap_or_else(|| self.user_type.default_limits())
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &super::Permission) -> bool {
        self.effective_permissions().contains(permission)
    }

    /// Add custom permission
    pub fn add_permission(&mut self, permission: super::Permission) {
        let mut permissions = self.effective_permissions();
        permissions.insert(permission);
        self.custom_permissions = Some(permissions);
    }

    /// Remove custom permission
    pub fn remove_permission(&mut self, permission: &super::Permission) {
        let mut permissions = self.effective_permissions();
        permissions.remove(permission);
        self.custom_permissions = Some(permissions);
    }

    /// Set custom resource limits
    pub fn set_limits(&mut self, limits: super::ResourceLimits) {
        self.custom_limits = Some(limits);
    }

    /// Get user display string
    pub fn display_string(&self) -> String {
        match &self.display_name {
            Some(name) => format!("{} ({})", name, self.user_type),
            None => format!("{} ({})", self.user_id, self.user_type),
        }
    }
}

impl Default for UserIdentity {
    fn default() -> Self {
        Self::anonymous()
    }
}

/// User identity provider trait for extensibility
pub trait IdentityProvider: Send + Sync {
    /// Authenticate user with credentials
    fn authenticate(&self, credentials: &HashMap<String, String>) -> Result<UserIdentity, String>;

    /// Get user by ID
    fn get_user(&self, user_id: &str) -> Result<Option<UserIdentity>, String>;

    /// Validate user session/token
    fn validate_session(&self, token: &str) -> Result<Option<UserIdentity>, String>;
}

/// Simple in-memory identity provider for testing and simple deployments
pub struct SimpleIdentityProvider {
    users: HashMap<String, UserIdentity>,
}

impl SimpleIdentityProvider {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    pub fn add_user(&mut self, user: UserIdentity) {
        self.users.insert(user.user_id.clone(), user);
    }
}

impl Default for SimpleIdentityProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentityProvider for SimpleIdentityProvider {
    fn authenticate(&self, credentials: &HashMap<String, String>) -> Result<UserIdentity, String> {
        let user_id = credentials
            .get("user_id")
            .ok_or_else(|| "Missing user_id".to_string())?;

        self.users
            .get(user_id)
            .cloned()
            .ok_or_else(|| "User not found".to_string())
    }

    fn get_user(&self, user_id: &str) -> Result<Option<UserIdentity>, String> {
        Ok(self.users.get(user_id).cloned())
    }

    fn validate_session(&self, token: &str) -> Result<Option<UserIdentity>, String> {
        // Simple token validation - in practice, this would be more sophisticated
        if let Some(user_id) = token.strip_prefix("user_") {
            self.get_user(user_id)
        } else {
            Ok(None)
        }
    }
}
