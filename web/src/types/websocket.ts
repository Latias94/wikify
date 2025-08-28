/**
 * WebSocket 消息类型定义
 * 与后端 WebSocket 协议保持一致
 */

import { SourceDocument } from './api';

// ============================================================================
// WebSocket 连接状态
// ============================================================================

export type WebSocketStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

export interface WebSocketState {
  status: WebSocketStatus;
  error?: string;
  lastConnected?: Date;
  reconnectAttempts: number;
}

// ============================================================================
// 基础消息类型
// ============================================================================

export type WebSocketMessageType = 
  | 'Chat'
  | 'ChatResponse' 
  | 'ChatError'
  | 'WikiGenerate'
  | 'WikiProgress'
  | 'WikiComplete'
  | 'WikiError'
  | 'IndexProgress'
  | 'IndexComplete'
  | 'IndexError'
  | 'Ping'
  | 'Pong';

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
  type: 'Chat';
  session_id: string;
  question: string;
  context?: string;
}

/**
 * 聊天响应消息
 */
export interface ChatResponseMessage extends BaseWebSocketMessage {
  type: 'ChatResponse';
  session_id: string;
  answer: string;
  sources: SourceDocument[];
  is_streaming?: boolean;
  is_complete?: boolean;
}

/**
 * 聊天错误消息
 */
export interface ChatErrorMessage extends BaseWebSocketMessage {
  type: 'ChatError';
  session_id: string;
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
  type: 'WikiGenerate';
  session_id: string;
  title?: string;
  description?: string;
  sections?: string[];
}

/**
 * Wiki 生成进度消息
 */
export interface WikiProgressMessage extends BaseWebSocketMessage {
  type: 'WikiProgress';
  session_id: string;
  progress: number; // 0-100
  current_step: string;
  total_steps: number;
  current_step_index: number;
}

/**
 * Wiki 生成完成消息
 */
export interface WikiCompleteMessage extends BaseWebSocketMessage {
  type: 'WikiComplete';
  session_id: string;
  wiki_id: string;
  title: string;
  description: string;
  pages_count: number;
}

/**
 * Wiki 生成错误消息
 */
export interface WikiErrorMessage extends BaseWebSocketMessage {
  type: 'WikiError';
  session_id: string;
  error: string;
  details?: Record<string, any>;
}

// ============================================================================
// 索引相关消息
// ============================================================================

/**
 * 索引进度消息
 */
export interface IndexProgressMessage extends BaseWebSocketMessage {
  type: 'IndexProgress';
  repository_id: string;
  progress: number; // 0-100
  current_file?: string;
  processed_files: number;
  total_files: number;
  status: 'processing' | 'embedding' | 'storing';
}

/**
 * 索引完成消息
 */
export interface IndexCompleteMessage extends BaseWebSocketMessage {
  type: 'IndexComplete';
  repository_id: string;
  total_files: number;
  total_chunks: number;
  duration_ms: number;
}

/**
 * 索引错误消息
 */
export interface IndexErrorMessage extends BaseWebSocketMessage {
  type: 'IndexError';
  repository_id: string;
  error: string;
  details?: Record<string, any>;
}

// ============================================================================
// 心跳消息
// ============================================================================

/**
 * Ping 消息
 */
export interface PingMessage extends BaseWebSocketMessage {
  type: 'Ping';
}

/**
 * Pong 消息
 */
export interface PongMessage extends BaseWebSocketMessage {
  type: 'Pong';
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
  | IndexProgressMessage
  | IndexCompleteMessage
  | IndexErrorMessage
  | PingMessage
  | PongMessage;

/**
 * 客户端发送的消息类型
 */
export type ClientMessage = 
  | ChatMessage
  | WikiGenerateMessage
  | PingMessage;

/**
 * 服务端发送的消息类型
 */
export type ServerMessage = 
  | ChatResponseMessage
  | ChatErrorMessage
  | WikiProgressMessage
  | WikiCompleteMessage
  | WikiErrorMessage
  | IndexProgressMessage
  | IndexCompleteMessage
  | IndexErrorMessage
  | PongMessage;

// ============================================================================
// WebSocket 事件处理器类型
// ============================================================================

export interface WebSocketEventHandlers {
  onChatResponse?: (message: ChatResponseMessage) => void;
  onChatError?: (message: ChatErrorMessage) => void;
  onWikiProgress?: (message: WikiProgressMessage) => void;
  onWikiComplete?: (message: WikiCompleteMessage) => void;
  onWikiError?: (message: WikiErrorMessage) => void;
  onIndexProgress?: (message: IndexProgressMessage) => void;
  onIndexComplete?: (message: IndexCompleteMessage) => void;
  onIndexError?: (message: IndexErrorMessage) => void;
  onConnect?: () => void;
  onDisconnect?: () => void;
  onError?: (error: Event) => void;
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
