//! HTTP request handlers for the Wikify web server
//!
//! This module contains all the HTTP request handlers organized by functionality.

pub mod chat;
pub mod config;
pub mod files;
pub mod health;
pub mod repository;
pub mod research;
pub mod types;
pub mod wiki;

// Re-export all handler functions to maintain API compatibility
pub use chat::*;
pub use config::*;
pub use files::*;
pub use health::*;
pub use repository::*;
pub use research::*;
pub use wiki::*;

// Re-export all types for convenience
pub use types::*;
