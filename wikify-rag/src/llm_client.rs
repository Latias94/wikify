//! LLM client integration using siumai
//!
//! This module provides a unified interface for interacting with various
//! LLM providers through the siumai framework.

use crate::types::{LlmConfig, RagError, RagResult};
use siumai::models;
use siumai::prelude::*;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Unified LLM client that supports multiple providers
pub struct WikifyLlmClient {
    client: Box<dyn LlmClient>,
    config: LlmConfig,
}

impl WikifyLlmClient {
    /// Create a new LLM client
    pub async fn new(config: LlmConfig) -> RagResult<Self> {
        let client = Self::build_client(&config).await?;

        info!(
            "Created LLM client for provider: {} with model: {}",
            config.provider, config.model
        );

        Ok(Self { client, config })
    }

    /// Build the appropriate siumai client based on configuration
    async fn build_client(config: &LlmConfig) -> RagResult<Box<dyn LlmClient>> {
        match config.provider.as_str() {
            "openai" => {
                let api_key = config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .ok_or_else(|| RagError::Config("OpenAI API key not found".to_string()))?;

                let mut builder = LlmBuilder::new()
                    .openai()
                    .api_key(&api_key)
                    .model(&config.model)
                    .temperature(config.temperature);

                if let Some(max_tokens) = config.max_tokens {
                    builder = builder.max_tokens(max_tokens);
                }

                if let Some(base_url) = &config.base_url {
                    builder = builder.base_url(base_url);
                }

                let client = builder
                    .build()
                    .await
                    .map_err(|e| RagError::Llm(format!("Failed to build OpenAI client: {}", e)))?;

                Ok(Box::new(client))
            }
            "anthropic" => {
                let api_key = config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                    .ok_or_else(|| RagError::Config("Anthropic API key not found".to_string()))?;

                let mut builder = LlmBuilder::new()
                    .anthropic()
                    .api_key(&api_key)
                    .model(&config.model)
                    .temperature(config.temperature);

                if let Some(max_tokens) = config.max_tokens {
                    builder = builder.max_tokens(max_tokens);
                }

                let client = builder.build().await.map_err(|e| {
                    RagError::Llm(format!("Failed to build Anthropic client: {}", e))
                })?;

                Ok(Box::new(client))
            }
            "ollama" => {
                let base_url = config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434".to_string());

                let mut builder = LlmBuilder::new()
                    .ollama()
                    .model(&config.model)
                    .base_url(&base_url)
                    .temperature(config.temperature);

                if let Some(max_tokens) = config.max_tokens {
                    builder = builder.max_tokens(max_tokens);
                }

                let client = builder
                    .build()
                    .await
                    .map_err(|e| RagError::Llm(format!("Failed to build Ollama client: {}", e)))?;

                Ok(Box::new(client))
            }
            "groq" => {
                let api_key = config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("GROQ_API_KEY").ok())
                    .ok_or_else(|| RagError::Config("Groq API key not found".to_string()))?;

                let mut builder = LlmBuilder::new()
                    .groq()
                    .api_key(&api_key)
                    .model(&config.model)
                    .temperature(config.temperature);

                if let Some(max_tokens) = config.max_tokens {
                    builder = builder.max_tokens(max_tokens);
                }

                let client = builder
                    .build()
                    .await
                    .map_err(|e| RagError::Llm(format!("Failed to build Groq client: {}", e)))?;

                Ok(Box::new(client))
            }
            provider => Err(RagError::Config(format!(
                "Unsupported LLM provider: {}",
                provider
            ))),
        }
    }

    /// Generate a response using the LLM
    pub async fn generate(&self, messages: Vec<ChatMessage>) -> RagResult<String> {
        let start_time = Instant::now();

        debug!("Generating response with {} messages", messages.len());

        let response = self
            .client
            .chat(messages)
            .await
            .map_err(|e| RagError::Llm(format!("LLM generation failed: {}", e)))?;

        let generation_time = start_time.elapsed();

        if let Some(content) = response.content_text() {
            info!(
                "Generated response in {:?} ({} chars)",
                generation_time,
                content.len()
            );
            Ok(content.to_string())
        } else {
            Err(RagError::Llm("No text content in LLM response".to_string()))
        }
    }

    /// Generate a response with system and user messages
    pub async fn generate_with_system(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> RagResult<String> {
        let messages = vec![system!(system_prompt), user!(user_message)];

        self.generate(messages).await
    }

    /// Generate embeddings (if the provider supports it)
    pub async fn generate_embeddings(&self, texts: Vec<String>) -> RagResult<Vec<Vec<f32>>> {
        if let Some(embedding_client) = self.client.as_embedding_capability() {
            let response = embedding_client
                .embed(texts)
                .await
                .map_err(|e| RagError::Embedding(format!("Embedding generation failed: {}", e)))?;

            Ok(response.embeddings)
        } else {
            Err(RagError::Config(format!(
                "Provider {} does not support embeddings",
                self.config.provider
            )))
        }
    }

    /// Test the connection to the LLM provider
    pub async fn test_connection(&self) -> RagResult<()> {
        debug!(
            "Testing connection to LLM provider: {}",
            self.config.provider
        );

        let test_messages = vec![user!(
            "Hello! Please respond with 'OK' to confirm the connection."
        )];

        match self.generate(test_messages).await {
            Ok(response) => {
                info!(
                    "Connection test successful. Response: {}",
                    response.chars().take(50).collect::<String>()
                );
                Ok(())
            }
            Err(e) => {
                warn!("Connection test failed: {}", e);
                Err(e)
            }
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// Get model information
    pub fn model_info(&self) -> ModelInfo {
        ModelInfo {
            provider: self.config.provider.clone(),
            model: self.config.model.clone(),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        }
    }
}

/// Information about the current model
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub provider: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

impl ModelInfo {
    pub fn summary(&self) -> String {
        format!(
            "{}/{} (temp: {:.1})",
            self.provider, self.model, self.temperature
        )
    }
}

/// Helper functions for creating common LLM configurations
pub mod configs {
    use super::*;

    /// Create OpenAI GPT-4o-mini configuration
    pub fn openai_gpt4o_mini() -> LlmConfig {
        LlmConfig {
            provider: "openai".to_string(),
            model: models::openai::GPT_4O_MINI.to_string(),
            api_key: None,
            base_url: None,
            temperature: 0.7,
            max_tokens: Some(2000),
        }
    }

    /// Create Anthropic Claude Haiku configuration
    pub fn anthropic_claude_haiku() -> LlmConfig {
        LlmConfig {
            provider: "anthropic".to_string(),
            model: models::anthropic::CLAUDE_HAIKU_3_5.to_string(),
            api_key: None,
            base_url: None,
            temperature: 0.7,
            max_tokens: Some(2000),
        }
    }

    /// Create Ollama configuration
    pub fn ollama_llama3(base_url: Option<String>) -> LlmConfig {
        LlmConfig {
            provider: "ollama".to_string(),
            model: "llama3.2".to_string(),
            api_key: None,
            base_url: base_url.or_else(|| Some("http://localhost:11434".to_string())),
            temperature: 0.7,
            max_tokens: Some(2000),
        }
    }

    /// Create Groq configuration
    pub fn groq_llama3() -> LlmConfig {
        LlmConfig {
            provider: "groq".to_string(),
            model: "llama-3.1-8b-instant".to_string(),
            api_key: None,
            base_url: None,
            temperature: 0.7,
            max_tokens: Some(2000),
        }
    }
}

/// Helper function to create a client with automatic provider detection
pub async fn create_auto_client() -> RagResult<WikifyLlmClient> {
    // Try providers in order of preference
    let providers = vec![
        ("openai", "OPENAI_API_KEY", configs::openai_gpt4o_mini()),
        (
            "anthropic",
            "ANTHROPIC_API_KEY",
            configs::anthropic_claude_haiku(),
        ),
        ("groq", "GROQ_API_KEY", configs::groq_llama3()),
    ];

    for (provider_name, env_var, config) in providers {
        if std::env::var(env_var).is_ok() {
            info!("Auto-detected {} provider", provider_name);
            match WikifyLlmClient::new(config).await {
                Ok(client) => return Ok(client),
                Err(e) => {
                    warn!("Failed to create {} client: {}", provider_name, e);
                    continue;
                }
            }
        }
    }

    // Try Ollama as fallback (no API key required)
    info!("Trying Ollama as fallback");
    let ollama_config = configs::ollama_llama3(None);
    WikifyLlmClient::new(ollama_config).await
}
