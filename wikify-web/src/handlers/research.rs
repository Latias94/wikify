//! Research functionality handlers

use super::types::{
    ResearchProgressResponse, ResearchTemplateResponse, StartResearchFromTemplateRequest,
    StartResearchRequest, StartResearchResponse,
};
use crate::{
    auth::{ModeAwareUser, RequireQuery},
    AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{sse::Event, Json, Sse},
    Json as JsonExtractor,
};
use futures_util::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;
use wikify_applications::research::types::ResearchStatus;
use wikify_applications::{ResearchCategory, ResearchTemplate};

/// Helper function to convert User to PermissionContext for application layer
fn user_to_permission_context(user: &crate::auth::User) -> wikify_applications::PermissionContext {
    user.to_permission_context()
}

/// Start research session
#[utoipa::path(
    post,
    path = "/api/research/start",
    tag = "Research",
    summary = "Start research session",
    description = "Start a new research session for a repository",
    request_body = StartResearchRequest,
    responses(
        (status = 200, description = "Research session started successfully", body = StartResearchResponse),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Failed to start research session")
    )
)]
pub async fn start_research(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    JsonExtractor(request): JsonExtractor<StartResearchRequest>,
) -> Result<Json<StartResearchResponse>, StatusCode> {
    info!(
        "Starting research for repository: {} (user: {})",
        request.repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Create research configuration from request
    let research_config = wikify_applications::ResearchConfig {
        max_iterations: request
            .config
            .as_ref()
            .and_then(|c| c.max_iterations)
            .unwrap_or(5),
        max_depth: 3,
        confidence_threshold: 0.7,
        max_sources_per_iteration: request
            .config
            .as_ref()
            .and_then(|c| c.max_sources_per_iteration)
            .unwrap_or(10),
        enable_parallel_research: true,
    };

    // Start research session using application layer
    match state
        .application
        .start_research(
            &context,
            &request.repository_id,
            request.research_question.clone(),
            Some(research_config.clone()),
        )
        .await
    {
        Ok(research_id) => {
            // Send research started event via WebSocket
            let _ =
                state
                    .progress_broadcaster
                    .send(crate::state::BroadcastMessage::IndexingUpdate(
                        crate::state::IndexingUpdate::ResearchStarted {
                            repository_id: request.repository_id.clone(),
                            research_id: research_id.clone(),
                            query: request.research_question,
                            total_iterations: research_config.max_iterations,
                        },
                    ));
            info!("Research session started successfully: {}", research_id);
            Ok(Json(StartResearchResponse {
                research_id,
                status: "started".to_string(),
                message: "Research session started successfully".to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to start research session: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Get research progress
#[utoipa::path(
    get,
    path = "/api/research/progress/{repository_id}",
    tag = "Research",
    summary = "Get research progress",
    description = "Get the current progress of a research session",
    params(
        ("repository_id" = String, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Research progress retrieved successfully", body = ResearchProgressResponse),
        (status = 404, description = "Research session not found"),
        (status = 500, description = "Failed to get research progress")
    )
)]
pub async fn get_research_progress(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(repository_id): Path<String>,
) -> Result<Json<ResearchProgressResponse>, StatusCode> {
    info!(
        "Getting research progress for repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Get research progress using application layer
    match state
        .application
        .get_research_progress(&context, &repository_id)
        .await
    {
        Ok(progress) => {
            info!("Research progress retrieved successfully");
            Ok(Json(ResearchProgressResponse::from(progress)))
        }
        Err(e) => {
            error!("Failed to get research progress: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Get research progress by ID
#[utoipa::path(
    get,
    path = "/api/research/{research_id}",
    tag = "Research",
    summary = "Get research progress by ID",
    description = "Get the current progress of a research session by research ID",
    params(
        ("research_id" = String, Path, description = "Research session ID")
    ),
    responses(
        (status = 200, description = "Research progress retrieved successfully", body = ResearchProgressResponse),
        (status = 404, description = "Research session not found"),
        (status = 500, description = "Failed to get research progress")
    )
)]
pub async fn get_research_progress_by_id(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(research_id): Path<String>,
) -> Result<Json<ResearchProgressResponse>, StatusCode> {
    info!(
        "Getting research progress for research ID: {} (user: {})",
        research_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Get research progress by ID using application layer
    match state
        .application
        .get_research_progress(&context, &research_id)
        .await
    {
        Ok(progress) => {
            info!("Research progress retrieved successfully");
            Ok(Json(ResearchProgressResponse::from(progress)))
        }
        Err(e) => {
            error!("Failed to get research progress: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Stop research session
#[utoipa::path(
    post,
    path = "/api/research/{research_id}/stop",
    tag = "Research",
    summary = "Stop research session",
    description = "Stop an active research session",
    params(
        ("research_id" = String, Path, description = "Research session ID")
    ),
    responses(
        (status = 200, description = "Research session stopped successfully"),
        (status = 404, description = "Research session not found"),
        (status = 500, description = "Failed to stop research session")
    )
)]
pub async fn stop_research(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(research_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Stopping research session: {} (user: {})",
        research_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Stop research session using application layer
    match state
        .application
        .cancel_research(&context, &research_id)
        .await
    {
        Ok(()) => {
            info!("Research session stopped successfully");
            Ok(Json(serde_json::json!({
                "status": "stopped",
                "message": "Research session stopped successfully",
                "research_id": research_id
            })))
        }
        Err(e) => {
            error!("Failed to stop research session: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// List research sessions
#[utoipa::path(
    get,
    path = "/api/research/sessions",
    tag = "Research",
    summary = "List research sessions",
    description = "List all research sessions for the current user",
    responses(
        (status = 200, description = "Research sessions listed successfully"),
        (status = 500, description = "Failed to list research sessions")
    )
)]
pub async fn list_research_sessions(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Listing research sessions (user: {})", user.id);

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // List research sessions using application layer
    match state.application.list_active_research(&context).await {
        Ok(research_ids) => {
            info!("Research sessions listed successfully");
            let sessions_json: Vec<serde_json::Value> = research_ids
                .into_iter()
                .map(|research_id| {
                    serde_json::json!({
                        "research_id": research_id,
                        "status": "active"
                    })
                })
                .collect();

            Ok(Json(serde_json::json!({
                "sessions": sessions_json,
                "count": sessions_json.len()
            })))
        }
        Err(e) => {
            error!("Failed to list research sessions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Execute research iteration
#[utoipa::path(
    post,
    path = "/api/research/iterate/{repository_id}",
    tag = "Research",
    summary = "Execute research iteration",
    description = "Execute a single research iteration for a repository",
    params(
        ("repository_id" = String, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Research iteration completed successfully", body = ResearchProgressResponse),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Failed to execute research iteration")
    )
)]
pub async fn research_iteration(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(repository_id): Path<String>,
) -> Result<Json<ResearchProgressResponse>, StatusCode> {
    info!(
        "Executing research iteration for repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Execute research iteration using application layer
    match state
        .application
        .research_iteration(&context, &repository_id, &repository_id) // Using repository_id as research_session_id
        .await
    {
        Ok(progress) => {
            info!("Research iteration completed successfully");
            Ok(Json(ResearchProgressResponse::from(progress)))
        }
        Err(e) => {
            error!("Failed to execute research iteration: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// List research templates
#[utoipa::path(
    get,
    path = "/api/research/templates",
    tag = "Research",
    summary = "List research templates",
    description = "List all available research templates",
    responses(
        (status = 200, description = "Research templates listed successfully"),
        (status = 500, description = "Failed to list research templates")
    )
)]
pub async fn list_research_templates(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Listing research templates");

    // List research templates using application layer
    match state.application.list_research_templates().await {
        Ok(templates) => {
            info!("Research templates listed successfully");
            let templates_json: Vec<ResearchTemplateResponse> = templates
                .into_iter()
                .map(ResearchTemplateResponse::from)
                .collect();

            Ok(Json(serde_json::json!({
                "templates": templates_json,
                "count": templates_json.len()
            })))
        }
        Err(e) => {
            error!("Failed to list research templates: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get research template by ID
#[utoipa::path(
    get,
    path = "/api/research/templates/{template_id}",
    tag = "Research",
    summary = "Get research template",
    description = "Get a specific research template by ID",
    params(
        ("template_id" = String, Path, description = "Template ID")
    ),
    responses(
        (status = 200, description = "Research template retrieved successfully", body = ResearchTemplateResponse),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Failed to get research template")
    )
)]
pub async fn get_research_template(
    State(state): State<AppState>,
    Path(template_id): Path<String>,
) -> Result<Json<ResearchTemplateResponse>, StatusCode> {
    info!("Getting research template: {}", template_id);

    // Get research template using application layer
    match state.application.get_research_template(&template_id).await {
        Ok(template) => {
            info!("Research template retrieved successfully");
            Ok(Json(ResearchTemplateResponse::from(template)))
        }
        Err(e) => {
            error!("Failed to get research template: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// List templates by category
#[utoipa::path(
    get,
    path = "/api/research/templates/category/{category}",
    tag = "Research",
    summary = "List templates by category",
    description = "List research templates filtered by category",
    params(
        ("category" = String, Path, description = "Template category")
    ),
    responses(
        (status = 200, description = "Research templates listed successfully"),
        (status = 500, description = "Failed to list research templates")
    )
)]
pub async fn list_templates_by_category(
    State(state): State<AppState>,
    Path(category): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Listing research templates by category: {}", category);

    // Parse category string to ResearchCategory enum
    let research_category = match category.to_lowercase().as_str() {
        "security" => ResearchCategory::Security,
        "architecture" => ResearchCategory::Architecture,
        "performance" => ResearchCategory::Performance,
        "documentation" => ResearchCategory::Documentation,
        "technical" => ResearchCategory::Technical,
        "business" => ResearchCategory::Business,
        "custom" => ResearchCategory::Custom,
        _ => {
            error!("Invalid category: {}", category);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // List research templates by category using application layer
    match state
        .application
        .list_templates_by_category(research_category)
        .await
    {
        Ok(templates) => {
            info!(
                "Research templates listed successfully for category: {}",
                category
            );
            let templates_json: Vec<ResearchTemplateResponse> = templates
                .into_iter()
                .map(ResearchTemplateResponse::from)
                .collect();

            Ok(Json(serde_json::json!({
                "templates": templates_json,
                "category": category,
                "count": templates_json.len()
            })))
        }
        Err(e) => {
            error!("Failed to list research templates by category: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Start research from template
#[utoipa::path(
    post,
    path = "/api/research/start-from-template",
    tag = "Research",
    summary = "Start research from template",
    description = "Start a new research session using a predefined template",
    request_body = StartResearchFromTemplateRequest,
    responses(
        (status = 200, description = "Research session started successfully", body = StartResearchResponse),
        (status = 404, description = "Repository or template not found"),
        (status = 500, description = "Failed to start research session")
    )
)]
pub async fn start_research_from_template(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    JsonExtractor(request): JsonExtractor<StartResearchFromTemplateRequest>,
) -> Result<Json<StartResearchResponse>, StatusCode> {
    info!(
        "Starting research from template: {} for repository: {} (user: {})",
        request.template_id, request.repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Start research from template using application layer
    match state
        .application
        .start_research_from_template(
            &context,
            &request.repository_id,
            &request.template_id,
            request.custom_questions,
            request.config_overrides,
        )
        .await
    {
        Ok(research_id) => {
            info!(
                "Research session started from template successfully: {}",
                research_id
            );
            Ok(Json(StartResearchResponse {
                research_id,
                status: "started".to_string(),
                message: "Research session started from template successfully".to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to start research session from template: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Get research history
pub async fn get_research_history(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Getting research history (user: {})", user.id);

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Get research history using application layer
    match state
        .application
        .get_research_history(&context, None, None)
        .await
    {
        Ok(history) => {
            info!("Research history retrieved successfully");
            let history_json: Vec<serde_json::Value> = history
                .into_iter()
                .map(|record| {
                    serde_json::json!({
                        "id": record.session_id,
                        "repository_id": record.context.repository_id,
                        "research_question": record.topic,
                        "status": format!("{:?}", record.status).to_lowercase(),
                        "created_at": record.created_at,
                        "updated_at": record.updated_at,
                        "findings": record.iterations.iter()
                            .flat_map(|iter| &iter.findings)
                            .map(|finding| &finding.content)
                            .collect::<Vec<_>>(),
                        "metadata": record.metadata
                    })
                })
                .collect();

            Ok(Json(serde_json::json!({
                "history": history_json,
                "count": history_json.len()
            })))
        }
        Err(e) => {
            error!("Failed to get research history: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get research record by repository ID
pub async fn get_research_record(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(repository_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Getting research record for repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Get research record using application layer
    match state
        .application
        .get_research_record(&context, &repository_id)
        .await
    {
        Ok(record) => {
            info!("Research record retrieved successfully");
            let record_json = serde_json::json!({
                "id": record.session_id,
                "repository_id": record.context.repository_id,
                "research_question": record.topic,
                "status": format!("{:?}", record.status).to_lowercase(),
                "created_at": record.created_at,
                "updated_at": record.updated_at,
                "findings": record.iterations.iter()
                    .flat_map(|iter| &iter.findings)
                    .map(|finding| &finding.content)
                    .collect::<Vec<_>>(),
                "metadata": record.metadata
            });

            Ok(Json(record_json))
        }
        Err(e) => {
            error!("Failed to get research record: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Delete research record
pub async fn delete_research_record(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(repository_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Deleting research record for repository: {} (user: {})",
        repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Delete research record using application layer
    match state
        .application
        .delete_research_record(&context, &repository_id)
        .await
    {
        Ok(()) => {
            info!("Research record deleted successfully");
            Ok(Json(serde_json::json!({
                "status": "deleted",
                "message": "Research record deleted successfully",
                "repository_id": repository_id
            })))
        }
        Err(e) => {
            error!("Failed to delete research record: {}", e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Get research statistics
pub async fn get_research_statistics(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Getting research statistics (user: {})", user.id);

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Get research statistics using application layer
    match state.application.get_research_statistics(&context).await {
        Ok(stats) => {
            info!("Research statistics retrieved successfully");
            let stats_json = serde_json::json!({
                "total_sessions": stats.total_sessions,
                "active_sessions": stats.in_progress_sessions,
                "completed_sessions": stats.completed_sessions,
                "failed_sessions": stats.failed_sessions,
                "average_session_duration": stats.average_duration_seconds,
                "popular_templates": stats.popular_templates,
                "activity_by_date": stats.activity_by_date
            });

            Ok(Json(stats_json))
        }
        Err(e) => {
            error!("Failed to get research statistics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ============================================================================
// Deep Research Streaming Endpoints
// ============================================================================

/// Start deep research with streaming response
#[utoipa::path(
    post,
    path = "/api/research/deep-stream",
    tag = "Research",
    summary = "Start deep research with streaming response",
    description = "Start a new deep research session with real-time streaming updates",
    request_body = StartResearchRequest,
    responses(
        (status = 200, description = "Streaming research updates", content_type = "text/event-stream"),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Failed to start research session")
    )
)]
pub async fn start_deep_research_stream(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    JsonExtractor(request): JsonExtractor<StartResearchRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    info!(
        "Starting deep research stream for repository: {} (user: {})",
        request.repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // Convert request config to application config
    let config = request
        .config
        .map(|c| wikify_applications::research::ResearchConfig {
            max_iterations: c.max_iterations.unwrap_or(5),
            max_depth: 3,
            confidence_threshold: 0.7,
            max_sources_per_iteration: c.max_sources_per_iteration.unwrap_or(10),
            enable_parallel_research: true,
        });

    // Start research session
    let research_id = match state
        .application
        .start_research(
            &context,
            &request.repository_id,
            request.research_question.clone(),
            config,
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to start research: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    info!("Started deep research session: {}", research_id);

    // Create streaming response
    let stream = create_research_progress_stream(state, research_id, request.research_question);

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    ))
}

/// Create a stream of research progress updates
fn create_research_progress_stream(
    state: AppState,
    research_id: String,
    original_query: String,
) -> impl Stream<Item = Result<Event, Infallible>> {
    stream::unfold(
        (state, research_id, original_query, 0u32, false),
        |(state, research_id, original_query, poll_count, completed)| async move {
            // If already completed, end the stream
            if completed {
                return None;
            }

            // Poll for research progress
            let context = wikify_applications::PermissionContext::local();
            match state
                .application
                .get_research_progress(&context, &research_id)
                .await
            {
                Ok(progress) => {
                    // Create detailed progress event
                    let event_data = serde_json::json!({
                        "type": "progress",
                        "research_id": research_id,
                        "original_query": original_query,
                        "status": format!("{:?}", progress.status),
                        "current_iteration": progress.current_iteration,
                        "max_iterations": progress.max_iterations,
                        "progress": progress.progress,
                        "current_response": progress.current_response,
                        "last_updated": progress.last_updated,
                        "poll_count": poll_count,
                        "timestamp": chrono::Utc::now()
                    });

                    let progress_event = Event::default()
                        .event("research_progress")
                        .data(event_data.to_string());

                    // Check if research is complete
                    let is_complete = matches!(
                        progress.status,
                        ResearchStatus::Completed
                            | ResearchStatus::Cancelled
                            | ResearchStatus::Failed(_)
                    );

                    if is_complete {
                        // Mark as completed to end stream after this event
                        Some((
                            Ok(progress_event),
                            (state, research_id, original_query, poll_count + 1, true),
                        ))
                    } else {
                        // Continue polling after a delay
                        tokio::time::sleep(Duration::from_millis(1000)).await;
                        Some((
                            Ok(progress_event),
                            (state, research_id, original_query, poll_count + 1, false),
                        ))
                    }
                }
                Err(e) => {
                    // Send error event and end stream
                    let error_event = Event::default().event("research_error").data(
                        serde_json::json!({
                            "type": "error",
                            "research_id": research_id,
                            "error": format!("Failed to get research progress: {}", e),
                            "timestamp": chrono::Utc::now()
                        })
                        .to_string(),
                    );

                    // End stream after error
                    Some((
                        Ok(error_event),
                        (state, research_id, original_query, poll_count + 1, true),
                    ))
                }
            }
        },
    )
}

/// Get detailed research result
#[utoipa::path(
    get,
    path = "/api/research/{research_id}/result",
    tag = "Research",
    summary = "Get detailed research result",
    description = "Get the complete research result including all iterations and final synthesis",
    responses(
        (status = 200, description = "Research result retrieved successfully"),
        (status = 404, description = "Research not found"),
        (status = 500, description = "Failed to get research result")
    )
)]
pub async fn get_research_result(
    State(state): State<AppState>,
    RequireQuery(user): RequireQuery,
    Path(research_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Getting research result for: {} (user: {})",
        research_id, user.id
    );

    let context = user_to_permission_context(&user);

    match state
        .application
        .get_research_details(&context, &research_id)
        .await
    {
        Ok(details) => {
            let response = serde_json::json!({
                "research_id": research_id,
                "topic": details.topic,
                "status": format!("{:?}", details.status),
                "iterations": details.iterations.iter().map(|iter| serde_json::json!({
                    "iteration": iter.iteration,
                    "questions": iter.questions,
                    "findings": iter.findings,
                    "new_questions": iter.new_questions,
                    "partial_synthesis": iter.partial_synthesis,
                    "confidence": iter.confidence,
                    "needs_more_research": iter.needs_more_research,
                    "duration_ms": iter.duration.as_millis()
                })).collect::<Vec<_>>(),
                "findings": details.findings,
                "questions": details.questions,
                "config": serde_json::json!({
                    "max_iterations": details.config.max_iterations,
                    "max_depth": details.config.max_depth,
                    "confidence_threshold": details.config.confidence_threshold
                }),
                "created_at": details.created_at,
                "updated_at": details.updated_at
            });

            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get research result: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
