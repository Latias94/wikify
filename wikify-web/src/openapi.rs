//! OpenAPI specification for Wikify Web Server
//!
//! This module defines the complete OpenAPI specification for the Wikify API.

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};

use crate::handlers::{
    ChatQueryRequest, ChatQueryResponse, GenerateWikiRequest, GenerateWikiResponse, HealthResponse,
    InitializeRepositoryRequest, InitializeRepositoryResponse, SourceDocument,
    WikiGenerationConfig,
};

/// Main OpenAPI specification for Wikify Web Server
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Wikify Web API",
        version = "0.1.0",
        description = "AI-powered repository documentation and chat system",
        contact(
            name = "Wikify Team",
            email = "support@wikify.dev"
        ),
        license(
            name = "MIT OR Apache-2.0"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "https://api.wikify.dev", description = "Production server")
    ),
    paths(
        // Health endpoints
        crate::handlers::health_check,

        // Repository management
        crate::handlers::initialize_repository,
        crate::handlers::get_repository_info,

        // Chat endpoints
        crate::handlers::chat_query,
        crate::handlers::chat_stream,

        // Wiki generation
        crate::handlers::generate_wiki,
        crate::handlers::get_wiki,
        crate::handlers::export_wiki,

        // Configuration
        crate::handlers::get_config,

        // SQLite-only endpoints (conditionally included)
        #[cfg(feature = "sqlite")]
        crate::handlers::get_repositories,
        #[cfg(feature = "sqlite")]
        crate::handlers::get_sessions,
        #[cfg(feature = "sqlite")]
        crate::handlers::get_query_history,
    ),
    components(
        schemas(
            HealthResponse,
            InitializeRepositoryRequest,
            InitializeRepositoryResponse,
            ChatQueryRequest,
            ChatQueryResponse,
            SourceDocument,
            GenerateWikiRequest,
            GenerateWikiResponse,
            WikiGenerationConfig,
        )
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Repository", description = "Repository management operations"),
        (name = "Chat", description = "AI chat and query operations"),
        (name = "Wiki", description = "Wiki generation and management"),
        (name = "Session", description = "Session management operations"),
        (name = "Configuration", description = "Server configuration operations"),
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security configuration for the API
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
            );
        }
    }
}

/// Get the OpenAPI specification as JSON
pub fn get_openapi_json() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap()
}

/// Get the OpenAPI specification as YAML
pub fn get_openapi_yaml() -> String {
    serde_yaml::to_string(&ApiDoc::openapi()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let openapi = ApiDoc::openapi();
        assert_eq!(openapi.info.title, "Wikify Web API");
        assert_eq!(openapi.info.version, "0.1.0");
        assert!(!openapi.paths.paths.is_empty());
    }

    #[test]
    fn test_openapi_json() {
        let json = get_openapi_json();
        assert!(json.contains("Wikify Web API"));
        assert!(json.contains("0.1.0"));
    }

    #[test]
    fn test_openapi_yaml() {
        let yaml = get_openapi_yaml();
        assert!(yaml.contains("Wikify Web API"));
        assert!(yaml.contains("0.1.0"));
    }
}
