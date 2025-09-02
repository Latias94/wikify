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

/// WebSocket message types
#[derive(Debug, Serialize, Deserialize)]
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
        sources: Vec<WsSourceDocument>,
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
        config: WsWikiConfig,
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
        metadata: Option<WsWikiMetadata>,
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
    #[serde(rename = "index_progress")]
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WsSourceDocument {
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub score: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WsWikiConfig {
    pub include_code_examples: bool,
    pub max_depth: usize,
    pub language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WsWikiMetadata {
    pub generation_time: f64,
    pub total_tokens: usize,
    pub model_used: String,
}

/// Unified WebSocket handler for all real-time communication
pub async fn unified_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_unified_socket(socket, state))
}

// Legacy handlers for backward compatibility - all redirect to unified handler
pub async fn chat_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_unified_socket(socket, state))
}

pub async fn wiki_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_unified_socket(socket, state))
}

pub async fn index_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_unified_socket(socket, state))
}

pub async fn global_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_unified_socket(socket, state))
}

/// Unified WebSocket connection handler
/// Handles all types of WebSocket communication: chat, wiki, indexing, and progress updates
async fn handle_unified_socket(mut socket: WebSocket, state: AppState) {
    info!("New unified WebSocket connection established");

    // Send welcome message
    let welcome = WsMessage::ChatResponse {
        repository_id: "system".to_string(),
        answer: "WebSocket connection established. You will receive real-time updates for all operations.".to_string(),
        sources: vec![],
        timestamp: chrono::Utc::now(),
        id: None,
        is_streaming: Some(false),
        is_complete: Some(true),
        chunk_id: None,
    };

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

            // Handle progress updates from broadcaster
            update_result = progress_receiver.recv() => {
                match update_result {
                    Ok(update) => {
                        // Create unique message ID to prevent duplicates
                        let message_id = match &update {
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
                            _ => format!("other-{}", chrono::Utc::now().timestamp_millis())
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

                        info!("Processing progress update: {:?}", update);

                        // Convert to WebSocket message and send
                        if let Some(ws_message) = convert_update_to_message(update) {
                            if let Ok(msg) = serde_json::to_string(&ws_message) {
                                if socket.send(Message::Text(msg.into())).await.is_err() {
                                    error!("Failed to send progress update, connection closed");
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
            if let Ok(msg) = serde_json::to_string(&pong) {
                socket.send(Message::Text(msg.into())).await?;
            }
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
            id: None,
        }),
        crate::state::IndexingUpdate::Progress {
            repository_id,
            percentage,
            files_processed,
            total_files,
            current_item,
            ..
        } => Some(WsMessage::IndexProgress {
            repository_id,
            progress: percentage, // percentage is already in 0.0-1.0 range
            files_processed: files_processed.unwrap_or(0),
            total_files: total_files.unwrap_or(0),
            current_file: current_item,
            processing_rate: None,
            timestamp: chrono::Utc::now(),
            id: None,
        }),
        crate::state::IndexingUpdate::Complete {
            repository_id,
            total_files,
            ..
        } => Some(WsMessage::IndexComplete {
            repository_id,
            total_files,
            processing_time: None,
            timestamp: chrono::Utc::now(),
            id: None,
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
        } => Some(WsMessage::WikiProgress {
            repository_id,
            progress: percentage,
            current_step: stage,
            total_steps: 100,
            completed_steps: (percentage * 100.0) as usize,
            step_details: None,
            timestamp: chrono::Utc::now(),
            id: None,
        }),
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
    context: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Processing chat request for repository: {}", repository_id);

    // TODO: Implement actual chat logic here
    // For now, send a placeholder response
    let response = WsMessage::ChatResponse {
        repository_id: repository_id.clone(),
        answer: format!(
            "Chat functionality not yet implemented for question: {}",
            question
        ),
        sources: vec![],
        timestamp: chrono::Utc::now(),
        id: None,
        is_streaming: Some(false),
        is_complete: Some(true),
        chunk_id: None,
    };

    if let Ok(msg) = serde_json::to_string(&response) {
        socket.send(Message::Text(msg.into())).await?;
    }

    Ok(())
}

/// Handle wiki generation requests
async fn handle_wiki_request(
    socket: &mut WebSocket,
    state: &AppState,
    repository_id: String,
    config: WsWikiConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Processing wiki generation request for repository: {}",
        repository_id
    );

    // TODO: Implement actual wiki generation logic here
    // For now, send a placeholder response
    let response = WsMessage::WikiComplete {
        repository_id: repository_id.clone(),
        wiki_id: "placeholder".to_string(),
        pages_count: 0,
        sections_count: 0,
        metadata: None,
        timestamp: chrono::Utc::now(),
        id: None,
    };

    if let Ok(msg) = serde_json::to_string(&response) {
        socket.send(Message::Text(msg.into())).await?;
    }

    Ok(())
}
