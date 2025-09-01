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
    /// General error message
    Error {
        message: String,
        code: String,
        context: WsErrorContext,
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    /// Ping/Pong for connection health
    Ping {
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
    Pong {
        timestamp: chrono::DateTime<chrono::Utc>,
        id: Option<String>,
    },
}

/// WebSocket source document
#[derive(Debug, Serialize, Deserialize)]
pub struct WsSourceDocument {
    pub file_path: String,
    pub content: String,
    pub similarity_score: f64,
}

/// WebSocket wiki configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct WsWikiConfig {
    pub language: Option<String>,
    pub max_pages: Option<usize>,
    pub include_diagrams: Option<bool>,
    pub comprehensive_view: Option<bool>,
}

/// Wiki metadata for completion messages
#[derive(Debug, Serialize, Deserialize)]
pub struct WsWikiMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Error context for better error handling
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsErrorContext {
    Chat {
        repository_id: String,
    },
    Wiki {
        repository_id: String,
        step: Option<String>,
    },
    Index {
        repository_id: String,
        file_path: Option<String>,
    },
    System,
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

/// Global WebSocket handler for unified progress updates
pub async fn global_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_global_socket(socket, state))
}

/// Handle chat WebSocket connection
async fn handle_chat_socket(mut socket: WebSocket, state: AppState) {
    info!("New chat WebSocket connection established");

    // Send welcome message
    let welcome = WsMessage::ChatResponse {
        repository_id: "system".to_string(),
        answer: "Welcome to Wikify! Please initialize a repository first.".to_string(),
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
    info!("Subscribed to progress broadcaster, waiting for updates...");

    loop {
        tokio::select! {
            // Receive progress updates from the broadcaster
            update_result = progress_receiver.recv() => {
                match update_result {
                    Ok(update) => {
                        info!("Received progress update in WebSocket handler: {:?}", update);
                        // Convert to WebSocket message format based on update type
                        let ws_message = match update {
                            crate::state::IndexingUpdate::Progress {
                                repository_id,
                                percentage,
                                files_processed,
                                total_files,
                                current_item,
                                ..
                            } => {
                                WsMessage::IndexProgress {
                                    repository_id,
                                    progress: percentage / 100.0, // Convert to 0.0-1.0 range
                                    files_processed: files_processed.unwrap_or(0),
                                    total_files: total_files.unwrap_or(0),
                                    current_file: current_item,
                                    processing_rate: None,
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::Complete {
                                repository_id,
                                total_files,
                                total_chunks,
                                ..
                            } => {
                                // Send a completion message
                                WsMessage::IndexProgress {
                                    repository_id,
                                    progress: 1.0,
                                    files_processed: total_files,
                                    total_files,
                                    current_file: Some(format!("Completed! Processed {} files into {} chunks", total_files, total_chunks)),
                                    processing_rate: None,
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::Error { repository_id, error } => {
                                WsMessage::IndexError {
                                    repository_id,
                                    error,
                                    details: None,
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationStarted { repository_id } => {
                                WsMessage::WikiProgress {
                                    repository_id,
                                    progress: 0.0,
                                    current_step: "Starting wiki generation...".to_string(),
                                    total_steps: 5,
                                    completed_steps: 0,
                                    step_details: None,
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationProgress { repository_id, stage, percentage } => {
                                WsMessage::WikiProgress {
                                    repository_id,
                                    progress: percentage / 100.0,
                                    current_step: stage,
                                    total_steps: 5,
                                    completed_steps: (percentage / 20.0) as usize, // 5 steps, so each is 20%
                                    step_details: None,
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationComplete { repository_id, wiki_content } => {
                                WsMessage::WikiComplete {
                                    repository_id: repository_id.clone(),
                                    wiki_id: repository_id, // Use repository_id as wiki_id for now
                                    pages_count: 1, // Placeholder
                                    sections_count: wiki_content.matches('#').count(),
                                    metadata: Some(WsWikiMetadata {
                                        title: Some("Generated Wiki".to_string()),
                                        description: Some("Auto-generated wiki documentation".to_string()),
                                        generated_at: chrono::Utc::now(),
                                    }),
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationError { repository_id, error } => {
                                WsMessage::WikiError {
                                    repository_id,
                                    error: format!("Wiki generation failed: {}", error),
                                    details: None,
                                    timestamp: chrono::Utc::now(),
                                    id: None,
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
            repository_id,
            question,
            context: _,
            ..
        } => {
            // Update repository activity (no longer session-based)
            // Note: Repository activity tracking can be implemented if needed

            // Execute RAG query using repository_id
            let response = match state.query_rag(&repository_id, &question).await {
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
                        repository_id: repository_id.clone(),
                        answer: rag_response.answer,
                        sources,
                        timestamp: chrono::Utc::now(),
                        id: None,
                        is_streaming: Some(false),
                        is_complete: Some(true),
                        chunk_id: None,
                    }
                }
                Err(e) => {
                    // Return error response
                    WsMessage::ChatError {
                        repository_id: repository_id.clone(),
                        error: format!("Sorry, I encountered an error: {}", e),
                        details: None,
                        timestamp: chrono::Utc::now(),
                        id: None,
                    }
                }
            };

            let response_text = serde_json::to_string(&response)?;
            socket.send(Message::Text(response_text.into())).await?;
        }
        WsMessage::Ping { .. } => {
            let pong = WsMessage::Pong {
                timestamp: chrono::Utc::now(),
                id: None,
            };
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
            repository_id,
            config: _,
            ..
        } => {
            // Get repository info (replacing session lookup)
            let _repository = match state.get_repository(&repository_id).await {
                Some(repository) => repository,
                None => {
                    let error = WsMessage::Error {
                        message: "Repository not found".to_string(),
                        code: "REPOSITORY_NOT_FOUND".to_string(),
                        context: WsErrorContext::Wiki {
                            repository_id: repository_id.clone(),
                            step: None,
                        },
                        timestamp: chrono::Utc::now(),
                        id: None,
                    };
                    let error_text = serde_json::to_string(&error)?;
                    socket.send(Message::Text(error_text.into())).await?;
                    return Ok(());
                }
            };

            // Send progress updates
            for i in 1..=5 {
                let progress = WsMessage::WikiProgress {
                    repository_id: repository_id.clone(),
                    progress: (i as f64) / 5.0,
                    current_step: format!("Step {}: Processing...", i),
                    total_steps: 5,
                    completed_steps: i - 1,
                    step_details: Some(format!("Processing step {} of 5", i)),
                    timestamp: chrono::Utc::now(),
                    id: None,
                };

                let progress_text = serde_json::to_string(&progress)?;
                socket.send(Message::Text(progress_text.into())).await?;

                // Simulate work
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }

            // Send completion
            let complete = WsMessage::WikiComplete {
                repository_id: repository_id.clone(),
                wiki_id: uuid::Uuid::new_v4().to_string(),
                pages_count: 4,
                sections_count: 2,
                metadata: Some(WsWikiMetadata {
                    title: Some("Generated Wiki".to_string()),
                    description: Some("Auto-generated wiki documentation".to_string()),
                    generated_at: chrono::Utc::now(),
                }),
                timestamp: chrono::Utc::now(),
                id: None,
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

/// Handle global WebSocket connection for unified progress updates
async fn handle_global_socket(mut socket: WebSocket, state: AppState) {
    info!("New global WebSocket connection established");

    // Send welcome message
    let welcome = WsMessage::ChatResponse {
        repository_id: "system".to_string(),
        answer: "Global WebSocket connection established. You will receive progress updates for all operations.".to_string(),
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

    // Subscribe to progress updates from the state broadcaster
    let mut progress_receiver = state.subscribe_to_progress().await;

    // Handle incoming messages and broadcast progress updates
    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            msg_result = socket.recv() => {
                match msg_result {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_global_message(&mut socket, &state, &text).await {
                            error!("Error handling global message: {}", e);
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Global WebSocket connection closed by client");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("Global WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("Global WebSocket connection terminated");
                        break;
                    }
                    _ => {}
                }
            }

            // Receive progress updates from the broadcaster
            update_result = progress_receiver.recv() => {
                match update_result {
                    Ok(update) => {
                        info!("Received progress update in global WebSocket handler: {:?}", update);
                        // Convert to WebSocket message format based on update type
                        let ws_message = match update {
                            crate::state::IndexingUpdate::Progress {
                                repository_id,
                                percentage,
                                files_processed,
                                total_files,
                                current_item,
                                ..
                            } => {
                                WsMessage::IndexProgress {
                                    repository_id,
                                    progress: percentage / 100.0, // Convert to 0.0-1.0 range
                                    files_processed: files_processed.unwrap_or(0),
                                    total_files: total_files.unwrap_or(0),
                                    current_file: current_item,
                                    processing_rate: None,
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::Complete {
                                repository_id,
                                total_files,
                                ..
                            } => {
                                WsMessage::IndexComplete {
                                    repository_id,
                                    total_files,
                                    processing_time: None,
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::Error {
                                repository_id,
                                error,
                                ..
                            } => {
                                WsMessage::IndexError {
                                    repository_id,
                                    error,
                                    details: None,
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationStarted {
                                repository_id,
                            } => {
                                WsMessage::WikiProgress {
                                    repository_id,
                                    progress: 0.0,
                                    current_step: "Starting wiki generation".to_string(),
                                    total_steps: 5,
                                    completed_steps: 0,
                                    step_details: Some("Initializing wiki generation process".to_string()),
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationProgress {
                                repository_id,
                                stage,
                                percentage,
                            } => {
                                WsMessage::WikiProgress {
                                    repository_id,
                                    progress: percentage / 100.0,
                                    current_step: stage,
                                    total_steps: 5,
                                    completed_steps: ((percentage / 100.0) * 5.0) as usize,
                                    step_details: Some(format!("Progress: {:.1}%", percentage)),
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationComplete {
                                repository_id,
                                wiki_content: _,
                            } => {
                                WsMessage::WikiComplete {
                                    repository_id,
                                    wiki_id: uuid::Uuid::new_v4().to_string(),
                                    pages_count: 1,
                                    sections_count: 1,
                                    metadata: Some(WsWikiMetadata {
                                        title: Some("Generated Wiki".to_string()),
                                        description: Some("Auto-generated documentation".to_string()),
                                        generated_at: chrono::Utc::now(),
                                    }),
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                            crate::state::IndexingUpdate::WikiGenerationError {
                                repository_id,
                                error,
                            } => {
                                WsMessage::WikiError {
                                    repository_id,
                                    error,
                                    details: None,
                                    timestamp: chrono::Utc::now(),
                                    id: None,
                                }
                            }
                        };

                        // Send the progress update to the client
                        if let Ok(message_text) = serde_json::to_string(&ws_message) {
                            if socket.send(Message::Text(message_text.into())).await.is_err() {
                                error!("Failed to send progress update to global WebSocket client");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error receiving progress update: {}", e);
                        break;
                    }
                }
            }
        }
    }

    info!("Global WebSocket connection terminated");
}

/// Handle individual global messages (ping/pong, etc.)
async fn handle_global_message(
    socket: &mut WebSocket,
    _state: &AppState,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message: WsMessage = serde_json::from_str(text)?;

    match message {
        WsMessage::Ping { .. } => {
            let pong = WsMessage::Pong {
                timestamp: chrono::Utc::now(),
                id: None,
            };
            let pong_text = serde_json::to_string(&pong)?;
            socket.send(Message::Text(pong_text.into())).await?;
        }
        _ => {
            warn!("Unexpected message type in global handler: {:?}", message);
        }
    }

    Ok(())
}
