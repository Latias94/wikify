/**
 * WebSocket hooks
 * 封装 WebSocket 连接和消息处理的 React hooks
 */

import { useEffect, useRef, useCallback } from 'react';
import { WebSocketClient } from '@/lib/websocket-client';
import { useChatStore } from '@/store/chat-store';
import { useAppStore } from '@/store/app-store';
import { useToast } from '@/hooks/use-toast';
import {
  WebSocketConfig,
  WebSocketOptions,
  ClientMessage,
  ChatMessage,
  WikiGenerateMessage,
  ChatResponseMessage,
  ChatErrorMessage,
  WikiProgressMessage,
  WikiCompleteMessage,
  WikiErrorMessage,
  IndexProgressMessage,
  IndexCompleteMessage,
  IndexErrorMessage,
} from '@/types/websocket';
import { UIChatMessage } from '@/types/ui';

// ============================================================================
// 聊天 WebSocket Hook
// ============================================================================

export function useChatWebSocket(
  sessionId?: string,
  config?: Partial<WebSocketConfig>,
  options?: Partial<WebSocketOptions>
) {
  const wsRef = useRef<WebSocketClient | null>(null);
  const { toast } = useToast();
  
  // Store actions
  const {
    addMessage,
    startStreamingMessage,
    updateStreamingMessage,
    finishStreamingMessage,
    cancelStreamingMessage,
    setConnectionState,
    setError,
    clearError,
  } = useChatStore();
  
  const { updateRepository } = useAppStore();

  // 初始化 WebSocket 连接
  useEffect(() => {
    if (!sessionId) return;

    const ws = new WebSocketClient('chat', config, {
      debug: import.meta.env.DEV,
      ...options,
    });

    // 设置事件处理器
    ws.setHandlers({
      onConnect: () => {
        console.log('Chat WebSocket connected');
        setConnectionState({ status: 'connected', error: undefined });
        clearError(sessionId);
      },

      onDisconnect: () => {
        console.log('Chat WebSocket disconnected');
        setConnectionState({ status: 'disconnected' });
      },

      onError: (event) => {
        console.error('Chat WebSocket error:', event);
        setConnectionState({ status: 'error', error: 'Connection error' });
        setError(sessionId, 'Connection error occurred');
      },

      onChatResponse: (message: ChatResponseMessage) => {
        if (message.session_id !== sessionId) return;

        if (message.is_streaming) {
          // 流式响应
          if (message.is_complete) {
            finishStreamingMessage();
          } else {
            const existingMessage = useChatStore.getState().streamingMessage;
            if (existingMessage) {
              updateStreamingMessage(message.answer);
            } else {
              // 开始新的流式消息
              const uiMessage: UIChatMessage = {
                id: `ai-${Date.now()}`,
                role: 'assistant',
                content: message.answer,
                timestamp: new Date(message.timestamp),
                sources: message.sources,
                isStreaming: true,
              };
              startStreamingMessage(sessionId, uiMessage);
            }
          }
        } else {
          // 普通响应
          const uiMessage: UIChatMessage = {
            id: `ai-${Date.now()}`,
            role: 'assistant',
            content: message.answer,
            timestamp: new Date(message.timestamp),
            sources: message.sources,
          };
          addMessage(sessionId, uiMessage);
        }
      },

      onChatError: (message: ChatErrorMessage) => {
        if (message.session_id !== sessionId) return;

        console.error('Chat error:', message.error);
        setError(sessionId, message.error);
        
        // 取消流式消息（如果有）
        cancelStreamingMessage();
        
        toast({
          title: "Chat Error",
          description: message.error,
          variant: "destructive",
        });
      },

      onIndexProgress: (message: IndexProgressMessage) => {
        // 更新仓库索引进度
        updateRepository(message.repository_id, {
          status: 'indexing',
          // 注意：这里需要根据后端实际字段调整
        });
      },

      onIndexComplete: (message: IndexCompleteMessage) => {
        // 索引完成
        updateRepository(message.repository_id, {
          status: 'indexed',
          last_indexed_at: new Date().toISOString(),
        });
        
        toast({
          title: "Indexing Complete",
          description: `Repository has been indexed successfully. Processed ${message.total_files} files.`,
        });
      },

      onIndexError: (message: IndexErrorMessage) => {
        // 索引错误
        updateRepository(message.repository_id, {
          status: 'failed',
        });
        
        toast({
          title: "Indexing Failed",
          description: message.error,
          variant: "destructive",
        });
      },
    });

    wsRef.current = ws;

    // 连接到 WebSocket
    ws.connect().catch((error) => {
      console.error('Failed to connect to chat WebSocket:', error);
      setError(sessionId, 'Failed to connect to chat service');
    });

    return () => {
      ws.disconnect();
      wsRef.current = null;
    };
  }, [sessionId, config, options]);

  // 发送聊天消息
  const sendMessage = useCallback((question: string, context?: string) => {
    if (!wsRef.current || !sessionId) {
      console.error('WebSocket not connected or session not available');
      return;
    }

    // 添加用户消息到 UI
    const userMessage: UIChatMessage = {
      id: `user-${Date.now()}`,
      role: 'user',
      content: question,
      timestamp: new Date(),
    };
    addMessage(sessionId, userMessage);

    // 发送到服务器
    const chatMessage: ChatMessage = {
      type: 'Chat',
      session_id: sessionId,
      question,
      context,
      timestamp: new Date().toISOString(),
    };

    wsRef.current.send(chatMessage);
    clearError(sessionId);
  }, [sessionId, addMessage, clearError]);

  // 获取连接状态
  const getConnectionState = useCallback(() => {
    return wsRef.current?.getState() || { status: 'disconnected', reconnectAttempts: 0 };
  }, []);

  // 检查是否已连接
  const isConnected = useCallback(() => {
    return wsRef.current?.isConnected() || false;
  }, []);

  return {
    sendMessage,
    getConnectionState,
    isConnected,
    disconnect: () => wsRef.current?.disconnect(),
    reconnect: () => wsRef.current?.connect(),
  };
}

// ============================================================================
// Wiki WebSocket Hook
// ============================================================================

export function useWikiWebSocket(
  sessionId?: string,
  config?: Partial<WebSocketConfig>,
  options?: Partial<WebSocketOptions>
) {
  const wsRef = useRef<WebSocketClient | null>(null);
  const { toast } = useToast();

  // 初始化 WebSocket 连接
  useEffect(() => {
    if (!sessionId) return;

    const ws = new WebSocketClient('wiki', config, {
      debug: import.meta.env.DEV,
      ...options,
    });

    // 设置事件处理器
    ws.setHandlers({
      onConnect: () => {
        console.log('Wiki WebSocket connected');
      },

      onDisconnect: () => {
        console.log('Wiki WebSocket disconnected');
      },

      onError: (event) => {
        console.error('Wiki WebSocket error:', event);
      },

      onWikiProgress: (message: WikiProgressMessage) => {
        if (message.session_id !== sessionId) return;
        
        console.log(`Wiki generation progress: ${message.progress}% - ${message.current_step}`);
        
        // 这里可以触发进度更新的回调
        // 或者更新全局状态
      },

      onWikiComplete: (message: WikiCompleteMessage) => {
        if (message.session_id !== sessionId) return;
        
        console.log('Wiki generation completed:', message);
        
        toast({
          title: "Wiki Generated",
          description: `Wiki "${message.title}" has been generated successfully with ${message.pages_count} pages.`,
        });
      },

      onWikiError: (message: WikiErrorMessage) => {
        if (message.session_id !== sessionId) return;
        
        console.error('Wiki generation error:', message.error);
        
        toast({
          title: "Wiki Generation Failed",
          description: message.error,
          variant: "destructive",
        });
      },
    });

    wsRef.current = ws;

    // 连接到 WebSocket
    ws.connect().catch((error) => {
      console.error('Failed to connect to wiki WebSocket:', error);
    });

    return () => {
      ws.disconnect();
      wsRef.current = null;
    };
  }, [sessionId, config, options]);

  // 生成 Wiki
  const generateWiki = useCallback((title?: string, description?: string, sections?: string[]) => {
    if (!wsRef.current || !sessionId) {
      console.error('WebSocket not connected or session not available');
      return;
    }

    const wikiMessage: WikiGenerateMessage = {
      type: 'WikiGenerate',
      session_id: sessionId,
      title,
      description,
      sections,
      timestamp: new Date().toISOString(),
    };

    wsRef.current.send(wikiMessage);
  }, [sessionId]);

  return {
    generateWiki,
    isConnected: () => wsRef.current?.isConnected() || false,
    disconnect: () => wsRef.current?.disconnect(),
  };
}

// ============================================================================
// 通用 WebSocket Hook
// ============================================================================

export function useWebSocket(
  endpoint: string,
  config?: Partial<WebSocketConfig>,
  options?: Partial<WebSocketOptions>
) {
  const wsRef = useRef<WebSocketClient | null>(null);

  useEffect(() => {
    const ws = new WebSocketClient(endpoint, config, {
      debug: import.meta.env.DEV,
      ...options,
    });

    wsRef.current = ws;

    return () => {
      ws.disconnect();
      wsRef.current = null;
    };
  }, [endpoint]);

  const send = useCallback((message: ClientMessage) => {
    wsRef.current?.send(message);
  }, []);

  const connect = useCallback(() => {
    return wsRef.current?.connect();
  }, []);

  const disconnect = useCallback(() => {
    wsRef.current?.disconnect();
  }, []);

  const setHandlers = useCallback((handlers: Parameters<WebSocketClient['setHandlers']>[0]) => {
    wsRef.current?.setHandlers(handlers);
  }, []);

  return {
    send,
    connect,
    disconnect,
    setHandlers,
    getState: () => wsRef.current?.getState(),
    isConnected: () => wsRef.current?.isConnected() || false,
  };
}
