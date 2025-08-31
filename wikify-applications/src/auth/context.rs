//! Permission Context
//!
//! Provides a unified permission context that encapsulates user identity,
//! permissions, and resource limits for authorization decisions.

use super::{Permission, PermissionMode, ResourceLimits, UserIdentity};
// use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Permission context encapsulates all authorization information for a request
#[derive(Debug, Clone)]
pub struct PermissionContext {
    /// User identity (None for anonymous users)
    pub identity: Option<UserIdentity>,
    /// Permission mode
    pub mode: PermissionMode,
    /// Effective permissions for this context
    pub permissions: HashSet<Permission>,
    /// Effective resource limits for this context
    pub limits: ResourceLimits,
    /// Additional context metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl PermissionContext {
    /// Create a new permission context
    pub fn new(
        identity: Option<UserIdentity>,
        mode: PermissionMode,
        permissions: HashSet<Permission>,
        limits: ResourceLimits,
    ) -> Self {
        Self {
            identity,
            mode,
            permissions,
            limits,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create an open permission context (all permissions granted)
    pub fn open() -> Self {
        Self {
            identity: None,
            mode: PermissionMode::Open,
            permissions: [
                Permission::Query,
                Permission::GenerateWiki,
                Permission::DeepResearch,
                Permission::Export,
                Permission::ManageRepository,
            ]
            .into_iter()
            .collect(),
            limits: ResourceLimits::unlimited(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a local permission context (for CLI usage)
    pub fn local() -> Self {
        Self {
            identity: Some(UserIdentity::local()),
            mode: PermissionMode::Local,
            permissions: [
                Permission::Query,
                Permission::GenerateWiki,
                Permission::DeepResearch,
                Permission::Export,
                Permission::ManageRepository,
                Permission::Admin,
            ]
            .into_iter()
            .collect(),
            limits: ResourceLimits::unlimited(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create an anonymous permission context
    pub fn anonymous(permissions: HashSet<Permission>, limits: ResourceLimits) -> Self {
        Self {
            identity: Some(UserIdentity::anonymous()),
            mode: PermissionMode::Restricted,
            permissions,
            limits,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a user permission context
    pub fn user(identity: UserIdentity) -> Self {
        let permissions = identity.effective_permissions();
        let limits = identity.effective_limits();

        Self {
            identity: Some(identity),
            mode: PermissionMode::Restricted,
            permissions,
            limits,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Check if this context has a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        match self.mode {
            PermissionMode::Local => true, // Local mode has all permissions
            PermissionMode::Open => true,  // Open mode has all permissions
            PermissionMode::Restricted => self.permissions.contains(permission),
        }
    }

    /// Check if this context can perform multiple permissions
    pub fn has_all_permissions(&self, permissions: &[Permission]) -> bool {
        permissions.iter().all(|p| self.has_permission(p))
    }

    /// Check if this context has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[Permission]) -> bool {
        permissions.iter().any(|p| self.has_permission(p))
    }

    /// Get user ID if available
    pub fn user_id(&self) -> Option<&str> {
        self.identity.as_ref().map(|i| i.user_id.as_str())
    }

    /// Get user display name if available
    pub fn display_name(&self) -> Option<&str> {
        self.identity
            .as_ref()
            .and_then(|i| i.display_name.as_deref())
    }

    /// Check if this is an anonymous context
    pub fn is_anonymous(&self) -> bool {
        match &self.identity {
            Some(identity) => identity.user_type == super::UserType::Anonymous,
            None => true,
        }
    }

    /// Check if this is a local context (CLI usage)
    pub fn is_local(&self) -> bool {
        self.mode == PermissionMode::Local
    }

    /// Check if this is an admin context
    pub fn is_admin(&self) -> bool {
        match &self.identity {
            Some(identity) => identity.user_type == super::UserType::Admin,
            None => false,
        }
    }

    /// Add metadata to the context
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Create a summary string for logging
    pub fn summary(&self) -> String {
        let user_info = match &self.identity {
            Some(identity) => format!("{}({})", identity.user_id, identity.user_type),
            None => "anonymous".to_string(),
        };

        let permissions_count = self.permissions.len();

        format!(
            "PermissionContext[user={}, mode={}, permissions={}, limits={}q/h]",
            user_info, self.mode, permissions_count, self.limits.queries_per_hour
        )
    }
}

impl Default for PermissionContext {
    fn default() -> Self {
        Self::open()
    }
}

/// Builder for creating permission contexts
pub struct PermissionContextBuilder {
    identity: Option<UserIdentity>,
    mode: Option<PermissionMode>,
    permissions: Option<HashSet<Permission>>,
    limits: Option<ResourceLimits>,
    metadata: std::collections::HashMap<String, String>,
}

impl PermissionContextBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            identity: None,
            mode: None,
            permissions: None,
            limits: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set user identity
    pub fn with_identity(mut self, identity: UserIdentity) -> Self {
        self.identity = Some(identity);
        self
    }

    /// Set permission mode
    pub fn with_mode(mut self, mode: PermissionMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Set permissions
    pub fn with_permissions(mut self, permissions: HashSet<Permission>) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Add a single permission
    pub fn add_permission(mut self, permission: Permission) -> Self {
        let mut permissions = self.permissions.unwrap_or_default();
        permissions.insert(permission);
        self.permissions = Some(permissions);
        self
    }

    /// Set resource limits
    pub fn with_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = Some(limits);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Build the permission context
    pub fn build(self) -> PermissionContext {
        let mode = self.mode.unwrap_or(PermissionMode::Open);

        let (permissions, limits) = match &self.identity {
            Some(identity) => (
                self.permissions
                    .unwrap_or_else(|| identity.effective_permissions()),
                self.limits.unwrap_or_else(|| identity.effective_limits()),
            ),
            None => (
                self.permissions
                    .unwrap_or_else(|| [Permission::Query].into_iter().collect()),
                self.limits.unwrap_or_default(),
            ),
        };

        PermissionContext {
            identity: self.identity,
            mode,
            permissions,
            limits,
            metadata: self.metadata,
        }
    }
}

impl Default for PermissionContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for creating common permission contexts
impl PermissionContext {
    /// Create context from HTTP headers (for web usage)
    pub fn from_headers(headers: &std::collections::HashMap<String, String>) -> Self {
        // Check for user authentication headers
        if let Some(user_id) = headers.get("x-user-id") {
            let _user_type = headers
                .get("x-user-type")
                .and_then(|t| t.parse().ok())
                .unwrap_or(super::UserType::Registered);

            let display_name = headers.get("x-user-name").cloned();
            let email = headers.get("x-user-email").cloned();

            let identity = UserIdentity::registered(user_id.clone(), display_name, email);
            Self::user(identity)
        } else if headers.get("x-anonymous").is_some() {
            // Anonymous user with limited permissions
            Self::anonymous(
                [Permission::Query].into_iter().collect(),
                ResourceLimits::anonymous(),
            )
        } else {
            // Default to open context
            Self::open()
        }
    }

    /// Create context from environment variables (for CLI usage)
    pub fn from_env() -> Self {
        if std::env::var("WIKIFY_LOCAL").is_ok() {
            Self::local()
        } else if let Ok(user_id) = std::env::var("WIKIFY_USER_ID") {
            let display_name = std::env::var("WIKIFY_USER_NAME").ok();
            let email = std::env::var("WIKIFY_USER_EMAIL").ok();

            let identity = UserIdentity::registered(user_id, display_name, email);
            Self::user(identity)
        } else {
            Self::local() // Default to local for CLI
        }
    }
}
