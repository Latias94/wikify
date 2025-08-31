/**
 * 认证提供者组件
 * 负责检测后端认证模式并初始化认证状态
 */

import React, { createContext, useContext, useEffect, ReactNode } from 'react';
import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import { useAppStore } from '@/store/app-store';
import { useAuth } from '@/hooks/use-auth';
import { AuthStatusResponse, AuthMode } from '@/types/api';
import { Loader2 } from 'lucide-react';

// ============================================================================
// Context 定义
// ============================================================================

interface AuthContextValue {
  authStatus: AuthStatusResponse | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

// ============================================================================
// Hook
// ============================================================================

export const useAuthContext = () => {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuthContext must be used within AuthProvider');
  }
  return context;
};

// ============================================================================
// 组件
// ============================================================================

interface AuthProviderProps {
  children: ReactNode;
}

export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const { setAuthMode, setAuthRequired, setLoading, setError } = useAppStore();
  const { isInitialized } = useAuth();

  // 查询认证状态
  const {
    data: authStatus,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['auth', 'status'],
    queryFn: () => apiClient.getAuthStatus(),
    staleTime: 5 * 60 * 1000, // 5 minutes
    retry: 3,
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
  });

  // 更新全局状态
  useEffect(() => {
    setLoading('auth', isLoading);
  }, [isLoading, setLoading]);

  useEffect(() => {
    if (error) {
      setError('auth', error.message);
      console.error('Failed to fetch auth status:', error);
    } else {
      setError('auth', undefined);
    }
  }, [error, setError]);

  useEffect(() => {
    if (authStatus) {
      setAuthMode(authStatus.auth_mode);
      setAuthRequired(authStatus.auth_required);

      console.log('Auth status loaded:', {
        mode: authStatus.auth_mode,
        required: authStatus.auth_required,
        features: authStatus.features,
      });
    }
  }, [authStatus, setAuthMode, setAuthRequired]);

  // 提供上下文值
  const contextValue: AuthContextValue = {
    authStatus,
    isLoading,
    error,
    refetch,
  };

  return (
    <AuthContext.Provider value={contextValue}>
      {children}
    </AuthContext.Provider>
  );
};

// ============================================================================
// 认证模式检测组件
// ============================================================================

interface AuthModeDetectorProps {
  children: ReactNode;
  fallback?: ReactNode;
}

export const AuthModeDetector: React.FC<AuthModeDetectorProps> = ({
  children,
  fallback,
}) => {
  const { authStatus, isLoading, error } = useAuthContext();

  // 加载中状态
  if (isLoading) {
    return (
      fallback || (
        <div className="flex items-center justify-center min-h-screen">
          <div className="flex flex-col items-center gap-4">
            <Loader2 className="h-8 w-8 animate-spin text-primary" />
            <p className="text-muted-foreground">Detecting authentication mode...</p>
          </div>
        </div>
      )
    );
  }

  // 错误状态
  if (error) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-center space-y-4">
          <div className="text-destructive">
            <h2 className="text-lg font-semibold">Connection Error</h2>
            <p className="text-sm text-muted-foreground">
              Unable to connect to the backend server
            </p>
          </div>
          <button
            onClick={() => window.location.reload()}
            className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  // 成功加载，渲染子组件
  return <>{children}</>;
};

// ============================================================================
// 权限模式条件渲染组件
// ============================================================================

interface AuthModeConditionalProps {
  modes: AuthMode[];
  children: ReactNode;
  fallback?: ReactNode;
}

export const AuthModeConditional: React.FC<AuthModeConditionalProps> = ({
  modes,
  children,
  fallback = null,
}) => {
  const { authStatus } = useAuthContext();

  if (!authStatus) {
    return <>{fallback}</>;
  }

  const shouldRender = modes.includes(authStatus.auth_mode);
  return shouldRender ? <>{children}</> : <>{fallback}</>;
};

// ============================================================================
// 功能检测组件
// ============================================================================

interface FeatureConditionalProps {
  feature: keyof AuthStatusResponse['features'];
  children: ReactNode;
  fallback?: ReactNode;
}

export const FeatureConditional: React.FC<FeatureConditionalProps> = ({
  feature,
  children,
  fallback = null,
}) => {
  const { authStatus } = useAuthContext();

  if (!authStatus) {
    return <>{fallback}</>;
  }

  const isEnabled = authStatus.features[feature];
  return isEnabled ? <>{children}</> : <>{fallback}</>;
};

// ============================================================================
// 认证要求组件
// ============================================================================

interface AuthRequiredProps {
  children: ReactNode;
  fallback?: ReactNode;
}

export const AuthRequired: React.FC<AuthRequiredProps> = ({
  children,
  fallback,
}) => {
  const { authStatus } = useAuthContext();
  const { isAuthenticated } = useAuth();

  if (!authStatus) {
    return <>{fallback}</>;
  }

  // 如果不需要认证，直接渲染
  if (!authStatus.auth_required) {
    return <>{children}</>;
  }

  // 需要认证但未认证
  if (!isAuthenticated) {
    return <>{fallback}</>;
  }

  // 已认证，渲染内容
  return <>{children}</>;
};

// ============================================================================
// 开源部署信息组件
// ============================================================================

export const OpenSourceBadge: React.FC = () => {
  return (
    <AuthModeConditional modes={['open']}>
      <div className="fixed bottom-4 right-4 z-50">
        <div className="bg-primary/10 border border-primary/20 rounded-lg px-3 py-2 text-xs">
          <span className="text-primary font-medium">Open Source Mode</span>
        </div>
      </div>
    </AuthModeConditional>
  );
};

export default AuthProvider;
