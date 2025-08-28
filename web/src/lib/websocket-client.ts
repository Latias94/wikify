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
} from '@/types/websocket';

// ============================================================================
// 配置和常量
// ============================================================================

const DEFAULT_CONFIG: Required<WebSocketConfig> = {
  url: import.meta.env.VITE_WS_BASE_URL || 'ws://localhost:8080/ws',
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

  constructor(
    endpoint: string,
    config: Partial<WebSocketConfig> = {},
    options: Partial<WebSocketOptions> = {}
  ) {
    this.config = {
      ...DEFAULT_CONFIG,
      ...config,
      url: `${DEFAULT_CONFIG.url}/${endpoint}`,
    };
    
    this.options = { ...DEFAULT_OPTIONS, ...options };
    
    this.state = {
      status: 'disconnected',
      reconnectAttempts: 0,
    };

    this.log('WebSocket client created', { endpoint, config: this.config });
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
        reject(new Error('Connection already in progress'));
        return;
      }

      this.isConnecting = true;
      this.updateState({ status: 'connecting' });
      this.log('Connecting to WebSocket server...');

      try {
        this.ws = new WebSocket(this.config.url);
        this.setupEventListeners(resolve, reject);
      } catch (error) {
        this.isConnecting = false;
        this.updateState({ status: 'error', error: (error as Error).message });
        reject(error);
      }
    });
  }

  /**
   * 断开连接
   */
  disconnect(): void {
    this.log('Disconnecting from WebSocket server...');
    
    // 清理定时器
    this.clearTimers();
    
    // 关闭连接
    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }
    
    this.updateState({ status: 'disconnected' });
    this.isConnecting = false;
    this.reconnectAttempts = 0;
  }

  /**
   * 设置事件监听器
   */
  private setupEventListeners(resolve: () => void, reject: (error: Error) => void): void {
    if (!this.ws) return;

    const connectTimeout = setTimeout(() => {
      reject(new Error('Connection timeout'));
      this.handleConnectionError();
    }, this.config.timeout);

    this.ws.onopen = () => {
      clearTimeout(connectTimeout);
      this.isConnecting = false;
      this.reconnectAttempts = 0;
      
      this.updateState({ 
        status: 'connected', 
        lastConnected: new Date(),
        error: undefined 
      });
      
      this.log('WebSocket connected successfully');
      
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
      
      this.log('WebSocket connection closed', { code: event.code, reason: event.reason });
      
      this.clearTimers();
      this.updateState({ status: 'disconnected' });
      
      this.handlers.onDisconnect?.();
      
      // 自动重连
      if (this.options.autoReconnect && event.code !== 1000) {
        this.scheduleReconnect();
      }
    };

    this.ws.onerror = (event) => {
      clearTimeout(connectTimeout);
      this.log('WebSocket error occurred', event);
      
      const error = new Error('WebSocket connection error');
      this.updateState({ status: 'error', error: error.message });
      
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
      this.log('Message sent', message);
    } else {
      // 连接未就绪，加入队列
      this.messageQueue.push(message);
      this.log('Message queued (connection not ready)', message);
      
      // 尝试连接
      if (this.state.status === 'disconnected') {
        this.connect().catch(error => {
          this.log('Failed to connect for queued message', error);
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
      this.log('Message received', message);
      
      // 处理心跳响应
      if (message.type === 'Pong') {
        this.lastPongTime = Date.now();
        return;
      }
      
      // 路由消息到对应的处理器
      this.routeMessage(message);
    } catch (error) {
      this.log('Failed to parse message', { data, error });
    }
  }

  /**
   * 路由消息到对应的处理器
   */
  private routeMessage(message: ServerMessage): void {
    switch (message.type) {
      case 'ChatResponse':
        this.handlers.onChatResponse?.(message);
        break;
      case 'ChatError':
        this.handlers.onChatError?.(message);
        break;
      case 'WikiProgress':
        this.handlers.onWikiProgress?.(message);
        break;
      case 'WikiComplete':
        this.handlers.onWikiComplete?.(message);
        break;
      case 'WikiError':
        this.handlers.onWikiError?.(message);
        break;
      case 'IndexProgress':
        this.handlers.onIndexProgress?.(message);
        break;
      case 'IndexComplete':
        this.handlers.onIndexComplete?.(message);
        break;
      case 'IndexError':
        this.handlers.onIndexError?.(message);
        break;
      default:
        this.log('Unknown message type', message);
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
    this.updateState({ status: 'error' });
    
    if (this.options.autoReconnect) {
      this.scheduleReconnect();
    }
  }

  /**
   * 安排重连
   */
  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
      this.log('Max reconnect attempts reached, giving up');
      return;
    }

    this.reconnectAttempts++;
    this.updateState({ reconnectAttempts: this.reconnectAttempts });
    
    const delay = Math.min(
      this.config.reconnectInterval * Math.pow(2, this.reconnectAttempts - 1),
      30000 // 最大 30 秒
    );
    
    this.log(`Scheduling reconnect attempt ${this.reconnectAttempts} in ${delay}ms`);
    
    this.reconnectTimer = setTimeout(() => {
      this.log(`Reconnect attempt ${this.reconnectAttempts}`);
      this.connect().catch(error => {
        this.log('Reconnect failed', error);
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
          type: 'Ping',
          timestamp: new Date().toISOString(),
        };
        
        this.send(pingMessage);
        
        // 检查上次 pong 的时间
        const now = Date.now();
        if (this.lastPongTime > 0 && now - this.lastPongTime > this.config.heartbeatInterval * 2) {
          this.log('Heartbeat timeout, reconnecting...');
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
      console.log(`[WebSocket] ${message}`, data || '');
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
}
