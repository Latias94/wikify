/**
 * 全局应用状态管理
 * 使用 Zustand 管理客户端状态
 */

import { create } from 'zustand';
import { devtools, persist } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';
import { Repository, Session, User } from '@/types/api';
import { Theme, UserSettings } from '@/types/ui';

// ============================================================================
// 状态接口定义
// ============================================================================

interface AppState {
  // 用户相关
  currentUser: User | null;
  isAuthenticated: boolean;
  
  // 仓库相关
  repositories: Repository[];
  selectedRepository: Repository | null;
  
  // 会话相关
  sessions: Session[];
  activeSession: Session | null;
  
  // UI 状态
  theme: Theme;
  sidebarCollapsed: boolean;
  settings: UserSettings;
  
  // 加载状态
  isLoading: {
    repositories: boolean;
    sessions: boolean;
    user: boolean;
  };
  
  // 错误状态
  errors: {
    repositories?: string;
    sessions?: string;
    user?: string;
    general?: string;
  };
}

interface AppActions {
  // 用户操作
  setCurrentUser: (user: User | null) => void;
  setAuthenticated: (authenticated: boolean) => void;
  
  // 仓库操作
  setRepositories: (repositories: Repository[]) => void;
  addRepository: (repository: Repository) => void;
  updateRepository: (id: string, updates: Partial<Repository>) => void;
  removeRepository: (id: string) => void;
  setSelectedRepository: (repository: Repository | null) => void;
  
  // 会话操作
  setSessions: (sessions: Session[]) => void;
  addSession: (session: Session) => void;
  updateSession: (id: string, updates: Partial<Session>) => void;
  removeSession: (id: string) => void;
  setActiveSession: (session: Session | null) => void;
  
  // UI 操作
  setTheme: (theme: Theme) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  updateSettings: (settings: Partial<UserSettings>) => void;
  
  // 加载状态操作
  setLoading: (key: keyof AppState['isLoading'], loading: boolean) => void;
  
  // 错误处理
  setError: (key: keyof AppState['errors'], error: string | undefined) => void;
  clearErrors: () => void;
  
  // 重置操作
  reset: () => void;
}

type AppStore = AppState & AppActions;

// ============================================================================
// 默认状态
// ============================================================================

const defaultSettings: UserSettings = {
  theme: 'system',
  language: 'en',
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
  // 用户相关
  currentUser: null,
  isAuthenticated: false,
  
  // 仓库相关
  repositories: [],
  selectedRepository: null,
  
  // 会话相关
  sessions: [],
  activeSession: null,
  
  // UI 状态
  theme: 'system',
  sidebarCollapsed: false,
  settings: defaultSettings,
  
  // 加载状态
  isLoading: {
    repositories: false,
    sessions: false,
    user: false,
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
        // 用户操作
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
            const index = state.repositories.findIndex(repo => repo.id === id);
            if (index !== -1) {
              Object.assign(state.repositories[index], updates);
            }
          });
        },
        
        removeRepository: (id) => {
          set((state) => {
            state.repositories = state.repositories.filter(repo => repo.id !== id);
            
            // 如果删除的是当前选中的仓库，清除选择
            if (state.selectedRepository?.id === id) {
              state.selectedRepository = null;
            }
            
            // 清除相关会话
            state.sessions = state.sessions.filter(session => session.repository_id !== id);
            if (state.activeSession?.repository_id === id) {
              state.activeSession = null;
            }
          });
        },
        
        setSelectedRepository: (repository) => {
          set((state) => {
            state.selectedRepository = repository;
          });
        },
        
        // ============================================================================
        // 会话操作
        // ============================================================================
        
        setSessions: (sessions) => {
          set((state) => {
            state.sessions = sessions;
          });
        },
        
        addSession: (session) => {
          set((state) => {
            state.sessions.push(session);
          });
        },
        
        updateSession: (id, updates) => {
          set((state) => {
            const index = state.sessions.findIndex(session => session.id === id);
            if (index !== -1) {
              Object.assign(state.sessions[index], updates);
            }
          });
        },
        
        removeSession: (id) => {
          set((state) => {
            state.sessions = state.sessions.filter(session => session.id !== id);
            
            // 如果删除的是当前活跃会话，清除选择
            if (state.activeSession?.id === id) {
              state.activeSession = null;
            }
          });
        },
        
        setActiveSession: (session) => {
          set((state) => {
            state.activeSession = session;
            
            // 自动设置选中的仓库
            if (session) {
              const repository = state.repositories.find(
                repo => repo.id === session.repository_id
              );
              if (repository) {
                state.selectedRepository = repository;
              }
            }
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
        // 重置操作
        // ============================================================================
        
        reset: () => {
          set(initialState);
        },
      })),
      {
        name: 'wikify-app-store',
        partialize: (state) => ({
          // 只持久化部分状态
          theme: state.theme,
          sidebarCollapsed: state.sidebarCollapsed,
          settings: state.settings,
          selectedRepository: state.selectedRepository,
          activeSession: state.activeSession,
        }),
      }
    ),
    {
      name: 'wikify-app-store',
    }
  )
);

// ============================================================================
// 选择器 Hooks
// ============================================================================

// 用户相关选择器
export const useCurrentUser = () => useAppStore(state => state.currentUser);
export const useIsAuthenticated = () => useAppStore(state => state.isAuthenticated);

// 仓库相关选择器
export const useRepositories = () => useAppStore(state => state.repositories);
export const useSelectedRepository = () => useAppStore(state => state.selectedRepository);

// 会话相关选择器
export const useSessions = () => useAppStore(state => state.sessions);
export const useActiveSession = () => useAppStore(state => state.activeSession);

// UI 相关选择器
export const useTheme = () => useAppStore(state => state.theme);
export const useSidebarCollapsed = () => useAppStore(state => state.sidebarCollapsed);
export const useSettings = () => useAppStore(state => state.settings);

// 加载状态选择器
export const useLoadingState = () => useAppStore(state => state.isLoading);
export const useErrors = () => useAppStore(state => state.errors);

// 组合选择器
export const useRepositoryById = (id: string) => 
  useAppStore(state => state.repositories.find(repo => repo.id === id));

export const useSessionsByRepository = (repositoryId: string) =>
  useAppStore(state => state.sessions.filter(session => session.repository_id === repositoryId));

export default useAppStore;
