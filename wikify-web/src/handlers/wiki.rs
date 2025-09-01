//! Wiki generation and management handlers

use super::types::{GenerateWikiRequest, GenerateWikiResponse, WikiResponse};
use crate::{
    auth::{RequireExport, RequireGenerateWiki},
    AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Json as JsonExtractor,
};
use tracing::{error, info};

/// Helper function to convert User to PermissionContext for application layer
fn user_to_permission_context(user: &crate::auth::User) -> wikify_applications::PermissionContext {
    user.to_permission_context()
}

/// Generate wiki for repository
#[utoipa::path(
    post,
    path = "/api/wiki/generate",
    tag = "Wiki",
    summary = "Generate wiki documentation",
    description = "Generate comprehensive wiki documentation for a repository",
    request_body = GenerateWikiRequest,
    responses(
        (status = 200, description = "Wiki generated successfully", body = GenerateWikiResponse),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Wiki generation failed")
    )
)]
pub async fn generate_wiki(
    State(state): State<AppState>,
    RequireGenerateWiki(user): RequireGenerateWiki,
    JsonExtractor(request): JsonExtractor<GenerateWikiRequest>,
) -> Result<Json<GenerateWikiResponse>, StatusCode> {
    info!(
        "Generating wiki for repository: {} (user: {})",
        request.repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);
    // Get repository info
    let repository = match state
        .application
        .get_repository(&context, &request.repository_id)
        .await
    {
        Ok(repository) => repository,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    // Create wiki configuration
    let mut wiki_config = wikify_wiki::WikiConfig::default();
    if let Some(language) = request.config.language {
        wiki_config.language = language;
    }
    if let Some(max_pages) = request.config.max_pages {
        wiki_config.max_pages = Some(max_pages);
    }
    if let Some(include_diagrams) = request.config.include_diagrams {
        wiki_config.include_diagrams = include_diagrams;
    }
    if let Some(comprehensive_view) = request.config.comprehensive_view {
        wiki_config.comprehensive_view = comprehensive_view;
    }

    // TODO: Implement wiki generation through application layer
    // For now, return a placeholder response
    let response = GenerateWikiResponse {
        wiki_id: request.repository_id.clone(), // Use repository_id as wiki_id
        status: "not_implemented".to_string(),
        pages_count: 0,
        sections_count: 0,
    };
    Ok(Json(response))
}

/// Get generated wiki
#[utoipa::path(
    get,
    path = "/api/wiki/{repository_id}",
    tag = "Wiki",
    summary = "Get generated wiki",
    description = "Retrieve the generated wiki documentation for a repository. No authentication required.",
    params(
        ("repository_id" = String, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Wiki retrieved successfully", body = WikiResponse),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Failed to generate wiki")
    )
)]
pub async fn get_wiki(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Getting wiki for repository: {}", repository_id);

    // Create anonymous permission context (no authentication required for wiki viewing)
    let context = state.create_anonymous_context();

    // Get repository info (verify repository exists)
    let repository = match state
        .application
        .get_repository(&context, &repository_id)
        .await
    {
        Ok(repository) => repository,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    // Check if wiki is stored in database first
    #[cfg(feature = "sqlite")]
    if let Some(ref database) = state.database {
        if let Ok(Some(wiki_record)) = database.get_wiki_by_repository(&repository_id).await {
            info!(
                "Returning database-stored wiki for repository: {}",
                repository_id
            );

            // Try to parse the stored structure, fallback to simple format if parsing fails
            let wiki_response = if let Some(structure_json) = &wiki_record.structure {
                if let Ok(wiki_structure) =
                    serde_json::from_str::<wikify_wiki::WikiStructure>(structure_json)
                {
                    // Convert WikiStructure to frontend format
                    serde_json::json!({
                        "id": wiki_structure.id,
                        "title": wiki_structure.title,
                        "description": wiki_structure.description,
                        "pages": wiki_structure.pages.iter().map(|page| {
                            serde_json::json!({
                                "id": page.id,
                                "title": page.title,
                                "content": page.content,
                                "description": page.description,
                                "importance": format!("{:?}", page.importance),
                                "file_paths": page.file_paths,
                                "related_pages": page.related_pages,
                                "parent_section": page.parent_section,
                                "tags": page.tags,
                                "reading_time": page.reading_time,
                                "generated_at": page.generated_at.to_rfc3339(),
                                "source_documents": page.source_documents.iter().map(|doc| {
                                    serde_json::json!({
                                        "path": doc.file_path,
                                        "title": doc.file_path.split('/').last().unwrap_or(&doc.file_path),
                                        "relevance_score": 1.0 // Default relevance score
                                    })
                                }).collect::<Vec<_>>()
                            })
                        }).collect::<Vec<_>>(),
                        "sections": wiki_structure.sections.iter().map(|section| {
                            serde_json::json!({
                                "id": section.id,
                                "title": section.title,
                                "description": section.description,
                                "pages": section.pages,
                                "subsections": section.subsections,
                                "importance": "Medium", // Default importance
                                "order": section.order
                            })
                        }).collect::<Vec<_>>()
                    })
                } else {
                    // Fallback to simple format
                    serde_json::json!({
                        "id": wiki_record.id,
                        "title": wiki_record.title,
                        "description": wiki_record.description.unwrap_or_default(),
                        "pages": [{
                            "id": "main",
                            "title": "Main Documentation",
                            "content": wiki_record.content,
                            "description": "Main documentation page",
                            "importance": "Critical",
                            "file_paths": [],
                            "related_pages": [],
                            "tags": ["documentation"],
                            "reading_time": (wiki_record.content.split_whitespace().count() / 200).max(1),
                            "generated_at": wiki_record.generated_at,
                            "source_documents": []
                        }],
                        "sections": []
                    })
                }
            } else {
                // No structure stored, use simple format
                serde_json::json!({
                    "id": wiki_record.id,
                    "title": wiki_record.title,
                    "description": wiki_record.description.unwrap_or_default(),
                    "pages": [{
                        "id": "main",
                        "title": "Main Documentation",
                        "content": wiki_record.content,
                        "description": "Main documentation page",
                        "importance": "Critical",
                        "file_paths": [],
                        "related_pages": [],
                        "tags": ["documentation"],
                        "reading_time": (wiki_record.content.split_whitespace().count() / 200).max(1),
                        "generated_at": wiki_record.generated_at,
                        "source_documents": []
                    }],
                    "sections": []
                })
            };

            return Ok(Json(wiki_response));
        }
    }

    // Fallback to memory cache
    let wiki_cache = state.wiki_cache.read().await;
    if let Some(cached_wiki) = wiki_cache.get(&repository_id) {
        info!(
            "Returning memory-cached wiki for repository: {}",
            repository_id
        );

        // Convert cached content to WikiStructure format
        let wiki_structure = serde_json::json!({
            "id": repository_id,
            "title": format!("{} Wiki", repository.url.split('/').last().unwrap_or("Repository")),
            "description": format!("Generated wiki for repository: {}", repository.url),
            "pages": [{
                "id": "main",
                "title": "Main Documentation",
                "content": cached_wiki.content,
                "description": "Main documentation page",
                "importance": "Critical",
                "file_paths": [],
                "related_pages": [],
                "tags": ["documentation"],
                "reading_time": (cached_wiki.content.split_whitespace().count() / 200).max(1), // ~200 words per minute
                "generated_at": cached_wiki.generated_at.to_rfc3339(),
                "source_documents": []
            }],
            "sections": []
        });

        return Ok(Json(wiki_structure));
    }
    drop(wiki_cache);

    // If no cached wiki, try to generate one
    info!(
        "No cached wiki found, generating new wiki for repository: {}",
        repository_id
    );

    // Generate wiki using wiki service
    let mut wiki_service = state.wiki_service.write().await;
    let wiki_config = wikify_wiki::WikiConfig::default();
    match wiki_service
        .generate_wiki(&repository.url, &wiki_config)
        .await
    {
        Ok(wiki_structure) => {
            // Extract actual markdown content from the first page, or create a summary
            let wiki_content = if let Some(first_page) = wiki_structure.pages.first() {
                first_page.content.clone()
            } else {
                format!(
                    "# {}\n\n{}\n\nNo content pages were generated.",
                    wiki_structure.title, wiki_structure.description
                )
            };
            let generated_at = chrono::Utc::now();

            // Store in database if available
            #[cfg(feature = "sqlite")]
            if let Some(ref database) = state.database {
                if let Err(e) = database
                    .store_wiki(&repository_id, &wiki_structure, &wiki_content)
                    .await
                {
                    error!("Failed to store wiki in database: {}", e);
                    // Continue with memory cache as fallback
                }
            }

            // Also cache in memory for faster access
            let cached_wiki = crate::state::CachedWiki {
                content: wiki_content.clone(),
                generated_at,
                repository: repository.url.clone(),
                format: "markdown".to_string(),
            };

            let mut wiki_cache = state.wiki_cache.write().await;
            wiki_cache.insert(repository_id.clone(), cached_wiki);
            drop(wiki_cache);

            info!(
                "Wiki generated and cached for repository: {}",
                repository_id
            );

            // Convert WikiStructure to the format expected by frontend
            let wiki_response = serde_json::json!({
                "id": wiki_structure.id,
                "title": wiki_structure.title,
                "description": wiki_structure.description,
                "pages": wiki_structure.pages.iter().map(|page| {
                    serde_json::json!({
                        "id": page.id,
                        "title": page.title,
                        "content": page.content,
                        "description": page.description,
                        "importance": format!("{:?}", page.importance),
                        "file_paths": page.file_paths,
                        "related_pages": page.related_pages,
                        "parent_section": page.parent_section,
                        "tags": page.tags,
                        "reading_time": page.reading_time,
                        "generated_at": page.generated_at.to_rfc3339(),
                        "source_documents": page.source_documents.iter().map(|doc| {
                            serde_json::json!({
                                "path": doc.file_path,
                                "title": doc.file_path.split('/').last().unwrap_or(&doc.file_path),
                                "relevance_score": 1.0 // Default relevance score
                            })
                        }).collect::<Vec<_>>()
                    })
                }).collect::<Vec<_>>(),
                "sections": wiki_structure.sections.iter().map(|section| {
                    serde_json::json!({
                        "id": section.id,
                        "title": section.title,
                        "description": section.description,
                        "pages": section.pages,
                        "subsections": section.subsections,
                        "importance": "Medium", // Default importance
                        "order": section.order
                    })
                }).collect::<Vec<_>>()
            });

            Ok(Json(wiki_response))
        }
        Err(e) => {
            error!(
                "Failed to generate wiki for repository {}: {}",
                repository_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Export wiki in various formats
#[utoipa::path(
    post,
    path = "/api/wiki/{repository_id}/export",
    tag = "Wiki",
    summary = "Export wiki",
    description = "Export generated wiki in various formats (not yet implemented)",
    params(
        ("repository_id" = String, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Wiki exported successfully"),
        (status = 501, description = "Not implemented")
    )
)]
pub async fn export_wiki(
    State(_state): State<AppState>,
    RequireExport(user): RequireExport,
    Path(_repository_id): Path<String>,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Exporting wiki for repository: {} (user: {})",
        _repository_id, user.id
    );

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
    // Placeholder for wiki export
    Ok(Json(serde_json::json!({
        "message": "Wiki export is not yet implemented"
    })))
}
