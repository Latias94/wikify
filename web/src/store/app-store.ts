/**
 * 全局应用状态管理
 * 使用 Zustand 管理客户端状态
 */

import { create } from "zustand";
import { devtools, persist } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";
import { Repository, UserInfo, AuthMode, Permission } from "@/types/api";
import { Theme, UserSettings } from "@/types/ui";

// ============================================================================
// 状态接口定义
// ============================================================================

interface AppState {
  // 认证相关
  currentUser: UserInfo | null;
  isAuthenticated: boolean;
  authMode: AuthMode;
  authRequired: boolean;

  // 仓库相关
  repositories: Repository[];
  selectedRepository: Repository | null;

  // UI 状态
  theme: Theme;
  sidebarCollapsed: boolean;
  settings: UserSettings;

  // 加载状态
  isLoading: {
    repositories: boolean;
    user: boolean;
    auth: boolean;
  };

  // 错误状态
  errors: {
    repositories?: string;
    user?: string;
    auth?: string;
    general?: string;
  };
}

interface AppActions {
  // 认证操作
  setCurrentUser: (user: UserInfo | null) => void;
  setAuthenticated: (authenticated: boolean) => void;
  setAuthMode: (mode: AuthMode) => void;
  setAuthRequired: (required: boolean) => void;

  // 仓库操作
  setRepositories: (repositories: Repository[]) => void;
  addRepository: (repository: Repository) => void;
  updateRepository: (id: string, updates: Partial<Repository>) => void;
  removeRepository: (id: string) => void;
  setSelectedRepository: (repository: Repository | null) => void;

  // UI 操作
  setTheme: (theme: Theme) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  updateSettings: (settings: Partial<UserSettings>) => void;

  // 加载状态操作
  setLoading: (key: keyof AppState["isLoading"], loading: boolean) => void;

  // 错误处理
  setError: (key: keyof AppState["errors"], error: string | undefined) => void;
  clearErrors: () => void;

  // 权限检查
  hasPermission: (permission: Permission) => boolean;
  isAdmin: () => boolean;

  // 重置操作
  reset: () => void;
}

type AppStore = AppState & AppActions;

// ============================================================================
// 默认状态
// ============================================================================

const defaultSettings: UserSettings = {
  theme: "system",
  language: "en",
  notifications: {
    enabled: true,
    sound: true,
    desktop: true,
  },
  chat: {
    showTimestamps: true,
    showSources: true,
    autoScroll: true,
    maxMessages: 100,
  },
  editor: {
    fontSize: 14,
    tabSize: 2,
    wordWrap: true,
    lineNumbers: true,
  },
};

const initialState: AppState = {
  // 认证相关
  currentUser: null,
  isAuthenticated: false,
  authMode: "open",
  authRequired: false,

  // 仓库相关
  repositories: [],
  selectedRepository: null,

  // UI 状态
  theme: "system",
  sidebarCollapsed: false,
  settings: defaultSettings,

  // 加载状态
  isLoading: {
    repositories: false,
    user: false,
    auth: false,
  },

  // 错误状态
  errors: {},
};

// ============================================================================
// Store 创建
// ============================================================================

export const useAppStore = create<AppStore>()(
  devtools(
    persist(
      immer((set, get) => ({
        ...initialState,

        // ============================================================================
        // 认证操作
        // ============================================================================

        setCurrentUser: (user) => {
          set((state) => {
            state.currentUser = user;
            state.isAuthenticated = !!user;
          });
        },

        setAuthenticated: (authenticated) => {
          set((state) => {
            state.isAuthenticated = authenticated;
            if (!authenticated) {
              state.currentUser = null;
            }
          });
        },

        setAuthMode: (mode) => {
          set((state) => {
            state.authMode = mode;
            state.authRequired = mode !== "open";
          });
        },

        setAuthRequired: (required) => {
          set((state) => {
            state.authRequired = required;
          });
        },

        // ============================================================================
        // 仓库操作
        // ============================================================================

        setRepositories: (repositories) => {
          set((state) => {
            state.repositories = repositories;
          });
        },

        addRepository: (repository) => {
          set((state) => {
            state.repositories.push(repository);
          });
        },

        updateRepository: (id, updates) => {
          set((state) => {
            const index = state.repositories.findIndex(
              (repo) => repo.id === id
            );
            if (index !== -1) {
              Object.assign(state.repositories[index], updates);
            }
          });
        },

        removeRepository: (id) => {
          set((state) => {
            state.repositories = state.repositories.filter(
              (repo) => repo.id !== id
            );

            // 如果删除的是当前选中的仓库，清除选择
            if (state.selectedRepository?.id === id) {
              state.selectedRepository = null;
            }
          });
        },

        setSelectedRepository: (repository) => {
          set((state) => {
            state.selectedRepository = repository;
          });
        },

        // ============================================================================
        // UI 操作
        // ============================================================================

        setTheme: (theme) => {
          set((state) => {
            state.theme = theme;
            state.settings.theme = theme;
          });
        },

        toggleSidebar: () => {
          set((state) => {
            state.sidebarCollapsed = !state.sidebarCollapsed;
          });
        },

        setSidebarCollapsed: (collapsed) => {
          set((state) => {
            state.sidebarCollapsed = collapsed;
          });
        },

        updateSettings: (settings) => {
          set((state) => {
            Object.assign(state.settings, settings);
          });
        },

        // ============================================================================
        // 加载状态操作
        // ============================================================================

        setLoading: (key, loading) => {
          set((state) => {
            state.isLoading[key] = loading;
          });
        },

        // ============================================================================
        // 错误处理
        // ============================================================================

        setError: (key, error) => {
          set((state) => {
            if (error) {
              state.errors[key] = error;
            } else {
              delete state.errors[key];
            }
          });
        },

        clearErrors: () => {
          set((state) => {
            state.errors = {};
          });
        },

        // ============================================================================
        // 权限检查
        // ============================================================================

        hasPermission: (permission) => {
          const state = get();
          if (!state.currentUser) return false;
          return (
            state.currentUser.permissions.includes(permission) ||
            state.currentUser.is_admin
          );
        },

        isAdmin: () => {
          const state = get();
          return state.currentUser?.is_admin || false;
        },

        // ============================================================================
        // 重置操作
        // ============================================================================

        reset: () => {
          set(initialState);
        },
      })),
      {
        name: "wikify-app-store",
        partialize: (state) => ({
          // 只持久化部分状态
          theme: state.theme,
          sidebarCollapsed: state.sidebarCollapsed,
          settings: state.settings,
          selectedRepository: state.selectedRepository,
        }),
      }
    ),
    {
      name: "wikify-app-store",
    }
  )
);

// ============================================================================
// 选择器 Hooks
// ============================================================================

// 认证相关选择器
export const useCurrentUser = () => useAppStore((state) => state.currentUser);
export const useIsAuthenticated = () =>
  useAppStore((state) => state.isAuthenticated);
export const useAuthMode = () => useAppStore((state) => state.authMode);
export const useAuthRequired = () => useAppStore((state) => state.authRequired);

// 仓库相关选择器
export const useRepositories = () => useAppStore((state) => state.repositories);
export const useSelectedRepository = () =>
  useAppStore((state) => state.selectedRepository);

// UI 相关选择器
export const useTheme = () => useAppStore((state) => state.theme);
export const useSidebarCollapsed = () =>
  useAppStore((state) => state.sidebarCollapsed);
export const useSettings = () => useAppStore((state) => state.settings);

// 加载状态选择器
export const useLoadingState = () => useAppStore((state) => state.isLoading);
export const useErrors = () => useAppStore((state) => state.errors);

// 权限相关选择器
export const useHasPermission = () =>
  useAppStore((state) => state.hasPermission);
export const useIsAdmin = () => useAppStore((state) => state.isAdmin);

// 组合选择器
export const useRepositoryById = (id: string) =>
  useAppStore((state) => state.repositories.find((repo) => repo.id === id));

// 权限检查组合选择器
export const usePermissionCheck = (permission: Permission) => {
  const hasPermission = useHasPermission();
  return hasPermission(permission);
};

export const useAuthState = () => {
  const currentUser = useCurrentUser();
  const isAuthenticated = useIsAuthenticated();
  const authMode = useAuthMode();
  const authRequired = useAuthRequired();
  const hasPermission = useHasPermission();
  const isAdmin = useIsAdmin();

  return {
    currentUser,
    isAuthenticated,
    authMode,
    authRequired,
    hasPermission,
    isAdmin,
    needsAuth: authRequired && !isAuthenticated,
  };
};

export default useAppStore;
