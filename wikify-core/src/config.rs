//! 配置管理

use crate::error::{WikifyError, WikifyResult};
use crate::types::WikifyConfig;

use std::path::Path;

impl Default for WikifyConfig {
    fn default() -> Self {
        Self {
            embedding: crate::types::EmbeddingConfig {
                provider: "openai".to_string(),
                model: "text-embedding-3-small".to_string(),
                dimensions: 256,
                batch_size: 500,
            },
            llm: crate::types::LlmConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_tokens: 4000,
            },
            repository: crate::types::RepositoryConfig {
                max_size_mb: 50000,
                excluded_dirs: vec![
                    ".git".to_string(),
                    "node_modules".to_string(),
                    "target".to_string(),
                    "build".to_string(),
                    "dist".to_string(),
                    ".venv".to_string(),
                    "venv".to_string(),
                ],
                excluded_files: vec![
                    "*.lock".to_string(),
                    "*.log".to_string(),
                    "*.tmp".to_string(),
                    "*.cache".to_string(),
                ],
                included_extensions: vec![
                    "rs".to_string(),
                    "py".to_string(),
                    "js".to_string(),
                    "ts".to_string(),
                    "md".to_string(),
                    "txt".to_string(),
                    "json".to_string(),
                    "yaml".to_string(),
                    "yml".to_string(),
                ],
            },
            storage: crate::types::StorageConfig {
                data_dir: "~/.wikify/data".to_string(),
                cache_dir: "~/.wikify/cache".to_string(),
                use_database: false,
            },
            rag: crate::types::RagConfig {
                similarity_threshold: 0.3, // Optimized for better recall based on testing
                top_k: 8,
                max_context_length: 12000,
                enable_reranking: false,
            },
            indexing: crate::types::IndexingConfig {
                chunk_size: 350,
                chunk_overlap: 100,
                sentence_aware: true,
                token_based_for_code: true,
                max_tokens_per_chunk: 250,
                preserve_markdown_structure: true,
                use_ast_code_splitting: true,
                max_file_size_mb: 10,
                max_files: Some(10000),
            },
        }
    }
}

impl WikifyConfig {
    /// 从文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> WikifyResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| WikifyError::Config {
            message: format!("Failed to read config file: {}", e),
            source: Some(Box::new(e)),
            context: crate::ErrorContext::new("config")
                .with_operation("read_file")
                .with_suggestion("Check if the config file exists and is readable"),
        })?;

        let config: WikifyConfig = toml::from_str(&content).map_err(|e| WikifyError::Config {
            message: format!("Failed to parse config: {}", e),
            source: Some(Box::new(e)),
            context: crate::ErrorContext::new("config")
                .with_operation("parse_toml")
                .with_suggestion("Check TOML syntax in config file"),
        })?;

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> WikifyResult<()> {
        let content = toml::to_string_pretty(self).map_err(|e| WikifyError::Config {
            message: format!("Failed to serialize config: {}", e),
            source: Some(Box::new(e)),
            context: crate::ErrorContext::new("config").with_operation("serialize_toml"),
        })?;

        std::fs::write(path, content).map_err(|e| WikifyError::Config {
            message: format!("Failed to write config file: {}", e),
            source: Some(Box::new(e)),
            context: crate::ErrorContext::new("config")
                .with_operation("write_file")
                .with_suggestion("Check if the directory exists and is writable"),
        })?;

        Ok(())
    }

    /// 验证配置
    pub fn validate(&self) -> WikifyResult<()> {
        if self.embedding.dimensions == 0 {
            return Err(WikifyError::Config {
                message: "Embedding dimensions must be greater than 0".to_string(),
                source: None,
                context: crate::ErrorContext::new("config")
                    .with_operation("validate")
                    .with_suggestion("Set embedding.dimensions to a positive value"),
            });
        }

        if self.llm.max_tokens == 0 {
            return Err(WikifyError::Config {
                message: "LLM max_tokens must be greater than 0".to_string(),
                source: None,
                context: crate::ErrorContext::new("config")
                    .with_operation("validate")
                    .with_suggestion("Set llm.max_tokens to a positive value"),
            });
        }

        if self.repository.max_size_mb == 0 {
            return Err(WikifyError::Config {
                message: "Repository max_size_mb must be greater than 0".to_string(),
                source: None,
                context: crate::ErrorContext::new("config")
                    .with_operation("validate")
                    .with_suggestion("Set repository.max_size_mb to a positive value"),
            });
        }

        Ok(())
    }
}
