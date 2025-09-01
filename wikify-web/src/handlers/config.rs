//! Configuration management handlers

use crate::{auth::AdminUser, AppState};
use axum::{extract::State, http::StatusCode, response::Json, Json as JsonExtractor};
use tracing::info;

/// Get server configuration
#[utoipa::path(
    get,
    path = "/api/config",
    tag = "Configuration",
    summary = "Get server configuration",
    description = "Get current server configuration",
    responses(
        (status = 200, description = "Configuration retrieved successfully")
    )
)]
pub async fn get_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "host": state.config.host,
        "port": state.config.port,
        "dev_mode": state.config.dev_mode,
    }))
}

/// Update server configuration
pub async fn update_config(
    State(_state): State<AppState>,
    AdminUser(user): AdminUser,
    JsonExtractor(_request): JsonExtractor<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Updating server configuration (admin user: {})", user.id);
    // Placeholder for configuration update
    Ok(Json(serde_json::json!({
        "message": "Configuration update is not yet implemented"
    })))
}
