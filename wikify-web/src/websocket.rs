//! WebSocket handlers for real-time communication
//!
//! This module handles WebSocket connections for chat, wiki generation, and indexing progress.

use crate::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::time::Duration;
use tracing::{error, info, warn};

/// WebSocket message types
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Chat message
    Chat {
        session_id: String,
        question: String,
        context: Option<String>,
    },
    /// Chat response
    ChatResponse {
        session_id: String,
        answer: String,
        sources: Vec<WsSourceDocument>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Wiki generation request
    WikiGenerate {
        session_id: String,
        config: WsWikiConfig,
    },
    /// Wiki generation progress
    WikiProgress {
        session_id: String,
        progress: f64,
        current_step: String,
        total_steps: usize,
        completed_steps: usize,
    },
    /// Wiki generation complete
    WikiComplete {
        session_id: String,
        wiki_id: String,
        pages_count: usize,
        sections_count: usize,
    },
    /// Indexing progress
    IndexProgress {
        session_id: String,
        progress: f64,
        files_processed: usize,
        total_files: usize,
        current_file: Option<String>,
    },
    /// Error message
    Error {
        message: String,
        code: Option<String>,
    },
    /// Ping/Pong for connection health
    Ping,
    Pong,
}

/// WebSocket source document
#[derive(Serialize, Deserialize)]
pub struct WsSourceDocument {
    pub file_path: String,
    pub content: String,
    pub similarity_score: f64,
}

/// WebSocket wiki configuration
#[derive(Serialize, Deserialize)]
pub struct WsWikiConfig {
    pub language: Option<String>,
    pub max_pages: Option<usize>,
    pub include_diagrams: Option<bool>,
    pub comprehensive_view: Option<bool>,
}

/// Chat WebSocket handler
pub async fn chat_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_chat_socket(socket, state))
}

/// Wiki generation WebSocket handler
pub async fn wiki_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_wiki_socket(socket, state))
}

/// Repository indexing WebSocket handler
pub async fn index_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_index_socket(socket, state))
}

/// Handle chat WebSocket connection
async fn handle_chat_socket(mut socket: WebSocket, state: AppState) {
    info!("New chat WebSocket connection established");

    // Send welcome message
    let welcome = WsMessage::ChatResponse {
        session_id: "system".to_string(),
        answer: "Welcome to Wikify! Please initialize a repository first.".to_string(),
        sources: vec![],
        timestamp: chrono::Utc::now(),
    };

    if let Ok(msg) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(msg.into())).await.is_err() {
            return;
        }
    }

    // Handle incoming messages
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_chat_message(&mut socket, &state, &text).await {
                    error!("Error handling chat message: {}", e);
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                info!("Chat WebSocket connection closed");
                break;
            }
            Err(e) => {
                error!("Chat WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

/// Handle wiki generation WebSocket connection
async fn handle_wiki_socket(mut socket: WebSocket, state: AppState) {
    info!("New wiki WebSocket connection established");

    // Handle incoming messages
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_wiki_message(&mut socket, &state, &text).await {
                    error!("Error handling wiki message: {}", e);
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                info!("Wiki WebSocket connection closed");
                break;
            }
            Err(e) => {
                error!("Wiki WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

/// Handle indexing WebSocket connection
async fn handle_index_socket(mut socket: WebSocket, state: AppState) {
    info!("New indexing WebSocket connection established");

    // Subscribe to progress updates
    let mut progress_receiver = state.progress_broadcaster.subscribe();

    loop {
        tokio::select! {
            // Receive progress updates from the broadcaster
            update_result = progress_receiver.recv() => {
                match update_result {
                    Ok(update) => {
                        // Convert to WebSocket message format based on update type
                        let ws_message = match update {
                            crate::state::IndexingUpdate::Progress {
                                session_id,
                                percentage,
                                files_processed,
                                total_files,
                                current_item,
                                ..
                            } => {
                                WsMessage::IndexProgress {
                                    session_id,
                                    progress: percentage / 100.0, // Convert to 0.0-1.0 range
                                    files_processed: files_processed.unwrap_or(0),
                                    total_files: total_files.unwrap_or(0),
                                    current_file: current_item,
                                }
                            }
                            crate::state::IndexingUpdate::Complete {
                                session_id,
                                total_files,
                                total_chunks,
                                ..
                            } => {
                                // Send a completion message
                                WsMessage::IndexProgress {
                                    session_id,
                                    progress: 1.0,
                                    files_processed: total_files,
                                    total_files,
                                    current_file: Some(format!("Completed! Processed {} files into {} chunks", total_files, total_chunks)),
                                }
                            }
                            crate::state::IndexingUpdate::Error { session_id: _, error } => {
                                WsMessage::Error {
                                    message: error,
                                    code: Some("INDEXING_ERROR".to_string()),
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationStarted { session_id } => {
                                WsMessage::WikiProgress {
                                    session_id,
                                    progress: 0.0,
                                    current_step: "Starting wiki generation...".to_string(),
                                    total_steps: 5,
                                    completed_steps: 0,
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationProgress { session_id, stage, percentage } => {
                                WsMessage::WikiProgress {
                                    session_id,
                                    progress: percentage / 100.0,
                                    current_step: stage,
                                    total_steps: 5,
                                    completed_steps: (percentage / 20.0) as usize, // 5 steps, so each is 20%
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationComplete { session_id, wiki_content } => {
                                WsMessage::WikiComplete {
                                    session_id: session_id.clone(),
                                    wiki_id: session_id, // Use session_id as wiki_id for now
                                    pages_count: 1, // Placeholder
                                    sections_count: wiki_content.matches('#').count(),
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationError { session_id: _, error } => {
                                WsMessage::Error {
                                    message: format!("Wiki generation failed: {}", error),
                                    code: Some("WIKI_GENERATION_ERROR".to_string()),
                                }
                            }
                        };

                        if let Ok(msg) = serde_json::to_string(&ws_message) {
                            if socket.send(Message::Text(msg.into())).await.is_err() {
                                info!("Client disconnected during update");
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("WebSocket client lagged behind, skipped {} messages", skipped);
                        // Continue receiving
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Progress broadcaster closed");
                        break;
                    }
                }
            }
            // Handle incoming WebSocket messages
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) => {
                        info!("Indexing WebSocket connection closed by client");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        error!("Indexing WebSocket error: {}", e);
                        break;
                    }
                    _ => {
                        // Ignore other message types
                    }
                }
            }
        }
    }

    info!("Indexing WebSocket connection terminated");
}

/// Handle individual chat messages
async fn handle_chat_message(
    socket: &mut WebSocket,
    state: &AppState,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message: WsMessage = serde_json::from_str(text)?;

    match message {
        WsMessage::Chat {
            session_id,
            question,
            context: _,
        } => {
            // Update session activity
            if let Err(e) = state.update_session_activity(&session_id).await {
                warn!("Failed to update session activity: {}", e);
            }

            // Execute RAG query
            let response = match state.query_rag(&session_id, &question).await {
                Ok(rag_response) => {
                    // Convert RAG response to WebSocket response
                    let sources = rag_response
                        .sources
                        .into_iter()
                        .enumerate()
                        .map(|(i, source_content)| WsSourceDocument {
                            file_path: format!("source_{}", i), // TODO: Extract actual file path
                            content: source_content,
                            similarity_score: 1.0, // TODO: Get actual similarity score
                        })
                        .collect();

                    WsMessage::ChatResponse {
                        session_id: session_id.clone(),
                        answer: rag_response.answer,
                        sources,
                        timestamp: chrono::Utc::now(),
                    }
                }
                Err(e) => {
                    // Return error response
                    WsMessage::ChatResponse {
                        session_id: session_id.clone(),
                        answer: format!("Sorry, I encountered an error: {}", e),
                        sources: vec![],
                        timestamp: chrono::Utc::now(),
                    }
                }
            };

            let response_text = serde_json::to_string(&response)?;
            socket.send(Message::Text(response_text.into())).await?;
        }
        WsMessage::Ping => {
            let pong = WsMessage::Pong;
            let pong_text = serde_json::to_string(&pong)?;
            socket.send(Message::Text(pong_text.into())).await?;
        }
        _ => {
            warn!("Unexpected message type in chat handler");
        }
    }

    Ok(())
}

/// Handle individual wiki messages
async fn handle_wiki_message(
    socket: &mut WebSocket,
    state: &AppState,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message: WsMessage = serde_json::from_str(text)?;

    match message {
        WsMessage::WikiGenerate {
            session_id,
            config: _,
        } => {
            // Get session info
            let _session = match state.get_session(&session_id).await {
                Some(session) => session,
                None => {
                    let error = WsMessage::Error {
                        message: "Session not found".to_string(),
                        code: Some("SESSION_NOT_FOUND".to_string()),
                    };
                    let error_text = serde_json::to_string(&error)?;
                    socket.send(Message::Text(error_text.into())).await?;
                    return Ok(());
                }
            };

            // Send progress updates
            for i in 1..=5 {
                let progress = WsMessage::WikiProgress {
                    session_id: session_id.clone(),
                    progress: (i as f64) / 5.0,
                    current_step: format!("Step {}: Processing...", i),
                    total_steps: 5,
                    completed_steps: i - 1,
                };

                let progress_text = serde_json::to_string(&progress)?;
                socket.send(Message::Text(progress_text.into())).await?;

                // Simulate work
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }

            // Send completion
            let complete = WsMessage::WikiComplete {
                session_id: session_id.clone(),
                wiki_id: uuid::Uuid::new_v4().to_string(),
                pages_count: 4,
                sections_count: 2,
            };

            let complete_text = serde_json::to_string(&complete)?;
            socket.send(Message::Text(complete_text.into())).await?;
        }
        _ => {
            warn!("Unexpected message type in wiki handler");
        }
    }

    Ok(())
}
