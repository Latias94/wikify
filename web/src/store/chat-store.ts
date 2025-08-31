/**
 * 聊天状态管理
 * 管理聊天消息、WebSocket 连接状态等
 */

import { create } from "zustand";
import { devtools } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";
import { UIChatMessage } from "@/types/ui";
import { WebSocketState } from "@/types/websocket";

// ============================================================================
// 状态接口定义
// ============================================================================

interface ChatState {
  // 消息相关
  messages: Record<string, UIChatMessage[]>; // repositoryId -> messages
  currentInput: string;
  isTyping: boolean;

  // WebSocket 状态
  connectionState: WebSocketState;

  // UI 状态
  showSources: boolean;
  showTimestamps: boolean;
  autoScroll: boolean;

  // 流式响应状态
  streamingMessage: UIChatMessage | null;

  // 错误状态
  errors: Record<string, string>; // repositoryId -> error
}

interface ChatActions {
  // 消息操作
  addMessage: (repositoryId: string, message: UIChatMessage) => void;
  updateMessage: (
    repositoryId: string,
    messageId: string,
    updates: Partial<UIChatMessage>
  ) => void;
  removeMessage: (repositoryId: string, messageId: string) => void;
  clearMessages: (repositoryId: string) => void;
  setMessages: (repositoryId: string, messages: UIChatMessage[]) => void;

  // 输入操作
  setCurrentInput: (input: string) => void;
  clearCurrentInput: () => void;

  // 流式响应操作
  startStreamingMessage: (repositoryId: string, message: UIChatMessage) => void;
  updateStreamingMessage: (content: string) => void;
  finishStreamingMessage: () => void;
  cancelStreamingMessage: () => void;

  // WebSocket 状态操作
  setConnectionState: (state: Partial<WebSocketState>) => void;

  // UI 状态操作
  setShowSources: (show: boolean) => void;
  setShowTimestamps: (show: boolean) => void;
  setAutoScroll: (autoScroll: boolean) => void;
  setIsTyping: (typing: boolean) => void;

  // 错误处理
  setError: (repositoryId: string, error: string | undefined) => void;
  clearError: (repositoryId: string) => void;
  clearAllErrors: () => void;

  // 重置操作
  reset: () => void;
  resetRepository: (repositoryId: string) => void;
}

type ChatStore = ChatState & ChatActions;

// ============================================================================
// 默认状态
// ============================================================================

const initialState: ChatState = {
  // 消息相关
  messages: {},
  currentInput: "",
  isTyping: false,

  // WebSocket 状态
  connectionState: {
    status: "disconnected",
    reconnectAttempts: 0,
  },

  // UI 状态
  showSources: true,
  showTimestamps: true,
  autoScroll: true,

  // 流式响应状态
  streamingMessage: null,

  // 错误状态
  errors: {},
};

// ============================================================================
// Store 创建
// ============================================================================

export const useChatStore = create<ChatStore>()(
  devtools(
    immer((set, get) => ({
      ...initialState,

      // ============================================================================
      // 消息操作
      // ============================================================================

      addMessage: (repositoryId, message) => {
        set((state) => {
          if (!state.messages[repositoryId]) {
            state.messages[repositoryId] = [];
          }
          state.messages[repositoryId].push(message);
        });
      },

      updateMessage: (repositoryId, messageId, updates) => {
        set((state) => {
          const messages = state.messages[repositoryId];
          if (messages) {
            const index = messages.findIndex((msg) => msg.id === messageId);
            if (index !== -1) {
              Object.assign(messages[index], updates);
            }
          }
        });
      },

      removeMessage: (repositoryId, messageId) => {
        set((state) => {
          const messages = state.messages[repositoryId];
          if (messages) {
            state.messages[repositoryId] = messages.filter(
              (msg) => msg.id !== messageId
            );
          }
        });
      },

      clearMessages: (repositoryId) => {
        set((state) => {
          state.messages[repositoryId] = [];
        });
      },

      setMessages: (repositoryId, messages) => {
        set((state) => {
          state.messages[repositoryId] = messages;
        });
      },

      // ============================================================================
      // 输入操作
      // ============================================================================

      setCurrentInput: (input) => {
        set((state) => {
          state.currentInput = input;
        });
      },

      clearCurrentInput: () => {
        set((state) => {
          state.currentInput = "";
        });
      },

      // ============================================================================
      // 流式响应操作
      // ============================================================================

      startStreamingMessage: (repositoryId, message) => {
        set((state) => {
          state.streamingMessage = { ...message, isStreaming: true };

          // 添加到消息列表
          if (!state.messages[repositoryId]) {
            state.messages[repositoryId] = [];
          }
          state.messages[repositoryId].push(state.streamingMessage);
        });
      },

      updateStreamingMessage: (content) => {
        set((state) => {
          if (state.streamingMessage) {
            state.streamingMessage.content = content;

            // 同时更新消息列表中的消息
            Object.keys(state.messages).forEach((sessionId) => {
              const messages = state.messages[sessionId];
              const index = messages.findIndex(
                (msg) => msg.id === state.streamingMessage?.id
              );
              if (index !== -1) {
                messages[index].content = content;
              }
            });
          }
        });
      },

      finishStreamingMessage: () => {
        set((state) => {
          if (state.streamingMessage) {
            // 更新消息列表中的消息状态
            Object.keys(state.messages).forEach((sessionId) => {
              const messages = state.messages[sessionId];
              const index = messages.findIndex(
                (msg) => msg.id === state.streamingMessage?.id
              );
              if (index !== -1) {
                messages[index].isStreaming = false;
              }
            });

            state.streamingMessage = null;
          }
        });
      },

      cancelStreamingMessage: () => {
        set((state) => {
          if (state.streamingMessage) {
            // 从消息列表中移除未完成的流式消息
            Object.keys(state.messages).forEach((sessionId) => {
              const messages = state.messages[sessionId];
              state.messages[sessionId] = messages.filter(
                (msg) => msg.id !== state.streamingMessage?.id
              );
            });

            state.streamingMessage = null;
          }
        });
      },

      // ============================================================================
      // WebSocket 状态操作
      // ============================================================================

      setConnectionState: (stateUpdates) => {
        set((state) => {
          Object.assign(state.connectionState, stateUpdates);
        });
      },

      // ============================================================================
      // UI 状态操作
      // ============================================================================

      setShowSources: (show) => {
        set((state) => {
          state.showSources = show;
        });
      },

      setShowTimestamps: (show) => {
        set((state) => {
          state.showTimestamps = show;
        });
      },

      setAutoScroll: (autoScroll) => {
        set((state) => {
          state.autoScroll = autoScroll;
        });
      },

      setIsTyping: (typing) => {
        set((state) => {
          state.isTyping = typing;
        });
      },

      // ============================================================================
      // 错误处理
      // ============================================================================

      setError: (repositoryId, error) => {
        set((state) => {
          if (error) {
            state.errors[repositoryId] = error;
          } else {
            delete state.errors[repositoryId];
          }
        });
      },

      clearError: (repositoryId) => {
        set((state) => {
          delete state.errors[repositoryId];
        });
      },

      clearAllErrors: () => {
        set((state) => {
          state.errors = {};
        });
      },

      // ============================================================================
      // 重置操作
      // ============================================================================

      reset: () => {
        set(initialState);
      },

      resetRepository: (repositoryId) => {
        set((state) => {
          delete state.messages[repositoryId];
          delete state.errors[repositoryId];
        });
      },
    })),
    {
      name: "wikify-chat-store",
    }
  )
);

// ============================================================================
// 选择器 Hooks
// ============================================================================

// 消息相关选择器
export const useMessages = (repositoryId: string) =>
  useChatStore((state) => state.messages[repositoryId] || []);

export const useCurrentInput = () =>
  useChatStore((state) => state.currentInput);
export const useIsTyping = () => useChatStore((state) => state.isTyping);

// WebSocket 状态选择器
export const useConnectionState = () =>
  useChatStore((state) => state.connectionState);
export const useIsConnected = () =>
  useChatStore((state) => state.connectionState.status === "connected");

// UI 状态选择器
export const useShowSources = () => useChatStore((state) => state.showSources);
export const useShowTimestamps = () =>
  useChatStore((state) => state.showTimestamps);
export const useAutoScroll = () => useChatStore((state) => state.autoScroll);

// 流式响应选择器
export const useStreamingMessage = () =>
  useChatStore((state) => state.streamingMessage);
export const useIsStreaming = () =>
  useChatStore((state) => !!state.streamingMessage);

// 错误选择器
export const useChatError = (repositoryId: string) =>
  useChatStore((state) => state.errors[repositoryId]);

export const useHasChatErrors = () =>
  useChatStore((state) => Object.keys(state.errors).length > 0);

// 组合选择器
export const useMessageCount = (sessionId: string) =>
  useChatStore((state) => state.messages[sessionId]?.length || 0);

export const useLastMessage = (sessionId: string) =>
  useChatStore((state) => {
    const messages = state.messages[sessionId];
    return messages && messages.length > 0
      ? messages[messages.length - 1]
      : null;
  });

export default useChatStore;
