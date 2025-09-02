//! Factory for creating document indexers
//!
//! This module provides a factory pattern for creating different types of
//! document indexers, allowing runtime selection and easy switching between
//! implementations.

use crate::indexing::{
    enhanced::EnhancedDocumentIndexer,
    legacy::LegacyDocumentIndexer,
    traits::{DocumentIndexer, IndexerFactory, IndexerType, IndexingConfig, IndexingError},
};
use wikify_core::WikifyResult;

/// Default factory implementation for creating document indexers
pub struct DefaultIndexerFactory;

impl IndexerFactory for DefaultIndexerFactory {
    fn create_indexer(indexer_type: IndexerType) -> WikifyResult<DocumentIndexer> {
        Self::create_indexer_with_config(indexer_type, IndexingConfig::default())
    }

    fn create_indexer_with_config(
        indexer_type: IndexerType,
        config: IndexingConfig,
    ) -> WikifyResult<DocumentIndexer> {
        match indexer_type {
            IndexerType::Legacy => {
                let indexer = LegacyDocumentIndexer::with_config(config)?;
                Ok(DocumentIndexer::Legacy(indexer))
            }
            IndexerType::Enhanced => {
                let indexer = EnhancedDocumentIndexer::with_unified_config(config)?;
                Ok(DocumentIndexer::Enhanced(indexer))
            }
            IndexerType::CheungfunPipeline => {
                // For now, use enhanced indexer as it includes pipeline features
                // TODO: Implement dedicated cheungfun pipeline indexer
                let indexer = EnhancedDocumentIndexer::with_unified_config(config)?;
                Ok(DocumentIndexer::Enhanced(indexer))
            }
        }
    }
}

/// Convenience functions for creating indexers
impl DefaultIndexerFactory {
    /// Create the recommended indexer for a specific use case
    pub fn create_for_use_case(use_case: &str) -> WikifyResult<DocumentIndexer> {
        let indexer_type = Self::recommended_for_use_case(use_case);
        Self::create_indexer(indexer_type)
    }

    /// Create an indexer with use case specific configuration
    pub fn create_for_use_case_with_config(
        use_case: &str,
        mut config: IndexingConfig,
    ) -> WikifyResult<DocumentIndexer> {
        let indexer_type = Self::recommended_for_use_case(use_case);

        // Apply use case specific optimizations
        match use_case.to_lowercase().as_str() {
            "code" | "code_repository" | "programming" => {
                config = IndexingConfig::for_code_repository();
            }
            "enterprise" | "large_codebase" | "production" => {
                config = IndexingConfig::for_enterprise();
            }
            "documentation" | "docs" | "markdown" => {
                config = IndexingConfig::for_documentation();
            }
            _ => {} // Use provided config as-is
        }

        Self::create_indexer_with_config(indexer_type, config)
    }

    /// Create an indexer from string configuration
    pub fn create_from_string(indexer_type_str: &str) -> WikifyResult<DocumentIndexer> {
        let indexer_type = IndexerType::from_str(indexer_type_str).ok_or_else(|| {
            wikify_core::WikifyError::Config {
                message: format!("Unsupported indexer type: {}", indexer_type_str),
                source: None,
                context: wikify_core::ErrorContext::new("factory")
                    .with_operation("create_from_string"),
            }
        })?;

        Self::create_indexer(indexer_type)
    }

    /// Create an indexer from environment variable
    pub fn create_from_env() -> WikifyResult<DocumentIndexer> {
        let indexer_type_str =
            std::env::var("WIKIFY_INDEXER_TYPE").unwrap_or_else(|_| "enhanced".to_string());

        Self::create_from_string(&indexer_type_str)
    }

    /// List all available indexer implementations with their descriptions
    pub fn list_available_indexers() -> Vec<(IndexerType, &'static str)> {
        Self::available_types()
            .into_iter()
            .map(|t| (t, t.description()))
            .collect()
    }

    /// Get feature comparison between indexer types
    pub fn compare_features() -> Vec<(IndexerType, Vec<(&'static str, bool)>)> {
        let features = [
            "ast_code_splitting",
            "semantic_splitting",
            "markdown_structure",
            "batch_processing",
            "enterprise_optimized",
            "pipeline_integration",
        ];

        Self::available_types()
            .into_iter()
            .map(|indexer_type| {
                // Create a temporary indexer to check features
                let indexer = Self::create_indexer(indexer_type).unwrap();
                let feature_support: Vec<_> = features
                    .iter()
                    .map(|&feature| (feature, indexer.supports_feature(feature)))
                    .collect();
                (indexer_type, feature_support)
            })
            .collect()
    }
}

/// Global convenience functions
///
/// These functions provide a simple API for creating indexers without
/// needing to work with the factory directly.

/// Create the default recommended indexer (Enhanced)
pub fn create_default_indexer() -> WikifyResult<DocumentIndexer> {
    DefaultIndexerFactory::create_indexer(IndexerType::Enhanced)
}

/// Create an indexer optimized for code repositories
pub fn create_code_indexer() -> WikifyResult<DocumentIndexer> {
    DefaultIndexerFactory::create_for_use_case("code_repository")
}

/// Create an indexer optimized for enterprise use
pub fn create_enterprise_indexer() -> WikifyResult<DocumentIndexer> {
    DefaultIndexerFactory::create_for_use_case("enterprise")
}

/// Create an indexer optimized for documentation
pub fn create_documentation_indexer() -> WikifyResult<DocumentIndexer> {
    DefaultIndexerFactory::create_for_use_case("documentation")
}

/// Create a legacy indexer for backward compatibility
pub fn create_legacy_indexer() -> WikifyResult<DocumentIndexer> {
    DefaultIndexerFactory::create_indexer(IndexerType::Legacy)
}

/// Create an indexer from configuration
pub fn create_indexer_with_config(
    indexer_type: IndexerType,
    config: IndexingConfig,
) -> WikifyResult<DocumentIndexer> {
    DefaultIndexerFactory::create_indexer_with_config(indexer_type, config)
}

/// Create an indexer from string (useful for CLI/config files)
pub fn create_indexer_from_string(indexer_type: &str) -> WikifyResult<DocumentIndexer> {
    DefaultIndexerFactory::create_from_string(indexer_type)
}

/// Auto-detect and create the best indexer for the environment
pub fn create_auto_indexer() -> WikifyResult<DocumentIndexer> {
    // Try environment variable first
    if let Ok(indexer) = DefaultIndexerFactory::create_from_env() {
        return Ok(indexer);
    }

    // Fall back to default
    create_default_indexer()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_creates_all_types() {
        for indexer_type in DefaultIndexerFactory::available_types() {
            let result = DefaultIndexerFactory::create_indexer(indexer_type);
            assert!(
                result.is_ok(),
                "Failed to create {:?} indexer",
                indexer_type
            );

            let indexer = result.unwrap();
            assert_eq!(indexer.implementation_name(), indexer_type.as_str());
        }
    }

    #[test]
    fn test_use_case_recommendations() {
        let test_cases = [
            ("code", IndexerType::Enhanced),
            ("enterprise", IndexerType::CheungfunPipeline),
            ("documentation", IndexerType::Enhanced),
            ("legacy", IndexerType::Legacy),
        ];

        for (use_case, expected) in test_cases {
            let recommended = DefaultIndexerFactory::recommended_for_use_case(use_case);
            assert_eq!(
                recommended, expected,
                "Wrong recommendation for {}",
                use_case
            );
        }
    }

    #[test]
    fn test_string_parsing() {
        let test_cases = [
            ("legacy", Some(IndexerType::Legacy)),
            ("enhanced", Some(IndexerType::Enhanced)),
            ("cheungfun", Some(IndexerType::CheungfunPipeline)),
            ("invalid", None),
        ];

        for (input, expected) in test_cases {
            let result = IndexerType::from_str(input);
            assert_eq!(result, expected, "Wrong parsing result for {}", input);
        }
    }
}
