//! Chat and RAG query handlers

use super::types::{ChatQueryRequest, ChatQueryResponse, SourceDocument};
use crate::{auth::ModeAwareUser, AppState};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    Json as JsonExtractor,
};
use tracing::{error, info};

/// Helper function to convert User to PermissionContext for application layer
fn user_to_permission_context(user: &crate::auth::User) -> wikify_applications::PermissionContext {
    user.to_permission_context()
}

/// Helper function to save query to database
#[cfg(feature = "sqlite")]
async fn save_query_to_database(
    database: &crate::simple_database::SimpleDatabaseService,
    repository_id: &str,
    question: &str,
    answer: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let query = crate::simple_database::SimpleQuery {
        id: uuid::Uuid::new_v4().to_string(),
        repository_id: Some(repository_id.to_string()),
        question: question.to_string(),
        answer: answer.to_string(),
        created_at: chrono::Utc::now(),
    };

    database
        .save_query(&query)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Handle chat queries
#[utoipa::path(
    post,
    path = "/api/chat",
    tag = "Chat",
    summary = "Ask a question",
    description = "Ask a question about the repository using RAG (Retrieval-Augmented Generation)",
    request_body = ChatQueryRequest,
    responses(
        (status = 200, description = "Question answered successfully", body = ChatQueryResponse)
    )
)]
pub async fn chat_query(
    State(state): State<AppState>,
    ModeAwareUser(user): ModeAwareUser,
    JsonExtractor(request): JsonExtractor<ChatQueryRequest>,
) -> Result<Json<ChatQueryResponse>, StatusCode> {
    info!(
        "Processing chat query for repository: {} (user: {})",
        request.repository_id, user.id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    // No session activity update needed for repository-based queries

    // Repository-based query only
    let repository_id = &request.repository_id;

    let repo_query = wikify_applications::RepositoryQuery {
        question: request.question.clone(),
        max_results: request.max_results,
        parameters: None,
    };

    match state
        .application
        .query_repository(&context, &repository_id, repo_query)
        .await
    {
        Ok(repo_response) => {
            info!("✅ Repository query completed for: {}", repository_id);

            // Convert repository response to chat response format
            let sources: Vec<SourceDocument> = repo_response
                .sources
                .into_iter()
                .map(|source_path| SourceDocument {
                    file_path: source_path.clone(),
                    content: format!("Source: {}", source_path), // TODO: Get actual content
                    similarity_score: 0.8, // TODO: Get actual similarity score
                })
                .collect();

            let response = ChatQueryResponse {
                answer: repo_response.answer,
                sources,
                repository_id: repository_id.clone(),
                timestamp: chrono::Utc::now(),
            };

            // Save query to database if available
            #[cfg(feature = "sqlite")]
            if let Some(database) = &state.database {
                if let Err(e) = save_query_to_database(
                    database,
                    &response.repository_id,
                    &request.question,
                    &response.answer,
                )
                .await
                {
                    tracing::warn!("Failed to save query to database: {}", e);
                }
            }

            info!("Chat query completed successfully");
            Ok(Json(response))
        }
        Err(e) => {
            error!("❌ Repository query failed for {}: {}", repository_id, e);

            let error_answer = format!(
                "Sorry, I encountered an error while processing your question: {}",
                e
            );

            // Save failed query to database if available
            #[cfg(feature = "sqlite")]
            if let Some(database) = &state.database {
                if let Err(db_e) = save_query_to_database(
                    database,
                    repository_id,
                    &request.question,
                    &error_answer,
                )
                .await
                {
                    tracing::warn!("Failed to save failed query to database: {}", db_e);
                }
            }

            // Return error response
            let response = ChatQueryResponse {
                answer: error_answer,
                sources: vec![],
                repository_id: repository_id.clone(),
                timestamp: chrono::Utc::now(),
            };

            Ok(Json(response))
        }
    }
}

/// Get query history (SQLite feature only)
#[cfg(feature = "sqlite")]
#[utoipa::path(
    get,
    path = "/api/history/{repository_id}",
    tag = "Chat",
    summary = "Get query history",
    description = "Get chat history for a specific repository (requires SQLite feature)",
    params(
        ("repository_id" = String, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Query history retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_query_history(
    State(state): State<AppState>,
    crate::auth::RequireQuery(user): crate::auth::RequireQuery,
    axum::extract::Path(_repository_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Getting query history for repository (user: {})", user.id);

    // Convert to permission context for application layer
    let _context = user_to_permission_context(&user);
    if let Some(database) = &state.database {
        // For now, get all queries. TODO: Filter by repository_id when session-repository mapping is implemented
        match database.get_query_history(None, 50).await {
            Ok(queries) => {
                let queries_json: Vec<serde_json::Value> = queries
                    .into_iter()
                    .map(|query| {
                        serde_json::json!({
                            "id": query.id,
                            "repository_id": query.repository_id,
                            "question": query.question,
                            "answer": query.answer,
                            "created_at": query.created_at,
                        })
                    })
                    .collect();

                Ok(Json(serde_json::json!({
                    "queries": queries_json,
                    "count": queries_json.len()
                })))
            }
            Err(e) => {
                tracing::error!("Failed to get query history: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // 数据库未启用，返回空列表
        Ok(Json(serde_json::json!({
            "queries": [],
            "count": 0,
            "message": "Database not enabled"
        })))
    }
}

/// Handle streaming chat queries (placeholder)
#[utoipa::path(
    post,
    path = "/api/chat/stream",
    tag = "Chat",
    summary = "Stream chat response",
    description = "Ask a question and receive streaming response (not yet implemented)",
    request_body = ChatQueryRequest,
    responses(
        (status = 200, description = "Streaming response started"),
        (status = 501, description = "Not implemented")
    )
)]
pub async fn chat_stream(
    State(state): State<AppState>,
    ModeAwareUser(user): ModeAwareUser,
    JsonExtractor(request): JsonExtractor<ChatQueryRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    info!(
        "Starting chat stream for user: {} with repository: {}",
        user.id, request.repository_id
    );

    // Convert to permission context for application layer
    let context = user_to_permission_context(&user);

    let repository_id = &request.repository_id;

    // Create repository query
    let repo_query = wikify_applications::RepositoryQuery {
        question: request.question.clone(),
        max_results: request.max_results,
        parameters: None, // TODO: Add support for additional parameters
    };

    // TODO: Implement streaming query when available
    // For now, return a placeholder response
    Ok(Json(serde_json::json!({
        "message": "Streaming queries not yet implemented for repository-based queries",
        "repository_id": repository_id
    }))
    .into_response())
}
