//! Token counting utilities for accurate token estimation
//!
//! This module provides accurate token counting using tiktoken-rs,
//! which is essential for managing context windows and costs.

use crate::types::{RagError, RagResult};
use std::sync::OnceLock;
use tiktoken_rs::{get_bpe_from_model, CoreBPE};
use tracing::{debug, warn};

/// Token counter for different models
pub struct TokenCounter {
    encoder: CoreBPE,
    model_name: String,
}

impl TokenCounter {
    /// Create a new token counter for the specified model
    pub fn new(model_name: &str) -> RagResult<Self> {
        let encoder = get_bpe_from_model(model_name).map_err(|e| {
            RagError::Config(format!(
                "Failed to get encoder for model {}: {}",
                model_name, e
            ))
        })?;

        Ok(Self {
            encoder,
            model_name: model_name.to_string(),
        })
    }

    /// Count tokens in a text string
    pub fn count_tokens(&self, text: &str) -> usize {
        self.encoder.encode_with_special_tokens(text).len()
    }

    /// Count tokens in multiple text strings
    pub fn count_tokens_batch(&self, texts: &[String]) -> Vec<usize> {
        texts.iter().map(|text| self.count_tokens(text)).collect()
    }

    /// Estimate cost based on token count (rough estimation)
    pub fn estimate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        // Rough cost estimation (prices as of 2024, may change)
        let (input_cost_per_1k, output_cost_per_1k) = match self.model_name.as_str() {
            "gpt-4" => (0.03, 0.06),
            "gpt-4-turbo" => (0.01, 0.03),
            "gpt-4o" => (0.005, 0.015),
            "gpt-4o-mini" => (0.00015, 0.0006),
            "gpt-3.5-turbo" => (0.0015, 0.002),
            _ => {
                warn!("Unknown model for cost estimation: {}", self.model_name);
                (0.001, 0.002) // Default fallback
            }
        };

        let input_cost = (input_tokens as f64 / 1000.0) * input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * output_cost_per_1k;

        input_cost + output_cost
    }

    /// Get model name
    pub fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Global token counter instances (cached for performance)
static GPT4_COUNTER: OnceLock<TokenCounter> = OnceLock::new();
static GPT35_COUNTER: OnceLock<TokenCounter> = OnceLock::new();
static GPT4O_COUNTER: OnceLock<TokenCounter> = OnceLock::new();

/// Get a cached token counter for common models
pub fn get_token_counter(model_name: &str) -> RagResult<&'static TokenCounter> {
    match model_name {
        "gpt-4" | "gpt-4-turbo" => {
            GPT4_COUNTER.get_or_init(|| {
                TokenCounter::new("gpt-4").unwrap_or_else(|_| {
                    warn!("Failed to create GPT-4 counter, using fallback");
                    TokenCounter::new("gpt-4o").expect("Failed to create fallback counter")
                })
            });
            Ok(GPT4_COUNTER.get().unwrap())
        }
        "gpt-3.5-turbo" => {
            GPT35_COUNTER.get_or_init(|| {
                TokenCounter::new("gpt-3.5-turbo").unwrap_or_else(|_| {
                    warn!("Failed to create GPT-3.5 counter, using fallback");
                    TokenCounter::new("gpt-4o").expect("Failed to create fallback counter")
                })
            });
            Ok(GPT35_COUNTER.get().unwrap())
        }
        "gpt-4o" | "gpt-4o-mini" => {
            GPT4O_COUNTER.get_or_init(|| {
                TokenCounter::new("gpt-4o").unwrap_or_else(|_| {
                    warn!("Failed to create GPT-4o counter, using fallback");
                    // This should not fail, but if it does, we have a bigger problem
                    panic!("Failed to create any token counter")
                })
            });
            Ok(GPT4O_COUNTER.get().unwrap())
        }
        _ => {
            debug!("Creating new token counter for model: {}", model_name);
            // For unknown models, try to create a counter
            // If it fails, fallback to gpt-4o
            TokenCounter::new(model_name)
                .or_else(|_| {
                    warn!(
                        "Failed to create counter for {}, falling back to gpt-4o",
                        model_name
                    );
                    TokenCounter::new("gpt-4o")
                })
                .map(|counter| {
                    // We can't cache this easily without more complex code,
                    // so we'll just return a reference to a leaked counter
                    Box::leak(Box::new(counter)) as &'static TokenCounter
                })
        }
    }
}

/// Utility function to count tokens for a specific model
pub fn count_tokens(text: &str, model_name: &str) -> RagResult<usize> {
    let counter = get_token_counter(model_name)?;
    Ok(counter.count_tokens(text))
}

/// Utility function to count tokens for multiple texts
pub fn count_tokens_batch(texts: &[String], model_name: &str) -> RagResult<Vec<usize>> {
    let counter = get_token_counter(model_name)?;
    Ok(counter.count_tokens_batch(texts))
}

/// Context window limits for different models
pub fn get_context_limit(model_name: &str) -> usize {
    match model_name {
        "gpt-4" => 8192,
        "gpt-4-turbo" => 128000,
        "gpt-4o" => 128000,
        "gpt-4o-mini" => 128000,
        "gpt-3.5-turbo" => 16385,
        "claude-3-haiku" => 200000,
        "claude-3-sonnet" => 200000,
        "claude-3-opus" => 200000,
        _ => {
            warn!(
                "Unknown context limit for model: {}, using default 8192",
                model_name
            );
            8192
        }
    }
}

/// Check if text fits within model's context window
pub fn fits_in_context(text: &str, model_name: &str) -> RagResult<bool> {
    let token_count = count_tokens(text, model_name)?;
    let context_limit = get_context_limit(model_name);
    Ok(token_count <= context_limit)
}

/// Truncate text to fit within context window, preserving as much as possible
pub fn truncate_to_context(
    text: &str,
    model_name: &str,
    reserve_tokens: usize,
) -> RagResult<String> {
    let context_limit = get_context_limit(model_name);
    let max_tokens = context_limit.saturating_sub(reserve_tokens);

    let counter = get_token_counter(model_name)?;
    let tokens = counter.encoder.encode_with_special_tokens(text);

    if tokens.len() <= max_tokens {
        return Ok(text.to_string());
    }

    // Truncate tokens and decode back to text
    let truncated_tokens = &tokens[..max_tokens];
    let truncated_text = counter
        .encoder
        .decode(truncated_tokens.to_vec())
        .map_err(|e| RagError::Config(format!("Failed to decode truncated tokens: {}", e)))?;

    debug!(
        "Truncated text from {} to {} tokens",
        tokens.len(),
        max_tokens
    );
    Ok(truncated_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counting() {
        let counter = TokenCounter::new("gpt-4o").unwrap();

        let text = "Hello, world! This is a test.";
        let token_count = counter.count_tokens(text);

        assert!(token_count > 0);
        assert!(token_count < 20); // Should be reasonable for this short text
    }

    #[test]
    fn test_context_limits() {
        assert_eq!(get_context_limit("gpt-4"), 8192);
        assert_eq!(get_context_limit("gpt-4o"), 128000);
        assert_eq!(get_context_limit("unknown-model"), 8192);
    }

    #[test]
    fn test_fits_in_context() {
        let short_text = "Hello world";
        assert!(fits_in_context(short_text, "gpt-4").unwrap());

        // Very long text should not fit in smaller context windows
        let long_text = "word ".repeat(10000);
        assert!(!fits_in_context(&long_text, "gpt-4").unwrap());
    }

    #[test]
    fn test_cost_estimation() {
        let counter = TokenCounter::new("gpt-4o-mini").unwrap();
        let cost = counter.estimate_cost(1000, 500);
        assert!(cost > 0.0);
        assert!(cost < 1.0); // Should be reasonable for these token counts
    }
}
