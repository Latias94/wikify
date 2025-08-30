//! Permission System
//!
//! Flexible permission system that supports different deployment modes
//! and can be easily extended for future requirements.

use super::identity::IdentityProvider;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Specific permissions that can be granted to users
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    /// Query repositories using RAG
    Query,
    /// Generate Wiki documentation
    GenerateWiki,
    /// Perform deep research analysis
    DeepResearch,
    /// Export data and results
    Export,
    /// Manage sessions (view, delete, etc.)
    ManageSession,
    /// Administrative functions
    Admin,
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Permission::Query => write!(f, "query"),
            Permission::GenerateWiki => write!(f, "generate_wiki"),
            Permission::DeepResearch => write!(f, "deep_research"),
            Permission::Export => write!(f, "export"),
            Permission::ManageSession => write!(f, "manage_session"),
            Permission::Admin => write!(f, "admin"),
        }
    }
}

impl std::str::FromStr for Permission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "query" => Ok(Permission::Query),
            "generate_wiki" => Ok(Permission::GenerateWiki),
            "deep_research" => Ok(Permission::DeepResearch),
            "export" => Ok(Permission::Export),
            "manage_session" => Ok(Permission::ManageSession),
            "admin" => Ok(Permission::Admin),
            _ => Err(format!("Unknown permission: {}", s)),
        }
    }
}

/// Permission mode determines how the system handles authorization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionMode {
    /// Open mode: All features available to everyone
    Open,
    /// Restricted mode: Requires authentication and permission checks
    Restricted,
    /// Local mode: Bypasses all permission checks (for CLI)
    Local,
}

impl std::fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionMode::Open => write!(f, "open"),
            PermissionMode::Restricted => write!(f, "restricted"),
            PermissionMode::Local => write!(f, "local"),
        }
    }
}

impl std::str::FromStr for PermissionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(PermissionMode::Open),
            "restricted" => Ok(PermissionMode::Restricted),
            "local" => Ok(PermissionMode::Local),
            _ => Err(format!("Unknown permission mode: {}", s)),
        }
    }
}

/// Resource limits for users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum queries per hour
    pub queries_per_hour: u32,
    /// Maximum concurrent sessions
    pub concurrent_sessions: u32,
    /// Maximum session duration in hours
    pub max_session_duration_hours: u32,
    /// Storage limit in MB
    pub storage_limit_mb: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            queries_per_hour: 100,
            concurrent_sessions: 3,
            max_session_duration_hours: 8,
            storage_limit_mb: 100,
        }
    }
}

impl ResourceLimits {
    /// Create unlimited resource limits
    pub fn unlimited() -> Self {
        Self {
            queries_per_hour: u32::MAX,
            concurrent_sessions: u32::MAX,
            max_session_duration_hours: u32::MAX,
            storage_limit_mb: u32::MAX,
        }
    }

    /// Create restrictive limits for anonymous users
    pub fn anonymous() -> Self {
        Self {
            queries_per_hour: 20,
            concurrent_sessions: 1,
            max_session_duration_hours: 1,
            storage_limit_mb: 10,
        }
    }
}

/// Permission configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// Permission mode
    pub mode: PermissionMode,
    /// Allow anonymous access
    pub allow_anonymous: bool,
    /// Default permissions for anonymous users
    pub anonymous_permissions: HashSet<Permission>,
    /// Default permissions for registered users
    pub registered_permissions: HashSet<Permission>,
    /// Default resource limits for anonymous users
    pub anonymous_limits: ResourceLimits,
    /// Default resource limits for registered users
    pub registered_limits: ResourceLimits,
    /// Custom permission overrides
    pub custom_permissions: HashMap<String, HashSet<Permission>>,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            mode: PermissionMode::Open,
            allow_anonymous: true,
            anonymous_permissions: [Permission::Query].into_iter().collect(),
            registered_permissions: [Permission::Query, Permission::GenerateWiki]
                .into_iter()
                .collect(),
            anonymous_limits: ResourceLimits::anonymous(),
            registered_limits: ResourceLimits::default(),
            custom_permissions: HashMap::new(),
        }
    }
}

impl PermissionConfig {
    /// Create open mode configuration
    pub fn open() -> Self {
        Self {
            mode: PermissionMode::Open,
            allow_anonymous: true,
            anonymous_permissions: [
                Permission::Query,
                Permission::GenerateWiki,
                Permission::DeepResearch,
                Permission::Export,
            ]
            .into_iter()
            .collect(),
            registered_permissions: [
                Permission::Query,
                Permission::GenerateWiki,
                Permission::DeepResearch,
                Permission::Export,
                Permission::ManageSession,
            ]
            .into_iter()
            .collect(),
            anonymous_limits: ResourceLimits::default(),
            registered_limits: ResourceLimits::unlimited(),
            custom_permissions: HashMap::new(),
        }
    }

    /// Create restricted mode configuration
    pub fn restricted() -> Self {
        Self {
            mode: PermissionMode::Restricted,
            allow_anonymous: false,
            anonymous_permissions: HashSet::new(),
            registered_permissions: [Permission::Query, Permission::GenerateWiki]
                .into_iter()
                .collect(),
            anonymous_limits: ResourceLimits::anonymous(),
            registered_limits: ResourceLimits::default(),
            custom_permissions: HashMap::new(),
        }
    }

    /// Create local mode configuration (no restrictions)
    pub fn local() -> Self {
        Self {
            mode: PermissionMode::Local,
            allow_anonymous: true,
            anonymous_permissions: [
                Permission::Query,
                Permission::GenerateWiki,
                Permission::DeepResearch,
                Permission::Export,
                Permission::ManageSession,
                Permission::Admin,
            ]
            .into_iter()
            .collect(),
            registered_permissions: [
                Permission::Query,
                Permission::GenerateWiki,
                Permission::DeepResearch,
                Permission::Export,
                Permission::ManageSession,
                Permission::Admin,
            ]
            .into_iter()
            .collect(),
            anonymous_limits: ResourceLimits::unlimited(),
            registered_limits: ResourceLimits::unlimited(),
            custom_permissions: HashMap::new(),
        }
    }
}

/// Permission manager handles authorization logic
pub struct PermissionManager {
    config: PermissionConfig,
    identity_provider: Option<Arc<dyn IdentityProvider>>,
    usage_tracker: Arc<RwLock<HashMap<String, UsageStats>>>,
}

/// Usage statistics for rate limiting
#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    pub queries_this_hour: u32,
    pub current_sessions: u32,
    pub last_reset: chrono::DateTime<chrono::Utc>,
}

impl PermissionManager {
    /// Create a new permission manager
    pub fn new(config: PermissionConfig) -> Self {
        Self {
            config,
            identity_provider: None,
            usage_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set identity provider
    pub fn with_identity_provider(mut self, provider: Arc<dyn IdentityProvider>) -> Self {
        self.identity_provider = Some(provider);
        self
    }

    /// Check if a user has permission to perform an operation
    pub async fn check_permission(
        &self,
        context: &super::PermissionContext,
        permission: &Permission,
    ) -> Result<bool, String> {
        match self.config.mode {
            PermissionMode::Local => Ok(true), // Local mode bypasses all checks
            PermissionMode::Open => {
                // Open mode allows all operations for all users
                Ok(true)
            }
            PermissionMode::Restricted => {
                // Restricted mode requires proper authentication and authorization
                match &context.identity {
                    Some(identity) => Ok(identity.has_permission(permission)),
                    None => {
                        if self.config.allow_anonymous {
                            Ok(self.config.anonymous_permissions.contains(permission))
                        } else {
                            Ok(false)
                        }
                    }
                }
            }
        }
    }

    /// Check resource limits for a user
    pub async fn check_resource_limits(
        &self,
        context: &super::PermissionContext,
        resource_type: ResourceType,
    ) -> Result<bool, String> {
        if self.config.mode == PermissionMode::Local {
            return Ok(true); // Local mode bypasses all limits
        }

        let limits = match &context.identity {
            Some(identity) => identity.effective_limits(),
            None => self.config.anonymous_limits.clone(),
        };

        let user_id = context
            .identity
            .as_ref()
            .map(|i| i.user_id.clone())
            .unwrap_or_else(|| "anonymous".to_string());

        let mut usage_tracker = self.usage_tracker.write().await;
        let stats = usage_tracker.entry(user_id).or_default();

        // Reset hourly counters if needed
        let now = chrono::Utc::now();
        if now.signed_duration_since(stats.last_reset).num_hours() >= 1 {
            stats.queries_this_hour = 0;
            stats.last_reset = now;
        }

        match resource_type {
            ResourceType::Query => Ok(stats.queries_this_hour < limits.queries_per_hour),
            ResourceType::Session => Ok(stats.current_sessions < limits.concurrent_sessions),
        }
    }

    /// Record resource usage
    pub async fn record_usage(
        &self,
        context: &super::PermissionContext,
        resource_type: ResourceType,
        delta: i32,
    ) {
        if self.config.mode == PermissionMode::Local {
            return; // Local mode doesn't track usage
        }

        let user_id = context
            .identity
            .as_ref()
            .map(|i| i.user_id.clone())
            .unwrap_or_else(|| "anonymous".to_string());

        let mut usage_tracker = self.usage_tracker.write().await;
        let stats = usage_tracker.entry(user_id).or_default();

        match resource_type {
            ResourceType::Query => {
                stats.queries_this_hour = stats.queries_this_hour.saturating_add_signed(delta);
            }
            ResourceType::Session => {
                stats.current_sessions = stats.current_sessions.saturating_add_signed(delta);
            }
        }
    }

    /// Get current usage stats for a user
    pub async fn get_usage_stats(&self, user_id: &str) -> Option<UsageStats> {
        let usage_tracker = self.usage_tracker.read().await;
        usage_tracker.get(user_id).cloned()
    }
}

/// Resource types for tracking usage
#[derive(Debug, Clone, Copy)]
pub enum ResourceType {
    Query,
    Session,
}
