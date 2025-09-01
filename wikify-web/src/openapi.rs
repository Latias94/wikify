//! OpenAPI specification for Wikify Web Server
//!
//! This module defines the complete OpenAPI specification for the Wikify API.

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};

use crate::{
    auth::{
        handlers::{AuthFeatures, AuthStatusResponse},
        users::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest},
    },
    handlers::types::{
        ChatQueryRequest, ChatQueryResponse, DeleteRepositoryResponse, GenerateWikiRequest,
        GenerateWikiResponse, HealthResponse, InitializeRepositoryRequest,
        InitializeRepositoryResponse, ResearchProgressResponse, SourceDocument,
        StartResearchFromTemplateRequest, StartResearchRequest, WikiGenerationConfig,
    },
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

        // Authentication endpoints
        crate::auth::handlers::get_auth_status,
        crate::auth::handlers::register_user,
        crate::auth::handlers::login_user,
        crate::auth::handlers::refresh_token,

        // Repository management
        crate::handlers::initialize_repository,
        crate::handlers::list_repositories,
        crate::handlers::get_repository_info,
        crate::handlers::delete_repository,
        crate::handlers::reindex_repository,

        // Chat endpoints
        crate::handlers::chat_query,
        crate::handlers::chat_stream,

        // Wiki generation
        crate::handlers::generate_wiki,
        crate::handlers::get_wiki,
        crate::handlers::export_wiki,

        // Configuration
        crate::handlers::get_config,

        // Research endpoints
        crate::handlers::start_research,
        crate::handlers::research_iteration,
        crate::handlers::get_research_progress,
        crate::handlers::get_research_progress_by_id,
        crate::handlers::stop_research,

        // Research template endpoints
        crate::handlers::list_research_templates,
        crate::handlers::get_research_template,
        crate::handlers::list_templates_by_category,
        crate::handlers::start_research_from_template,

        // Research history endpoints (TODO: Add utoipa::path annotations)
        // crate::handlers::get_research_history,
        // crate::handlers::get_research_record,
        // crate::handlers::delete_research_record,
        // crate::handlers::get_research_statistics,

        // File operations endpoints
        crate::handlers::get_file_tree,
        crate::handlers::get_file_content,
        crate::handlers::get_readme,

        // SQLite-only endpoints (conditionally included)
        #[cfg(feature = "sqlite")]
        crate::handlers::get_repositories,
        #[cfg(feature = "sqlite")]
        crate::handlers::get_query_history,
    ),
    components(
        schemas(
            // Authentication schemas
            AuthStatusResponse,
            AuthFeatures,
            RegisterRequest,
            LoginRequest,
            RefreshRequest,
            AuthResponse,
            // Other schemas
            HealthResponse,
            InitializeRepositoryRequest,
            InitializeRepositoryResponse,
            DeleteRepositoryResponse,
            ChatQueryRequest,
            ChatQueryResponse,
            SourceDocument,
            GenerateWikiRequest,
            GenerateWikiResponse,
            WikiGenerationConfig,
            StartResearchRequest,
            ResearchProgressResponse,
            StartResearchFromTemplateRequest,
            // File operation schemas (TODO: Add when properly imported)
            // GetFileTreeRequest,
            // FileTreeResponse,
            // RepositoryFileInfo,
            // GetFileContentRequest,
            // FileContentResponse,
            // GetReadmeRequest,
            // ReadmeResponse,
        )
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Authentication", description = "User authentication and authorization"),
        (name = "Repository", description = "Repository management operations"),
        (name = "Chat", description = "AI chat and query operations"),
        (name = "Wiki", description = "Wiki generation and management"),
        (name = "Research", description = "Deep research and investigation operations"),
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
