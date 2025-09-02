import { useEffect, useRef, useState } from "react";
import { WebSocketClient } from "@/lib/websocket-client";
import { useProgressIntegration } from "@/hooks/use-progress-integration";
import { useAppStore } from "@/store/app-store";
import { useToast } from "@/hooks/use-toast";

// ============================================================================
// 全局 WebSocket 连接管理器
// ============================================================================

interface WebSocketManagerState {
  isConnected: boolean;
  connectionCount: number;
  lastError?: string;
}

// 全局消息去重集合 - 简化版本
const processedMessages = new Set<string>();

/**
 * 统一 WebSocket 连接管理器
 *
 * 使用后端的统一WebSocket端点，处理所有类型的实时通信
 * 包括聊天、Wiki生成、索引进度等
 */
export function useWebSocketManager() {
  const wsRef = useRef<WebSocketClient | null>(null);
  const [state, setState] = useState<WebSocketManagerState>({
    isConnected: false,
    connectionCount: 0,
  });

  const { toast } = useToast();
  const { updateRepository } = useAppStore();
  const {
    handleIndexProgress,
    handleIndexError,
    handleWikiProgress,
    handleWikiComplete,
    handleWikiError,
  } = useProgressIntegration();

  // 初始化全局 WebSocket 连接
  useEffect(() => {
    if (wsRef.current) return;

    const ws = new WebSocketClient(
      "unified",
      {
        reconnectInterval: 3000,
        maxReconnectAttempts: 10,
        heartbeatInterval: 30000,
      },
      {
        debug: import.meta.env.DEV,
      }
    );

    // 设置事件处理器
    ws.setHandlers({
      onConnect: () => {
        console.log("Global WebSocket connected");
        setState((prev) => ({
          ...prev,
          isConnected: true,
          lastError: undefined,
        }));
      },

      onDisconnect: () => {
        console.log("Global WebSocket disconnected");
        setState((prev) => ({ ...prev, isConnected: false }));
      },

      onError: (event) => {
        console.error("Global WebSocket error:", event);
        const errorMessage = "WebSocket connection error";
        setState((prev) => ({ ...prev, lastError: errorMessage }));
      },

      // 简化的消息处理 - 后端已经处理了去重
      onMessage: (message: any) => {
        try {
          console.log("WebSocket message received:", message.type, message);

          switch (message.type) {
            // 索引相关消息
            case "IndexStart":
              console.log(
                `Index started for repository: ${message.repository_id}`
              );
              updateRepository(message.repository_id, {
                status: "indexing",
              });
              break;

            case "IndexProgress":
            case "index_progress":
              handleIndexProgress(message);
              break;

            case "IndexComplete":
              console.log(
                `Index completed for repository: ${message.repository_id}`
              );
              updateRepository(message.repository_id, {
                status: "indexed",
                last_indexed_at: new Date().toISOString(),
              });
              break;

            case "IndexError":
              console.log(
                `Index error for repository: ${message.repository_id}`
              );
              handleIndexError(message);
              updateRepository(message.repository_id, {
                status: "failed",
              });
              break;

            // Wiki 生成消息
            case "WikiProgress":
              handleWikiProgress(message);
              updateRepository(message.repository_id, {
                wiki_status: "generating",
              });
              break;

            case "WikiComplete":
              handleWikiComplete(message);
              updateRepository(message.repository_id, {
                wiki_status: "generated",
                wiki_generated_at: new Date().toISOString(),
              });
              toast({
                title: "Wiki Generated",
                description:
                  "Wiki has been successfully generated and is ready to view.",
              });
              break;

            case "WikiError":
              handleWikiError(message);
              updateRepository(message.repository_id, {
                wiki_status: "failed",
              });
              toast({
                title: "Wiki Generation Failed",
                description: message.error,
                variant: "destructive",
              });
              break;

            // Research 相关消息
            case "ResearchStart":
              console.log(
                `Research started for repository: ${message.repository_id}, research_id: ${message.research_id}`
              );
              // 可以在这里更新研究状态或触发UI更新
              break;

            case "ResearchProgress":
              console.log(
                `Research progress for repository: ${message.repository_id}, iteration: ${message.current_iteration}/${message.total_iterations}`
              );
              // 可以在这里更新研究进度
              break;

            case "ResearchComplete":
              console.log(
                `Research completed for repository: ${message.repository_id}, research_id: ${message.research_id}`
              );
              // 可以在这里处理研究完成事件
              break;

            case "ResearchError":
              console.log(
                `Research error for repository: ${message.repository_id}, research_id: ${message.research_id}: ${message.error}`
              );
              // 可以在这里处理研究错误
              break;

            // 兼容旧的研究消息格式
            case "research_progress":
            case "research_complete":
            case "research_error":
              // 这些消息由具体的研究组件处理
              break;

            default:
              console.log("Unknown message type:", message.type);
          }
        } catch (error) {
          console.error("Failed to handle WebSocket message:", error);
        }
      },
    });

    wsRef.current = ws;

    // 连接到 WebSocket
    ws.connect().catch((error) => {
      console.error("Failed to connect to global WebSocket:", error);
      setState((prev) => ({
        ...prev,
        lastError: "Failed to connect to server",
      }));
    });

    return () => {
      ws.disconnect();
      wsRef.current = null;
    };
  }, [
    handleIndexProgress,
    handleIndexError,
    handleWikiProgress,
    handleWikiComplete,
    handleWikiError,
    updateRepository,
    toast,
  ]);

  // 连接计数管理
  const addConnection = () => {
    setState((prev) => ({
      ...prev,
      connectionCount: prev.connectionCount + 1,
    }));
  };

  const removeConnection = () => {
    setState((prev) => ({
      ...prev,
      connectionCount: Math.max(0, prev.connectionCount - 1),
    }));
  };

  // 手动重连
  const reconnect = () => {
    if (wsRef.current) {
      wsRef.current.connect().catch((error) => {
        console.error("Failed to reconnect to global WebSocket:", error);
        setState((prev) => ({
          ...prev,
          lastError: "Failed to reconnect to server",
        }));
      });
    }
  };

  return {
    isConnected: state.isConnected,
    connectionCount: state.connectionCount,
    lastError: state.lastError,
    addConnection,
    removeConnection,
    reconnect,
    // 获取原始 WebSocket 客户端（用于发送消息）
    getClient: () => wsRef.current,
  };
}

/**
 * 简化的 hook，用于组件订阅全局 WebSocket
 */
export function useGlobalWebSocket() {
  const manager = useWebSocketManager();

  useEffect(() => {
    manager.addConnection();
    return () => {
      manager.removeConnection();
    };
  }, [manager]);

  return {
    isConnected: manager.isConnected,
    lastError: manager.lastError,
    reconnect: manager.reconnect,
  };
}
