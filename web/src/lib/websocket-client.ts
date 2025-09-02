/**
 * WebSocket 客户端
 * 支持自动重连、心跳检测和消息路由
 */

import {
  WebSocketConfig,
  WebSocketOptions,
  WebSocketState,
  WebSocketStatus,
  WebSocketMessage,
  ClientMessage,
  ServerMessage,
  WebSocketEventHandlers,
  PingMessage,
  PongMessage,
} from "@/types/websocket";

// ============================================================================
// 配置和常量
// ============================================================================

const DEFAULT_CONFIG: Required<WebSocketConfig> = {
  url: import.meta.env.VITE_WS_BASE_URL || "ws://localhost:8080/ws",
  reconnectInterval: 3000,
  maxReconnectAttempts: 5,
  heartbeatInterval: 30000,
  timeout: 10000,
};

const DEFAULT_OPTIONS: Required<WebSocketOptions> = {
  autoReconnect: true,
  heartbeat: true,
  debug: false,
};

// ============================================================================
// WebSocket 客户端类
// ============================================================================

export class WebSocketClient {
  private ws: WebSocket | null = null;
  private config: Required<WebSocketConfig>;
  private options: Required<WebSocketOptions>;
  private state: WebSocketState;
  private handlers: WebSocketEventHandlers = {};

  // 重连相关
  private reconnectTimer: NodeJS.Timeout | null = null;
  private reconnectAttempts = 0;

  // 心跳相关
  private heartbeatTimer: NodeJS.Timeout | null = null;
  private lastPongTime = 0;

  // 消息队列
  private messageQueue: ClientMessage[] = [];
  private isConnecting = false;

  // 消息去重和追踪
  private receivedMessageIds = new Set<string>();
  private sentMessageIds = new Map<
    string,
    { timestamp: number; type: string }
  >();
  private readonly MESSAGE_ID_CLEANUP_INTERVAL = 300000; // 5分钟
  private readonly MAX_STORED_IDS = 1000;
  private cleanupTimer: NodeJS.Timeout | null = null;

  constructor(
    endpoint: string,
    config: Partial<WebSocketConfig> = {},
    options: Partial<WebSocketOptions> = {}
  ) {
    // All endpoints now use the unified WebSocket handler
    this.config = {
      ...DEFAULT_CONFIG,
      ...config,
      url: DEFAULT_CONFIG.url, // Use unified endpoint only
    };

    this.options = { ...DEFAULT_OPTIONS, ...options };

    this.state = {
      status: "disconnected",
      reconnectAttempts: 0,
    };

    // 启动消息ID清理定时器
    this.startMessageIdCleanup();

    this.log("WebSocket client created", { endpoint, config: this.config });
  }

  // ============================================================================
  // 连接管理
  // ============================================================================

  /**
   * 连接到 WebSocket 服务器
   */
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        resolve();
        return;
      }

      if (this.isConnecting) {
        reject(new Error("Connection already in progress"));
        return;
      }

      this.isConnecting = true;
      this.updateState({ status: "connecting" });
      this.log("Connecting to WebSocket server...");

      try {
        this.ws = new WebSocket(this.config.url);
        this.setupEventListeners(resolve, reject);
      } catch (error) {
        this.isConnecting = false;
        this.updateState({ status: "error", error: (error as Error).message });
        reject(error);
      }
    });
  }

  /**
   * 断开连接
   */
  disconnect(): void {
    this.log("Disconnecting from WebSocket server...");

    // 清理定时器
    this.clearTimers();

    // 关闭连接
    if (this.ws) {
      this.ws.close(1000, "Client disconnect");
      this.ws = null;
    }

    this.updateState({ status: "disconnected" });
    this.isConnecting = false;
    this.reconnectAttempts = 0;
  }

  /**
   * 设置事件监听器
   */
  private setupEventListeners(
    resolve: () => void,
    reject: (error: Error) => void
  ): void {
    if (!this.ws) return;

    const connectTimeout = setTimeout(() => {
      reject(new Error("Connection timeout"));
      this.handleConnectionError();
    }, this.config.timeout);

    this.ws.onopen = () => {
      clearTimeout(connectTimeout);
      this.isConnecting = false;
      this.reconnectAttempts = 0;

      this.updateState({
        status: "connected",
        lastConnected: new Date(),
        error: undefined,
      });

      this.log("WebSocket connected successfully");

      // 发送队列中的消息
      this.flushMessageQueue();

      // 启动心跳
      if (this.options.heartbeat) {
        this.startHeartbeat();
      }

      this.handlers.onConnect?.();
      resolve();
    };

    this.ws.onclose = (event) => {
      clearTimeout(connectTimeout);
      this.isConnecting = false;

      this.log("WebSocket connection closed", {
        code: event.code,
        reason: event.reason,
      });

      this.clearTimers();
      this.updateState({ status: "disconnected" });

      this.handlers.onDisconnect?.();

      // 自动重连
      if (this.options.autoReconnect && event.code !== 1000) {
        this.scheduleReconnect();
      }
    };

    this.ws.onerror = (event) => {
      clearTimeout(connectTimeout);
      this.log("WebSocket error occurred", event);

      const error = new Error("WebSocket connection error");
      this.updateState({ status: "error", error: error.message });

      this.handlers.onError?.(event);

      if (this.isConnecting) {
        reject(error);
      }
    };

    this.ws.onmessage = (event) => {
      this.handleMessage(event.data);
    };
  }

  // ============================================================================
  // 消息处理
  // ============================================================================

  /**
   * 发送消息
   */
  send(message: ClientMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      const messageStr = JSON.stringify({
        ...message,
        timestamp: new Date().toISOString(),
      });

      this.ws.send(messageStr);
      this.log("Message sent", message);
    } else {
      // 连接未就绪，加入队列
      this.messageQueue.push(message);
      this.log("Message queued (connection not ready)", message);

      // 尝试连接
      if (this.state.status === "disconnected") {
        this.connect().catch((error) => {
          this.log("Failed to connect for queued message", error);
        });
      }
    }
  }

  /**
   * 处理接收到的消息
   */
  private handleMessage(data: string): void {
    try {
      const message: ServerMessage = JSON.parse(data);
      this.log("Message received", message);

      // 处理心跳响应
      if (message.type === "Pong") {
        this.lastPongTime = Date.now();
        return;
      }

      // 首先调用通用消息处理器（如果存在）
      this.handlers.onMessage?.(message);

      // 然后路由消息到对应的处理器
      this.routeMessage(message);
    } catch (error) {
      this.log("Failed to parse message", { data, error });
    }
  }

  /**
   * 路由消息到对应的处理器
   */
  private routeMessage(message: ServerMessage): void {
    // 消息去重检查
    if (message.id && this.receivedMessageIds.has(message.id)) {
      this.log("Duplicate message received, skipping", {
        id: message.id,
        type: message.type,
      });
      return;
    }

    // 记录消息ID
    if (message.id) {
      this.receivedMessageIds.add(message.id);
      this.log("Processing message", { id: message.id, type: message.type });
    }

    switch (message.type) {
      case "Chat":
        this.handlers.onChat?.(message);
        break;
      case "ChatResponse":
        this.handlers.onChatResponse?.(message);
        break;
      case "ChatError":
        this.handlers.onChatError?.(message);
        break;
      case "WikiProgress":
        this.handlers.onWikiProgress?.(message);
        break;
      case "WikiComplete":
        this.handlers.onWikiComplete?.(message);
        break;
      case "WikiError":
        this.handlers.onWikiError?.(message);
        break;
      case "IndexStart":
        this.handlers.onIndexStart?.(message);
        break;
      case "IndexProgress":
        this.handlers.onIndexProgress?.(message);
        break;
      case "IndexComplete":
        this.handlers.onIndexComplete?.(message);
        break;
      case "IndexError":
        this.handlers.onIndexError?.(message);
        break;
      case "ResearchStart":
        this.handlers.onResearchStart?.(message);
        break;
      case "ResearchProgress":
        this.handlers.onResearchProgress?.(message);
        break;
      case "ResearchComplete":
        this.handlers.onResearchComplete?.(message);
        break;
      case "ResearchError":
        this.handlers.onResearchError?.(message);
        break;
      case "Error":
        this.handlers.onGeneralError?.(message);
        break;
      default:
        this.log("Unknown message type", message);
    }
  }

  /**
   * 发送队列中的消息
   */
  private flushMessageQueue(): void {
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift();
      if (message) {
        this.send(message);
      }
    }
  }

  // ============================================================================
  // 重连逻辑
  // ============================================================================

  /**
   * 处理连接错误
   */
  private handleConnectionError(): void {
    this.isConnecting = false;
    this.updateState({ status: "error" });

    if (this.options.autoReconnect) {
      this.scheduleReconnect();
    }
  }

  /**
   * 安排重连
   */
  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
      this.log("Max reconnect attempts reached, giving up");
      return;
    }

    this.reconnectAttempts++;
    this.updateState({ reconnectAttempts: this.reconnectAttempts });

    const delay = Math.min(
      this.config.reconnectInterval * Math.pow(2, this.reconnectAttempts - 1),
      30000 // 最大 30 秒
    );

    this.log(
      `Scheduling reconnect attempt ${this.reconnectAttempts} in ${delay}ms`
    );

    this.reconnectTimer = setTimeout(() => {
      this.log(`Reconnect attempt ${this.reconnectAttempts}`);
      this.connect().catch((error) => {
        this.log("Reconnect failed", error);
      });
    }, delay);
  }

  // ============================================================================
  // 心跳检测
  // ============================================================================

  /**
   * 启动心跳检测
   */
  private startHeartbeat(): void {
    this.heartbeatTimer = setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        const pingMessage: PingMessage = {
          type: "Ping",
          timestamp: new Date().toISOString(),
        };

        this.send(pingMessage);

        // 检查上次 pong 的时间
        const now = Date.now();
        if (
          this.lastPongTime > 0 &&
          now - this.lastPongTime > this.config.heartbeatInterval * 2
        ) {
          this.log("Heartbeat timeout, reconnecting...");
          this.handleConnectionError();
        }
      }
    }, this.config.heartbeatInterval);
  }

  // ============================================================================
  // 工具方法
  // ============================================================================

  /**
   * 清理定时器
   */
  private clearTimers(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  /**
   * 更新状态
   */
  private updateState(updates: Partial<WebSocketState>): void {
    this.state = { ...this.state, ...updates };
  }

  /**
   * 日志输出
   */
  private log(message: string, data?: any): void {
    if (this.options.debug) {
      console.log(`[WebSocket] ${message}`, data || "");
    }
  }

  // ============================================================================
  // 公共 API
  // ============================================================================

  /**
   * 获取当前状态
   */
  getState(): WebSocketState {
    return { ...this.state };
  }

  /**
   * 设置事件处理器
   */
  setHandlers(handlers: Partial<WebSocketEventHandlers>): void {
    this.handlers = { ...this.handlers, ...handlers };
  }

  /**
   * 检查是否已连接
   */
  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  /**
   * 获取连接状态
   */
  getReadyState(): number | undefined {
    return this.ws?.readyState;
  }

  // ============================================================================
  // 便捷方法 - 发送特定类型的消息
  // ============================================================================

  /**
   * 发送聊天消息
   */
  sendChatMessage(
    repositoryId: string,
    question: string,
    context?: string
  ): string {
    const messageId = this.generateMessageId();
    const message = {
      type: "Chat" as const,
      repository_id: repositoryId,
      question,
      context,
      timestamp: new Date().toISOString(),
      id: messageId,
    };

    // 追踪发送的消息
    this.sentMessageIds.set(messageId, {
      timestamp: Date.now(),
      type: "Chat",
    });

    this.send(message);
    return messageId; // 返回消息ID供调用者追踪
  }

  /**
   * 发送Wiki生成请求
   */
  sendWikiGenerateRequest(
    repositoryId: string,
    config: {
      include_code_examples?: boolean;
      max_depth?: number;
      language?: string;
    } = {}
  ): string {
    const messageId = this.generateMessageId();
    const message = {
      type: "WikiGenerate" as const,
      repository_id: repositoryId,
      config: {
        include_code_examples: config.include_code_examples ?? true,
        max_depth: config.max_depth ?? 3,
        language: config.language,
      },
      timestamp: new Date().toISOString(),
      id: messageId,
    };

    // 追踪发送的消息
    this.sentMessageIds.set(messageId, {
      timestamp: Date.now(),
      type: "WikiGenerate",
    });

    this.send(message);
    return messageId; // 返回消息ID供调用者追踪
  }

  /**
   * 生成唯一消息ID
   */
  private generateMessageId(): string {
    return `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  // ============================================================================
  // 消息ID管理
  // ============================================================================

  /**
   * 启动消息ID清理定时器
   */
  private startMessageIdCleanup(): void {
    this.cleanupTimer = setInterval(() => {
      this.cleanupMessageIds();
    }, this.MESSAGE_ID_CLEANUP_INTERVAL);
  }

  /**
   * 清理过期的消息ID
   */
  private cleanupMessageIds(): void {
    const now = Date.now();
    const expiredThreshold = now - this.MESSAGE_ID_CLEANUP_INTERVAL;

    // 清理接收到的消息ID（保持最近的1000个）
    if (this.receivedMessageIds.size > this.MAX_STORED_IDS) {
      const idsArray = Array.from(this.receivedMessageIds);
      const toKeep = idsArray.slice(-this.MAX_STORED_IDS / 2); // 保留一半
      this.receivedMessageIds.clear();
      toKeep.forEach((id) => this.receivedMessageIds.add(id));
      this.log("Cleaned up received message IDs", {
        before: idsArray.length,
        after: this.receivedMessageIds.size,
      });
    }

    // 清理发送的消息ID（基于时间）
    let cleanedSentCount = 0;
    for (const [id, info] of this.sentMessageIds.entries()) {
      if (info.timestamp < expiredThreshold) {
        this.sentMessageIds.delete(id);
        cleanedSentCount++;
      }
    }

    if (cleanedSentCount > 0) {
      this.log("Cleaned up sent message IDs", {
        cleaned: cleanedSentCount,
        remaining: this.sentMessageIds.size,
      });
    }
  }

  /**
   * 检查消息是否已发送
   */
  public isMessageSent(messageId: string): boolean {
    return this.sentMessageIds.has(messageId);
  }

  /**
   * 获取发送消息的信息
   */
  public getSentMessageInfo(
    messageId: string
  ): { timestamp: number; type: string } | null {
    return this.sentMessageIds.get(messageId) || null;
  }

  /**
   * 获取消息统计信息
   */
  public getMessageStats(): {
    receivedIds: number;
    sentIds: number;
    oldestSentTimestamp: number | null;
  } {
    let oldestSentTimestamp: number | null = null;
    for (const info of this.sentMessageIds.values()) {
      if (
        oldestSentTimestamp === null ||
        info.timestamp < oldestSentTimestamp
      ) {
        oldestSentTimestamp = info.timestamp;
      }
    }

    return {
      receivedIds: this.receivedMessageIds.size,
      sentIds: this.sentMessageIds.size,
      oldestSentTimestamp,
    };
  }

  /**
   * 清理资源
   */
  public destroy(): void {
    if (this.cleanupTimer) {
      clearInterval(this.cleanupTimer);
      this.cleanupTimer = null;
    }

    this.receivedMessageIds.clear();
    this.sentMessageIds.clear();

    this.disconnect();
  }
}
