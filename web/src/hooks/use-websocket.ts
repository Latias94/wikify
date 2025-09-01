/**
 * WebSocket hooks
 * 封装 WebSocket 连接和消息处理的 React hooks
 */

import { useEffect, useRef, useCallback } from "react";
import { WebSocketClient } from "@/lib/websocket-client";
import { useChatStore } from "@/store/chat-store";
import { useAppStore } from "@/store/app-store";
import { useToast } from "@/hooks/use-toast";
import { useProgressIntegration } from "@/hooks/use-progress-integration";
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
} from "@/types/websocket";
import { UIChatMessage } from "@/types/ui";

// ============================================================================
// 聊天 WebSocket Hook
// ============================================================================

export function useChatWebSocket(
  repositoryId?: string,
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
    if (!repositoryId) return;

    const ws = new WebSocketClient("chat", config, {
      debug: import.meta.env.DEV,
      ...options,
    });

    // 设置事件处理器
    ws.setHandlers({
      onConnect: () => {
        console.log("Chat WebSocket connected");
        setConnectionState({ status: "connected", error: undefined });
        clearError(repositoryId);
      },

      onDisconnect: () => {
        console.log("Chat WebSocket disconnected");
        setConnectionState({ status: "disconnected" });
      },

      onError: (event) => {
        console.error("Chat WebSocket error:", event);
        setConnectionState({ status: "error", error: "Connection error" });
        setError(repositoryId, "Connection error occurred");
      },

      onChatResponse: (message: ChatResponseMessage) => {
        if (message.repository_id !== repositoryId) return;

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
                role: "assistant",
                content: message.answer,
                timestamp: new Date(message.timestamp),
                sources: message.sources,
                isStreaming: true,
              };
              startStreamingMessage(repositoryId, uiMessage);
            }
          }
        } else {
          // 普通响应
          const uiMessage: UIChatMessage = {
            id: `ai-${Date.now()}`,
            role: "assistant",
            content: message.answer,
            timestamp: new Date(message.timestamp),
            sources: message.sources,
          };
          addMessage(repositoryId, uiMessage);
        }
      },

      onChatError: (message: ChatErrorMessage) => {
        if (message.repository_id !== repositoryId) return;

        console.error("Chat error:", message.error);
        setError(repositoryId, message.error);

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
          status: "indexing",
          indexing_progress: message.progress,
        });
      },

      onIndexComplete: (message: IndexCompleteMessage) => {
        // 索引完成
        updateRepository(message.repository_id, {
          status: "indexed",
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
          status: "failed",
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
      console.error("Failed to connect to chat WebSocket:", error);
      setError(repositoryId, "Failed to connect to chat service");
    });

    return () => {
      ws.disconnect();
      wsRef.current = null;
    };
  }, [repositoryId, config, options]);

  // 发送聊天消息
  const sendMessage = useCallback(
    (question: string, context?: string) => {
      if (!wsRef.current || !repositoryId) {
        console.error("WebSocket not connected or repository not available");
        return;
      }

      // 添加用户消息到 UI
      const userMessage: UIChatMessage = {
        id: `user-${Date.now()}`,
        role: "user",
        content: question,
        timestamp: new Date(),
      };
      addMessage(repositoryId, userMessage);

      // 发送到服务器
      const chatMessage: ChatMessage = {
        type: "Chat",
        repository_id: repositoryId,
        question,
        context,
        timestamp: new Date().toISOString(),
      };

      wsRef.current.send(chatMessage);
      clearError(repositoryId);
    },
    [repositoryId, addMessage, clearError]
  );

  // 获取连接状态
  const getConnectionState = useCallback(() => {
    return (
      wsRef.current?.getState() || {
        status: "disconnected",
        reconnectAttempts: 0,
      }
    );
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
// 索引进度 WebSocket Hook
// ============================================================================

export function useIndexProgressWebSocket(
  onProgress?: (progress: IndexProgressMessage) => void,
  onComplete?: (completion: IndexCompleteMessage) => void,
  onError?: (error: IndexErrorMessage) => void,
  config?: Partial<WebSocketConfig>,
  options?: Partial<WebSocketOptions>
) {
  const wsRef = useRef<WebSocketClient | null>(null);
  const { toast } = useToast();
  const { updateRepository } = useAppStore();
  const {
    handleIndexProgress,
    handleIndexError,
    handleWikiProgress,
    handleWikiComplete,
    handleWikiError,
  } = useProgressIntegration();

  // 初始化 WebSocket 连接
  useEffect(() => {
    const ws = new WebSocketClient("index", config, {
      debug: import.meta.env.DEV,
      ...options,
    });

    // 设置事件处理器
    ws.setHandlers({
      onConnect: () => {
        console.log("Index progress WebSocket connected");
      },

      onDisconnect: () => {
        console.log("Index progress WebSocket disconnected");
      },

      onError: (event) => {
        console.error("Index progress WebSocket error:", event);
      },

      onIndexProgress: (message: IndexProgressMessage) => {
        console.log(
          `Indexing progress: ${(message.progress * 100).toFixed(1)}%`
        );

        // 集成到统一进度系统
        handleIndexProgress(message);

        // 更新仓库状态
        updateRepository(message.repository_id, {
          status: "indexing",
          indexing_progress: message.progress,
        });

        // 检查是否完成
        if (message.progress >= 1.0) {
          // 这是完成消息
          const completionMessage: IndexCompleteMessage = {
            ...message,
            progress: 1.0,
          };
          onComplete?.(completionMessage);

          // 更新仓库状态为已索引
          updateRepository(message.repository_id, {
            status: "indexed",
            indexing_progress: 1.0,
            last_indexed_at: new Date().toISOString(),
          });

          toast({
            title: "Indexing Complete",
            description:
              message.current_file ||
              `Repository indexed successfully. Processed ${message.files_processed} files.`,
          });
        } else {
          // 这是进度更新
          onProgress?.(message);
        }
      },

      onIndexError: (message: IndexErrorMessage) => {
        console.error("Indexing error:", message.error);

        // 集成到统一进度系统
        handleIndexError(message);

        onError?.(message);

        toast({
          title: "Indexing Failed",
          description: message.error,
          variant: "destructive",
        });
      },

      // Wiki生成状态处理
      onWikiProgress: (message: WikiProgressMessage) => {
        console.log(
          `Wiki generation progress: ${(message.progress * 100).toFixed(
            1
          )}% - ${message.current_step}`
        );

        // 集成到统一进度系统
        handleWikiProgress(message);

        // 更新仓库的Wiki状态
        updateRepository(message.repository_id, {
          wiki_status: "generating",
        });
      },

      onWikiComplete: (message: WikiCompleteMessage) => {
        console.log(
          `Wiki generation completed for repository: ${message.repository_id}`
        );

        // 集成到统一进度系统
        handleWikiComplete(message);

        // 更新仓库的Wiki状态为已生成
        updateRepository(message.repository_id, {
          wiki_status: "generated",
          wiki_generated_at: new Date().toISOString(),
        });

        toast({
          title: "Wiki Generated",
          description:
            "Wiki has been successfully generated and is ready to view.",
        });
      },

      onWikiError: (message: WikiErrorMessage) => {
        console.error(
          `Wiki generation failed for repository: ${message.repository_id}`,
          message.error
        );

        // 集成到统一进度系统
        handleWikiError(message);

        // 更新仓库的Wiki状态为失败
        updateRepository(message.repository_id, {
          wiki_status: "failed",
        });

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
      console.error("Failed to connect to index progress WebSocket:", error);
    });

    return () => {
      ws.disconnect();
      wsRef.current = null;
    };
  }, [config, options, onProgress, onComplete, onError]);

  return {
    isConnected: () => wsRef.current?.isConnected() || false,
    disconnect: () => wsRef.current?.disconnect(),
    reconnect: () => wsRef.current?.connect(),
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

  const setHandlers = useCallback(
    (handlers: Parameters<WebSocketClient["setHandlers"]>[0]) => {
      wsRef.current?.setHandlers(handlers);
    },
    []
  );

  return {
    send,
    connect,
    disconnect,
    setHandlers,
    getState: () => wsRef.current?.getState(),
    isConnected: () => wsRef.current?.isConnected() || false,
  };
}

// ============================================================================
// 研究进度 WebSocket Hook
// ============================================================================

export function useResearchWebSocket(
  researchId?: string,
  onProgress?: (update: any) => void,
  onComplete?: (result: any) => void,
  onError?: (error: string) => void,
  config?: Partial<WebSocketConfig>,
  options?: Partial<WebSocketOptions>
) {
  const wsRef = useRef<WebSocketClient | null>(null);
  const { toast } = useToast();

  // 初始化 WebSocket 连接
  useEffect(() => {
    if (!researchId) return;

    const ws = new WebSocketClient("research", config, {
      debug: import.meta.env.DEV,
      ...options,
    });

    // 设置事件处理器
    ws.setHandlers({
      onConnect: () => {
        console.log("Research WebSocket connected");
      },

      onDisconnect: () => {
        console.log("Research WebSocket disconnected");
      },

      onError: (event) => {
        console.error("Research WebSocket error:", event);
        onError?.("WebSocket connection error");
      },

      // 研究进度消息处理
      onMessage: (message: any) => {
        try {
          if (message.type === "research_progress") {
            onProgress?.(message);
          } else if (message.type === "research_complete") {
            onComplete?.(message);
          } else if (message.type === "research_error") {
            onError?.(message.error);
          }
        } catch (error) {
          console.error("Failed to handle research message:", error);
          onError?.("Failed to process research update");
        }
      },
    });

    wsRef.current = ws;

    // 连接到 WebSocket
    ws.connect().catch((error) => {
      console.error("Failed to connect to research WebSocket:", error);
      onError?.("Failed to connect to research service");
    });

    return () => {
      ws.disconnect();
      wsRef.current = null;
    };
  }, [researchId, onProgress, onComplete, onError, config, options]);

  return {
    isConnected: () => wsRef.current?.isConnected() || false,
    disconnect: () => wsRef.current?.disconnect(),
  };
}
