//! Authentication and Authorization Module
//!
//! This module provides flexible authentication and authorization capabilities
//! that can be configured for different deployment scenarios:
//! - Open mode: All features available to everyone
//! - Restricted mode: Requires authentication and permission checks
//! - Local mode: Bypasses all authentication (for CLI usage)

pub mod context;
pub mod identity;
pub mod permissions;

pub use context::PermissionContext;
pub use identity::{UserIdentity, UserType};
pub use permissions::{Permission, PermissionManager, PermissionMode, ResourceLimits};
