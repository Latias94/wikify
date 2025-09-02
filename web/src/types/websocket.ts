/**
 * WebSocket 消息类型定义
 * 与后端 WebSocket 协议保持一致
 */

import { SourceDocument } from "./api";

// Import unified types to match backend
interface WikiConfig {
  include_code_examples: boolean; // required
  max_depth: number; // usize -> number (note: potential precision loss for very large values)
  language?: string; // optional
}

interface WikiMetadata {
  generation_time: number; // in seconds (f64 -> number)
  total_tokens: number; // usize -> number (note: potential precision loss for very large values)
  model_used: string;
}

// ============================================================================
// WebSocket 连接状态
// ============================================================================

export type WebSocketStatus =
  | "connecting"
  | "connected"
  | "disconnected"
  | "error";

export interface WebSocketState {
  status: WebSocketStatus;
  error?: string;
  lastConnected?: Date;
  reconnectAttempts: number;
}

// ============================================================================
// 注意：现在 WebSocket 和 REST API 使用统一的 SourceDocument 类型
// ============================================================================

// ============================================================================
// 基础消息类型
// ============================================================================

export type WebSocketMessageType =
  | "Chat"
  | "ChatResponse"
  | "ChatError"
  | "WikiGenerate"
  | "WikiProgress"
  | "WikiComplete"
  | "WikiError"
  | "IndexStart"
  | "IndexProgress"
  | "IndexComplete"
  | "IndexError"
  | "ResearchStart"
  | "ResearchProgress"
  | "ResearchComplete"
  | "ResearchError"
  | "Error"
  | "Ping"
  | "Pong";

/**
 * WebSocket 消息基础结构
 */
export interface BaseWebSocketMessage {
  type: WebSocketMessageType;
  timestamp: string;
  id?: string;
}

// ============================================================================
// 聊天相关消息
// ============================================================================

/**
 * 聊天请求消息
 */
export interface ChatMessage extends BaseWebSocketMessage {
  type: "Chat";
  repository_id: string;
  question: string;
  context?: string;
}

/**
 * 聊天响应消息
 */
export interface ChatResponseMessage extends BaseWebSocketMessage {
  type: "ChatResponse";
  repository_id: string;
  answer: string;
  sources: SourceDocument[]; // 现在使用统一的 SourceDocument 类型
  is_streaming?: boolean;
  is_complete?: boolean;
  chunk_id?: string;
}

/**
 * 聊天错误消息
 */
export interface ChatErrorMessage extends BaseWebSocketMessage {
  type: "ChatError";
  repository_id: string;
  error: string;
  details?: Record<string, any>;
}

// ============================================================================
// Wiki 生成相关消息
// ============================================================================

/**
 * Wiki 生成请求消息
 */
export interface WikiGenerateMessage extends BaseWebSocketMessage {
  type: "WikiGenerate";
  repository_id: string;
  config: WikiConfig; // Use unified WikiConfig type
}

/**
 * Wiki 生成进度消息
 */
export interface WikiProgressMessage extends BaseWebSocketMessage {
  type: "WikiProgress";
  repository_id: string;
  progress: number; // 0.0-1.0 range
  current_step: string;
  total_steps: number;
  completed_steps: number;
  step_details?: string;
}

/**
 * Wiki 生成完成消息
 */
export interface WikiCompleteMessage extends BaseWebSocketMessage {
  type: "WikiComplete";
  repository_id: string;
  wiki_id: string;
  pages_count: number;
  sections_count: number;
  metadata?: WikiMetadata; // Use unified WikiMetadata type
}

/**
 * Wiki 生成错误消息
 */
export interface WikiErrorMessage extends BaseWebSocketMessage {
  type: "WikiError";
  repository_id: string;
  error: string;
  details?: Record<string, any>;
}

// ============================================================================
// 索引相关消息
// ============================================================================

/**
 * 索引开始消息
 */
export interface IndexStartMessage extends BaseWebSocketMessage {
  type: "IndexStart";
  repository_id: string;
  total_files?: number;
  estimated_duration?: number; // in seconds (to match AsyncAPI)
}

/**
 * 索引进度消息
 */
export interface IndexProgressMessage extends BaseWebSocketMessage {
  type: "IndexProgress";
  repository_id: string;
  progress: number; // 0.0-1.0 range
  current_file?: string;
  files_processed: number;
  total_files: number;
  processing_rate?: number;
}

/**
 * 索引完成消息
 */
export interface IndexCompleteMessage extends BaseWebSocketMessage {
  type: "IndexComplete";
  repository_id: string;
  total_files: number;
  processing_time?: number; // in seconds (to match AsyncAPI)
}

/**
 * 索引错误消息
 */
export interface IndexErrorMessage extends BaseWebSocketMessage {
  type: "IndexError";
  repository_id: string;
  error: string;
  details?: Record<string, any>;
}

// ============================================================================
// 研究相关消息
// ============================================================================

/**
 * 研究开始消息
 */
export interface ResearchStartMessage extends BaseWebSocketMessage {
  type: "ResearchStart";
  repository_id: string;
  research_id: string;
  query: string;
  total_iterations: number;
}

/**
 * 研究进度消息
 */
export interface ResearchProgressMessage extends BaseWebSocketMessage {
  type: "ResearchProgress";
  repository_id: string;
  research_id: string;
  current_iteration: number;
  total_iterations: number;
  current_focus: string;
  progress: number; // 0.0-1.0 range
  findings: string[];
}

/**
 * 研究完成消息
 */
export interface ResearchCompleteMessage extends BaseWebSocketMessage {
  type: "ResearchComplete";
  repository_id: string;
  research_id: string;
  total_iterations: number;
  final_conclusion: string;
  all_findings: string[];
  processing_time?: number; // in seconds (to match AsyncAPI)
}

/**
 * 研究错误消息
 */
export interface ResearchErrorMessage extends BaseWebSocketMessage {
  type: "ResearchError";
  repository_id: string;
  research_id: string;
  error: string;
  details?: Record<string, any>;
}

/**
 * 通用错误消息
 */
export interface ErrorMessage extends BaseWebSocketMessage {
  type: "Error";
  message: string;
  code?: string; // Optional to match AsyncAPI
  details?: Record<string, any>; // Changed from context to details
}

// ============================================================================
// 心跳消息
// ============================================================================

/**
 * Ping 消息
 */
export interface PingMessage extends BaseWebSocketMessage {
  type: "Ping";
}

/**
 * Pong 消息
 */
export interface PongMessage extends BaseWebSocketMessage {
  type: "Pong";
}

// ============================================================================
// 联合类型
// ============================================================================

/**
 * 所有 WebSocket 消息类型的联合
 */
export type WebSocketMessage =
  | ChatMessage
  | ChatResponseMessage
  | ChatErrorMessage
  | WikiGenerateMessage
  | WikiProgressMessage
  | WikiCompleteMessage
  | WikiErrorMessage
  | IndexStartMessage
  | IndexProgressMessage
  | IndexCompleteMessage
  | IndexErrorMessage
  | ResearchStartMessage
  | ResearchProgressMessage
  | ResearchCompleteMessage
  | ResearchErrorMessage
  | ErrorMessage
  | PingMessage
  | PongMessage;

/**
 * 客户端发送的消息类型
 */
export type ClientMessage = ChatMessage | WikiGenerateMessage | PingMessage;

/**
 * 服务端发送的消息类型
 */
export type ServerMessage =
  | ChatResponseMessage
  | ChatErrorMessage
  | WikiProgressMessage
  | WikiCompleteMessage
  | WikiErrorMessage
  | IndexStartMessage
  | IndexProgressMessage
  | IndexCompleteMessage
  | IndexErrorMessage
  | ResearchStartMessage
  | ResearchProgressMessage
  | ResearchCompleteMessage
  | ResearchErrorMessage
  | ErrorMessage
  | PongMessage;

// ============================================================================
// WebSocket 事件处理器类型
// ============================================================================

export interface WebSocketEventHandlers {
  onChat?: (message: ChatMessage) => void;
  onChatResponse?: (message: ChatResponseMessage) => void;
  onChatError?: (message: ChatErrorMessage) => void;
  onWikiProgress?: (message: WikiProgressMessage) => void;
  onWikiComplete?: (message: WikiCompleteMessage) => void;
  onWikiError?: (message: WikiErrorMessage) => void;
  onIndexStart?: (message: IndexStartMessage) => void;
  onIndexProgress?: (message: IndexProgressMessage) => void;
  onIndexComplete?: (message: IndexCompleteMessage) => void;
  onIndexError?: (message: IndexErrorMessage) => void;
  onResearchStart?: (message: ResearchStartMessage) => void;
  onResearchProgress?: (message: ResearchProgressMessage) => void;
  onResearchComplete?: (message: ResearchCompleteMessage) => void;
  onResearchError?: (message: ResearchErrorMessage) => void;
  onGeneralError?: (message: ErrorMessage) => void;
  onConnect?: () => void;
  onDisconnect?: () => void;
  onError?: (error: Event) => void;
  onMessage?: (message: any) => void;
}

// ============================================================================
// WebSocket 配置
// ============================================================================

export interface WebSocketConfig {
  url: string;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  heartbeatInterval?: number;
  timeout?: number;
}

/**
 * WebSocket 连接选项
 */
export interface WebSocketOptions {
  autoReconnect?: boolean;
  heartbeat?: boolean;
  debug?: boolean;
}
