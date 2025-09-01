/**
 * 进度状态管理
 * 统一管理所有类型的进度状态
 */

import { create } from "zustand";
import { devtools } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";
import { 
  ProgressState, 
  ProgressType, 
  ProgressStatus,
  ProgressStats,
  ProgressHistoryEntry,
  ProgressNotificationConfig
} from "@/types/progress";

// ============================================================================
// 状态接口定义
// ============================================================================

interface ProgressStoreState {
  // 当前进度状态
  progressStates: Record<string, ProgressState>;
  
  // 历史记录
  history: ProgressHistoryEntry[];
  
  // 通知配置
  notificationConfig: ProgressNotificationConfig;
  
  // 订阅者
  subscribers: Map<string, (states: ProgressState[]) => void>;
  progressSubscribers: Map<string, Map<string, (state: ProgressState) => void>>;
}

interface ProgressStoreActions {
  // 状态查询
  getProgress: (id: string) => ProgressState | undefined;
  getAllProgress: () => ProgressState[];
  getProgressByType: (type: ProgressType) => ProgressState[];
  getProgressByRepository: (repositoryId: string) => ProgressState[];
  getProgressStats: () => ProgressStats;
  
  // 进度操作
  startProgress: (state: Omit<ProgressState, "id" | "startTime">) => string;
  updateProgress: (id: string, updates: Partial<ProgressState>) => void;
  completeProgress: (id: string, result?: any) => void;
  errorProgress: (id: string, error: string) => void;
  cancelProgress: (id: string) => void;
  clearProgress: (id: string) => void;
  clearAllProgress: () => void;
  
  // 历史记录
  getHistory: () => ProgressHistoryEntry[];
  clearHistory: () => void;
  
  // 通知配置
  updateNotificationConfig: (config: Partial<ProgressNotificationConfig>) => void;
  
  // 事件订阅
  subscribe: (callback: (states: ProgressState[]) => void) => () => void;
  subscribeToProgress: (id: string, callback: (state: ProgressState) => void) => () => void;
  
  // 内部方法
  _notifySubscribers: () => void;
  _notifyProgressSubscribers: (id: string, state: ProgressState) => void;
  _addToHistory: (state: ProgressState) => void;
}

type ProgressStore = ProgressStoreState & ProgressStoreActions;

// ============================================================================
// 默认配置
// ============================================================================

const defaultNotificationConfig: ProgressNotificationConfig = {
  enabled: true,
  showStart: true,
  showProgress: false,
  showComplete: true,
  showError: true,
  sound: true,
  desktop: true,
  progressInterval: 30, // 30秒间隔
};

// ============================================================================
// Store 创建
// ============================================================================

export const useProgressStore = create<ProgressStore>()(
  devtools(
    immer((set, get) => ({
      // 初始状态
      progressStates: {},
      history: [],
      notificationConfig: defaultNotificationConfig,
      subscribers: new Map(),
      progressSubscribers: new Map(),

      // ============================================================================
      // 状态查询
      // ============================================================================

      getProgress: (id: string) => {
        return get().progressStates[id];
      },

      getAllProgress: () => {
        return Object.values(get().progressStates);
      },

      getProgressByType: (type: ProgressType) => {
        return Object.values(get().progressStates).filter(
          (state) => state.type === type
        );
      },

      getProgressByRepository: (repositoryId: string) => {
        return Object.values(get().progressStates).filter(
          (state) => 
            "repositoryId" in state && state.repositoryId === repositoryId
        );
      },

      getProgressStats: () => {
        const states = Object.values(get().progressStates);
        const stats: ProgressStats = {
          total: states.length,
          running: 0,
          completed: 0,
          failed: 0,
          cancelled: 0,
          byType: {
            indexing: 0,
            wiki_generation: 0,
            rag_query: 0,
            research: 0,
          },
          byRepository: {},
        };

        states.forEach((state) => {
          // 按状态统计
          switch (state.status) {
            case "running":
            case "connecting":
              stats.running++;
              break;
            case "completed":
              stats.completed++;
              break;
            case "error":
              stats.failed++;
              break;
            case "cancelled":
              stats.cancelled++;
              break;
          }

          // 按类型统计
          stats.byType[state.type]++;

          // 按仓库统计
          if ("repositoryId" in state) {
            const repoId = (state as any).repositoryId;
            stats.byRepository[repoId] = (stats.byRepository[repoId] || 0) + 1;
          }
        });

        return stats;
      },

      // ============================================================================
      // 进度操作
      // ============================================================================

      startProgress: (state) => {
        const id = `progress_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        const newState: ProgressState = {
          ...state,
          id,
          startTime: new Date(),
          status: "running",
        } as ProgressState;

        set((draft) => {
          draft.progressStates[id] = newState;
        });

        get()._notifySubscribers();
        get()._notifyProgressSubscribers(id, newState);

        return id;
      },

      updateProgress: (id, updates) => {
        const currentState = get().progressStates[id];
        if (!currentState) return;

        const updatedState = { ...currentState, ...updates };

        set((draft) => {
          draft.progressStates[id] = updatedState;
        });

        get()._notifySubscribers();
        get()._notifyProgressSubscribers(id, updatedState);
      },

      completeProgress: (id, result) => {
        const currentState = get().progressStates[id];
        if (!currentState) return;

        const completedState: ProgressState = {
          ...currentState,
          status: "completed",
          progress: 1.0,
          endTime: new Date(),
        };

        set((draft) => {
          draft.progressStates[id] = completedState;
        });

        get()._addToHistory(completedState);
        get()._notifySubscribers();
        get()._notifyProgressSubscribers(id, completedState);
      },

      errorProgress: (id, error) => {
        const currentState = get().progressStates[id];
        if (!currentState) return;

        const errorState: ProgressState = {
          ...currentState,
          status: "error",
          error,
          endTime: new Date(),
        };

        set((draft) => {
          draft.progressStates[id] = errorState;
        });

        get()._addToHistory(errorState);
        get()._notifySubscribers();
        get()._notifyProgressSubscribers(id, errorState);
      },

      cancelProgress: (id) => {
        const currentState = get().progressStates[id];
        if (!currentState) return;

        const cancelledState: ProgressState = {
          ...currentState,
          status: "cancelled",
          endTime: new Date(),
        };

        set((draft) => {
          draft.progressStates[id] = cancelledState;
        });

        get()._addToHistory(cancelledState);
        get()._notifySubscribers();
        get()._notifyProgressSubscribers(id, cancelledState);
      },

      clearProgress: (id) => {
        set((draft) => {
          delete draft.progressStates[id];
        });

        get()._notifySubscribers();
      },

      clearAllProgress: () => {
        set((draft) => {
          draft.progressStates = {};
        });

        get()._notifySubscribers();
      },

      // ============================================================================
      // 历史记录
      // ============================================================================

      getHistory: () => {
        return get().history;
      },

      clearHistory: () => {
        set((draft) => {
          draft.history = [];
        });
      },

      // ============================================================================
      // 通知配置
      // ============================================================================

      updateNotificationConfig: (config) => {
        set((draft) => {
          draft.notificationConfig = { ...draft.notificationConfig, ...config };
        });
      },

      // ============================================================================
      // 事件订阅
      // ============================================================================

      subscribe: (callback) => {
        const id = Math.random().toString(36);
        get().subscribers.set(id, callback);
        
        return () => {
          get().subscribers.delete(id);
        };
      },

      subscribeToProgress: (progressId, callback) => {
        const { progressSubscribers } = get();
        
        if (!progressSubscribers.has(progressId)) {
          progressSubscribers.set(progressId, new Map());
        }
        
        const id = Math.random().toString(36);
        progressSubscribers.get(progressId)!.set(id, callback);
        
        return () => {
          const subs = get().progressSubscribers.get(progressId);
          if (subs) {
            subs.delete(id);
            if (subs.size === 0) {
              get().progressSubscribers.delete(progressId);
            }
          }
        };
      },

      // ============================================================================
      // 内部方法
      // ============================================================================

      _notifySubscribers: () => {
        const states = get().getAllProgress();
        get().subscribers.forEach((callback) => {
          try {
            callback(states);
          } catch (error) {
            console.error("Error in progress subscriber:", error);
          }
        });
      },

      _notifyProgressSubscribers: (id, state) => {
        const subs = get().progressSubscribers.get(id);
        if (subs) {
          subs.forEach((callback) => {
            try {
              callback(state);
            } catch (error) {
              console.error("Error in progress subscriber:", error);
            }
          });
        }
      },

      _addToHistory: (state) => {
        const historyEntry: ProgressHistoryEntry = {
          id: state.id,
          type: state.type,
          repositoryId: "repositoryId" in state ? (state as any).repositoryId : undefined,
          startTime: state.startTime!,
          endTime: state.endTime,
          duration: state.startTime && state.endTime 
            ? state.endTime.getTime() - state.startTime.getTime() 
            : undefined,
          status: state.status,
          error: state.error,
          metadata: {
            progress: state.progress,
            // 添加类型特定的元数据
            ...(state.type === "indexing" && "filesProcessed" in state 
              ? { filesProcessed: state.filesProcessed, totalFiles: state.totalFiles }
              : {}),
            ...(state.type === "wiki_generation" && "pagesCount" in state 
              ? { pagesCount: state.pagesCount, sectionsCount: state.sectionsCount }
              : {}),
          },
        };

        set((draft) => {
          draft.history.unshift(historyEntry);
          // 保留最近 100 条记录
          if (draft.history.length > 100) {
            draft.history = draft.history.slice(0, 100);
          }
        });
      },
    })),
    {
      name: "progress-store",
    }
  )
);

// ============================================================================
// 便捷 Hooks
// ============================================================================

export const useProgress = (id?: string) => {
  const store = useProgressStore();
  
  if (id) {
    return store.getProgress(id);
  }
  
  return {
    getAllProgress: store.getAllProgress,
    getProgressByType: store.getProgressByType,
    getProgressByRepository: store.getProgressByRepository,
    getProgressStats: store.getProgressStats,
  };
};

export const useProgressActions = () => {
  const store = useProgressStore();
  
  return {
    startProgress: store.startProgress,
    updateProgress: store.updateProgress,
    completeProgress: store.completeProgress,
    errorProgress: store.errorProgress,
    cancelProgress: store.cancelProgress,
    clearProgress: store.clearProgress,
    clearAllProgress: store.clearAllProgress,
  };
};
