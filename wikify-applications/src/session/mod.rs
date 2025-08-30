//! Session Management Module
//!
//! Provides comprehensive session management with permission-aware operations,
//! supporting multiple deployment scenarios and user types.

pub mod manager;
pub mod storage;
pub mod types;

pub use manager::SessionManager;
pub use storage::SessionStorage;
pub use types::*;
