//! Unified WebSocket handler for real-time communication
//!
//! This module provides a single WebSocket endpoint that handles all types of real-time communication:
//! - Chat messages and responses
//! - Wiki generation progress
//! - Repository indexing progress
//! - System notifications

use crate::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid;

/// Error types for unified error handling
#[derive(Debug, Clone)]
pub enum ErrorType {
    Chat,
    Wiki,
    Index,
    Research,
    General,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Chat message
    Chat {
        repository_id: String,
        question: String,
        context: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Chat response
    ChatResponse {
        repository_id: String,
        answer: String,
        sources: Vec<SourceDocument>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
        is_streaming: Option<bool>,
        is_complete: Option<bool>,
        chunk_id: Option<String>,
    },
    /// Chat error
    ChatError {
        repository_id: String,
        error: String,
        details: Option<serde_json::Value>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Wiki generation request
    WikiGenerate {
        repository_id: String,
        config: WikiConfig,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Wiki generation progress
    WikiProgress {
        repository_id: String,
        progress: f64,
        current_step: String,
        total_steps: usize,
        completed_steps: usize,
        step_details: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Wiki generation complete
    WikiComplete {
        repository_id: String,
        wiki_id: String,
        pages_count: usize,
        sections_count: usize,
        metadata: Option<WikiMetadata>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Wiki generation error
    WikiError {
        repository_id: String,
        error: String,
        details: Option<serde_json::Value>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Indexing started
    IndexStart {
        repository_id: String,
        total_files: Option<usize>,
        estimated_duration: Option<u64>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Indexing progress
    IndexProgress {
        repository_id: String,
        progress: f64,
        files_processed: usize,
        total_files: usize,
        current_file: Option<String>,
        processing_rate: Option<f64>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Indexing complete
    IndexComplete {
        repository_id: String,
        total_files: usize,
        processing_time: Option<u64>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Indexing error
    IndexError {
        repository_id: String,
        error: String,
        details: Option<serde_json::Value>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Research started
    ResearchStart {
        repository_id: String,
        research_id: String,
        query: String,
        total_iterations: usize,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Research progress
    ResearchProgress {
        repository_id: String,
        research_id: String,
        current_iteration: usize,
        total_iterations: usize,
        current_focus: String,
        progress: f64,
        findings: Vec<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Research complete
    ResearchComplete {
        repository_id: String,
        research_id: String,
        total_iterations: usize,
        final_conclusion: String,
        all_findings: Vec<String>,
        processing_time: Option<u64>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Research error
    ResearchError {
        repository_id: String,
        research_id: String,
        error: String,
        details: Option<serde_json::Value>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Ping message for heartbeat
    Ping {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Pong response to ping
    Pong {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// General system error
    Error {
        message: String,
        code: Option<String>,
        details: Option<serde_json::Value>,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
}

// Use the unified SourceDocument from handlers::types
use crate::handlers::types::SourceDocument;

// Use unified types from handlers::types
use crate::handlers::types::{WikiConfig, WikiMetadata};

/// Generate a unique message ID
fn generate_message_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Send a WebSocket message with proper error handling
async fn send_message(
    socket: &mut WebSocket,
    message: WsMessage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let msg_str = serde_json::to_string(&message)?;
    socket.send(Message::Text(msg_str.into())).await?;
    Ok(())
}

/// Send an error response with unified error handling
async fn send_error_response(
    socket: &mut WebSocket,
    repository_id: String,
    error: String,
    error_type: ErrorType,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let error_msg = match error_type {
        ErrorType::Chat => WsMessage::ChatError {
            repository_id,
            error,
            details: None,
            timestamp: chrono::Utc::now(),
            id: Some(generate_message_id()),
        },
        ErrorType::Wiki => WsMessage::WikiError {
            repository_id,
            error,
            details: None,
            timestamp: chrono::Utc::now(),
            id: Some(generate_message_id()),
        },
        ErrorType::Index => WsMessage::IndexError {
            repository_id,
            error,
            details: None,
            timestamp: chrono::Utc::now(),
            id: Some(generate_message_id()),
        },
        ErrorType::Research => WsMessage::ResearchError {
            repository_id,
            research_id: "unknown".to_string(), // Default research_id for errors
            error,
            details: None,
            timestamp: chrono::Utc::now(),
            id: Some(generate_message_id()),
        },
        ErrorType::General => WsMessage::Error {
            message: error,
            code: None,
            details: None,
            timestamp: chrono::Utc::now(),
            id: Some(generate_message_id()),
        },
    };

    send_message(socket, error_msg).await
}

/// Create a message with guaranteed ID
fn create_message_with_id<F>(create_fn: F) -> WsMessage
where
    F: FnOnce(String) -> WsMessage,
{
    let id = generate_message_id();
    create_fn(id)
}

/// Unified WebSocket handler for all real-time communication
pub async fn unified_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_unified_socket(socket, state))
}

/// Unified WebSocket connection handler
/// Handles all types of WebSocket communication: chat, wiki, indexing, and progress updates
async fn handle_unified_socket(mut socket: WebSocket, state: AppState) {
    info!("New unified WebSocket connection established");

    // Send welcome message
    let welcome = create_message_with_id(|id| {
        WsMessage::ChatResponse {
        repository_id: "system".to_string(),
        answer: "WebSocket connection established. You will receive real-time updates for all operations.".to_string(),
        sources: vec![],
        timestamp: chrono::Utc::now(),
        id: Some(id),
        is_streaming: Some(false),
        is_complete: Some(true),
        chunk_id: None,
    }
    });

    if let Ok(msg) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(msg.into())).await.is_err() {
            return;
        }
    }

    // Subscribe to progress updates
    let mut progress_receiver = state.progress_broadcaster.subscribe();
    info!("Subscribed to progress broadcaster");

    // Track sent messages to avoid duplicates
    let mut sent_messages = HashSet::new();

    loop {
        tokio::select! {
            // Handle incoming messages from client
            msg_result = socket.recv() => {
                match msg_result {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_unified_message(&mut socket, &state, &text).await {
                            error!("Error handling unified message: {}", e);
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Unified WebSocket connection closed by client");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("Unified WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("Unified WebSocket connection terminated");
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast messages from broadcaster
            broadcast_result = progress_receiver.recv() => {
                match broadcast_result {
                    Ok(broadcast_msg) => {
                        // Create unique message ID to prevent duplicates
                        let message_id = match &broadcast_msg {
                            crate::state::BroadcastMessage::IndexingUpdate(update) => {
                                match update {
                                    crate::state::IndexingUpdate::Started { repository_id, .. } => {
                                        format!("started-{}", repository_id)
                                    }
                                    crate::state::IndexingUpdate::Progress { repository_id, percentage, .. } => {
                                        format!("progress-{}-{:.3}", repository_id, percentage)
                                    }
                                    crate::state::IndexingUpdate::Complete { repository_id, .. } => {
                                        format!("complete-{}", repository_id)
                                    }
                                    crate::state::IndexingUpdate::Error { repository_id, .. } => {
                                        format!("error-{}", repository_id)
                                    }
                                    crate::state::IndexingUpdate::ResearchStarted { repository_id, research_id, .. } => {
                                        format!("research-started-{}-{}", repository_id, research_id)
                                    }
                                    crate::state::IndexingUpdate::ResearchProgress { repository_id, research_id, current_iteration, .. } => {
                                        format!("research-progress-{}-{}-{}", repository_id, research_id, current_iteration)
                                    }
                                    crate::state::IndexingUpdate::ResearchComplete { repository_id, research_id, .. } => {
                                        format!("research-complete-{}-{}", repository_id, research_id)
                                    }
                                    crate::state::IndexingUpdate::ResearchError { repository_id, research_id, .. } => {
                                        format!("research-error-{}-{}", repository_id, research_id)
                                    }
                                    _ => format!("indexing-other-{}", chrono::Utc::now().timestamp_millis())
                                }
                            }
                            crate::state::BroadcastMessage::WebSocketMessage { message: _ } => {
                                format!("ws-{}", chrono::Utc::now().timestamp_millis())
                            }
                        };

                        // Skip if we've already sent this message
                        if sent_messages.contains(&message_id) {
                            continue;
                        }
                        sent_messages.insert(message_id);

                        // Clean up old message IDs to prevent memory leak
                        if sent_messages.len() > 1000 {
                            sent_messages.clear();
                        }

                        info!("Processing broadcast message: {:?}", broadcast_msg);

                        // Convert to WebSocket message and send
                        let ws_message = match broadcast_msg {
                            crate::state::BroadcastMessage::IndexingUpdate(update) => {
                                convert_update_to_message(update)
                            }
                            crate::state::BroadcastMessage::WebSocketMessage { message } => {
                                Some(message)
                            }
                        };

                        if let Some(ws_msg) = ws_message {
                            if let Ok(msg) = serde_json::to_string(&ws_msg) {
                                if socket.send(Message::Text(msg.into())).await.is_err() {
                                    error!("Failed to send broadcast message, connection closed");
                                    break;
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Progress receiver lagged, skipped {} messages", skipped);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Progress broadcaster closed");
                        break;
                    }
                }
            }
        }
    }

    info!("Unified WebSocket connection ended");
}

/// Handle unified WebSocket messages (chat, wiki generation, etc.)
async fn handle_unified_message(
    socket: &mut WebSocket,
    state: &AppState,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message: WsMessage = serde_json::from_str(text)?;

    match message {
        WsMessage::Chat {
            repository_id,
            question,
            context,
            ..
        } => {
            handle_chat_request(socket, state, repository_id, question, context).await?;
        }
        WsMessage::WikiGenerate {
            repository_id,
            config,
            ..
        } => {
            handle_wiki_request(socket, state, repository_id, config).await?;
        }
        WsMessage::Ping { .. } => {
            // Respond with pong
            let pong = WsMessage::Pong {
                timestamp: chrono::Utc::now(),
            };
            send_message(socket, pong).await?;
        }
        WsMessage::Pong { .. } => {
            // Acknowledge pong, no action needed
        }
        _ => {
            warn!("Received unsupported message type in unified handler");
        }
    }

    Ok(())
}

/// Calculate wiki generation steps based on stage and progress
fn calculate_wiki_steps(stage: &str, progress: f64) -> (usize, usize) {
    // 定义Wiki生成的主要阶段
    let stages = [
        "Initializing wiki generation",
        "Analyzing repository structure",
        "Generating wiki content",
        "Finalizing wiki generation",
    ];

    let total_steps = stages.len();

    // 根据阶段名称确定当前步骤
    let current_stage_index = stages.iter().position(|&s| stage.contains(s)).unwrap_or(0);

    // 计算完成的步骤数：前面完成的步骤 + 当前步骤的进度
    let completed_steps = if progress >= 1.0 {
        total_steps // 如果当前阶段完成，所有步骤都完成
    } else {
        current_stage_index + if progress > 0.0 { 1 } else { 0 }
    };

    (total_steps, completed_steps.min(total_steps))
}

/// Convert IndexingUpdate to WebSocket message
fn convert_update_to_message(update: crate::state::IndexingUpdate) -> Option<WsMessage> {
    match update {
        crate::state::IndexingUpdate::Started {
            repository_id,
            total_files,
            estimated_duration,
        } => Some(WsMessage::IndexStart {
            repository_id,
            total_files,
            estimated_duration,
            timestamp: chrono::Utc::now(),
            id: Some(generate_message_id()),
        }),
        crate::state::IndexingUpdate::Progress {
            repository_id,
            percentage,
            files_processed,
            total_files,
            current_item,
            ..
        } => {
            let processed = files_processed.unwrap_or(0);
            let total = total_files.unwrap_or(0);

            // 简单的处理速率估算：假设每个文件平均处理时间为0.1秒
            let processing_rate = if processed > 0 && total > 0 {
                Some(10.0) // 大约10文件/秒的估算值
            } else {
                None
            };

            Some(WsMessage::IndexProgress {
                repository_id,
                progress: percentage, // percentage is already in 0.0-1.0 range
                files_processed: processed,
                total_files: total,
                current_file: current_item,
                processing_rate,
                timestamp: chrono::Utc::now(),
                id: Some(generate_message_id()),
            })
        }
        crate::state::IndexingUpdate::Complete {
            repository_id,
            total_files,
            ..
        } => Some(WsMessage::IndexComplete {
            repository_id,
            total_files,
            processing_time: None,
            timestamp: chrono::Utc::now(),
            id: Some(generate_message_id()),
        }),
        crate::state::IndexingUpdate::Error {
            repository_id,
            error,
            ..
        } => Some(WsMessage::IndexError {
            repository_id,
            error,
            details: None,
            timestamp: chrono::Utc::now(),
            id: None,
        }),
        crate::state::IndexingUpdate::WikiGenerationProgress {
            repository_id,
            stage,
            percentage,
            ..
        } => {
            let clamped_progress = percentage.clamp(0.0, 1.0);
            // 根据阶段确定总步骤数和当前步骤
            let (total_steps, completed_steps) = calculate_wiki_steps(&stage, clamped_progress);

            Some(WsMessage::WikiProgress {
                repository_id,
                progress: clamped_progress,
                current_step: stage,
                total_steps,
                completed_steps,
                step_details: None,
                timestamp: chrono::Utc::now(),
                id: None,
            })
        }
        crate::state::IndexingUpdate::WikiGenerationComplete { repository_id, .. } => {
            Some(WsMessage::WikiComplete {
                repository_id,
                wiki_id: "generated".to_string(),
                pages_count: 0,
                sections_count: 0,
                metadata: None,
                timestamp: chrono::Utc::now(),
                id: None,
            })
        }
        crate::state::IndexingUpdate::WikiGenerationStarted { repository_id } => {
            Some(WsMessage::WikiProgress {
                repository_id,
                progress: 0.0,
                current_step: "Starting wiki generation...".to_string(),
                total_steps: 4, // 预设的Wiki生成步骤数
                completed_steps: 0,
                step_details: None,
                timestamp: chrono::Utc::now(),
                id: None,
            })
        }
        crate::state::IndexingUpdate::WikiGenerationError {
            repository_id,
            error,
            ..
        } => Some(WsMessage::WikiError {
            repository_id,
            error,
            details: None,
            timestamp: chrono::Utc::now(),
            id: None,
        }),
        crate::state::IndexingUpdate::ResearchStarted {
            repository_id,
            research_id,
            query,
            total_iterations,
        } => Some(WsMessage::ResearchStart {
            repository_id,
            research_id,
            query,
            total_iterations,
            timestamp: chrono::Utc::now(),
            id: None,
        }),
        crate::state::IndexingUpdate::ResearchProgress {
            repository_id,
            research_id,
            current_iteration,
            total_iterations,
            current_focus,
            progress,
            findings,
        } => Some(WsMessage::ResearchProgress {
            repository_id,
            research_id,
            current_iteration,
            total_iterations,
            current_focus,
            progress,
            findings,
            timestamp: chrono::Utc::now(),
            id: None,
        }),
        crate::state::IndexingUpdate::ResearchComplete {
            repository_id,
            research_id,
            total_iterations,
            final_conclusion,
            all_findings,
            processing_time,
        } => Some(WsMessage::ResearchComplete {
            repository_id,
            research_id,
            total_iterations,
            final_conclusion,
            all_findings,
            processing_time,
            timestamp: chrono::Utc::now(),
            id: None,
        }),
        crate::state::IndexingUpdate::ResearchError {
            repository_id,
            research_id,
            error,
        } => Some(WsMessage::ResearchError {
            repository_id,
            research_id,
            error,
            details: None,
            timestamp: chrono::Utc::now(),
            id: None,
        }),
        _ => None, // Skip unknown update types
    }
}

/// Handle chat requests
async fn handle_chat_request(
    socket: &mut WebSocket,
    state: &AppState,
    repository_id: String,
    question: String,
    _context: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Processing chat request for repository: {}", repository_id);

    // Create a local permission context for system operations
    let permission_context = wikify_applications::PermissionContext::local();

    // Create repository query
    let repo_query = wikify_applications::RepositoryQuery {
        question: question.clone(),
        max_results: Some(5),
        parameters: None,
    };

    // Try to process the chat query using the application layer
    match state
        .application
        .query_repository(&permission_context, &repository_id, repo_query)
        .await
    {
        Ok(repo_response) => {
            // Convert application response to WebSocket message
            let sources: Vec<SourceDocument> = repo_response
                .sources
                .into_iter()
                .map(|source_path| SourceDocument {
                    file_path: source_path.clone(),
                    content: format!("Source: {}", source_path), // TODO: Get actual content
                    similarity_score: 0.8, // TODO: Get actual similarity score
                    start_line: None,      // TODO: Get actual line information
                    end_line: None,        // TODO: Get actual line information
                    chunk_index: None,     // TODO: Get actual chunk information
                    metadata: None,        // TODO: Get actual metadata
                })
                .collect();

            let response = create_message_with_id(|id| WsMessage::ChatResponse {
                repository_id: repository_id.clone(),
                answer: repo_response.answer,
                sources,
                timestamp: chrono::Utc::now(),
                id: Some(id),
                is_streaming: Some(false),
                is_complete: Some(true),
                chunk_id: None,
            });

            send_message(socket, response).await?;
        }
        Err(e) => {
            error!("Failed to process chat query: {}", e);
            send_error_response(
                socket,
                repository_id,
                format!("Failed to process chat query: {}", e),
                ErrorType::Chat,
            )
            .await?;
        }
    }

    Ok(())
}

/// Handle wiki generation requests
async fn handle_wiki_request(
    socket: &mut WebSocket,
    state: &AppState,
    repository_id: String,
    _config: WikiConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Processing wiki generation request for repository: {}",
        repository_id
    );

    // Create a local permission context for system operations
    let permission_context = wikify_applications::PermissionContext::local();

    // Get repository information first
    let repository = match state
        .application
        .get_repository(&permission_context, &repository_id)
        .await
    {
        Ok(repo) => repo,
        Err(e) => {
            error!("Failed to get repository info: {}", e);
            send_error_response(
                socket,
                repository_id,
                format!("Repository not found: {}", e),
                ErrorType::Wiki,
            )
            .await?;
            return Ok(());
        }
    };

    // Send wiki generation started message
    let start_msg = create_message_with_id(|id| WsMessage::WikiProgress {
        repository_id: repository_id.clone(),
        progress: 0.1,
        current_step: "Initializing wiki generation...".to_string(),
        total_steps: 3,
        completed_steps: 0,
        step_details: Some("Setting up wiki generation environment".to_string()),
        timestamp: chrono::Utc::now(),
        id: Some(id),
    });
    send_message(socket, start_msg).await?;

    // Generate wiki using wiki service
    let mut wiki_service = state.wiki_service.write().await;
    let wiki_config = wikify_wiki::WikiConfig::default();

    match wiki_service
        .generate_wiki(&repository.url, &wiki_config)
        .await
    {
        Ok(wiki_structure) => {
            // Send completion message
            let complete_msg = create_message_with_id(|id| WsMessage::WikiComplete {
                repository_id: repository_id.clone(),
                wiki_id: format!("wiki-{}", chrono::Utc::now().timestamp()),
                pages_count: wiki_structure.pages.len(),
                sections_count: wiki_structure.pages.len(), // Use pages count as sections count for now
                metadata: Some(WikiMetadata {
                    generation_time: 2.0, // TODO: Track actual generation time
                    total_tokens: 1500,   // TODO: Track actual token usage
                    model_used: "default".to_string(),
                }),
                timestamp: chrono::Utc::now(),
                id: Some(id),
            });
            send_message(socket, complete_msg).await?;
        }
        Err(e) => {
            error!("Failed to generate wiki: {}", e);
            send_error_response(
                socket,
                repository_id,
                format!("Wiki generation failed: {}", e),
                ErrorType::Wiki,
            )
            .await?;
        }
    }

    Ok(())
}

/// Send a general error message to all connected clients
pub async fn broadcast_error(
    state: &AppState,
    message: String,
    code: Option<String>,
    details: Option<serde_json::Value>,
) {
    let error_message = create_message_with_id(|id| WsMessage::Error {
        message,
        code,
        details,
        timestamp: chrono::Utc::now(),
        id: Some(id),
    });

    broadcast_message(state, error_message).await;
}

/// Send a message to all connected clients via broadcast channel
pub async fn broadcast_message(state: &AppState, message: WsMessage) {
    let broadcast_message = crate::state::BroadcastMessage::WebSocketMessage { message };

    if let Err(e) = state.progress_broadcaster.send(broadcast_message) {
        tracing::warn!("Failed to broadcast WebSocket message: {}", e);
    }
}
