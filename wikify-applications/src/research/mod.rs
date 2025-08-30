//! Deep research engine for complex multi-step investigations
//!
//! This module provides intelligent research capabilities that can:
//! - Break down complex questions into sub-questions
//! - Iteratively gather information from multiple sources
//! - Synthesize findings into comprehensive reports
//! - Track research progress and maintain context

pub mod engine;
pub mod history;
pub mod planner;
pub mod strategy;
pub mod synthesizer;
pub mod templates;
pub mod types;

pub use engine::ResearchEngine;
pub use history::*;
pub use planner::ResearchPlanner;
pub use strategy::*;
pub use synthesizer::ResearchSynthesizer;
pub use templates::*;
pub use types::*;
