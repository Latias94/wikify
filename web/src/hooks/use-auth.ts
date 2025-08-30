/**
 * 认证相关的 React Hooks
 * 提供登录、注册、权限检查等功能
 */

import { useState, useEffect, useCallback } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { useToast } from "@/hooks/use-toast";
import { apiClient } from "@/lib/api-client";
import { useAppStore } from "@/store/app-store";
import {
  AuthStatusResponse,
  LoginRequest,
  RegisterRequest,
  AuthResponse,
  RefreshTokenRequest,
  UserInfo,
  Permission,
  AuthMode,
} from "@/types/api";

// ============================================================================
// 认证状态管理
// ============================================================================

/**
 * 获取认证状态
 */
export const useAuthStatus = () => {
  return useQuery({
    queryKey: ["auth", "status"],
    queryFn: () => apiClient.getAuthStatus(),
    staleTime: 5 * 60 * 1000, // 5 minutes
    retry: 1,
  });
};

/**
 * 用户认证 Hook
 */
export const useAuth = () => {
  const navigate = useNavigate();
  const { toast } = useToast();
  const queryClient = useQueryClient();

  // Store state
  const { currentUser, isAuthenticated, setCurrentUser, setAuthenticated } =
    useAppStore();

  // Local state
  const [isInitialized, setIsInitialized] = useState(false);

  // ============================================================================
  // Token 管理
  // ============================================================================

  const getStoredToken = useCallback(() => {
    return localStorage.getItem("wikify_access_token");
  }, []);

  const getStoredRefreshToken = useCallback(() => {
    return localStorage.getItem("wikify_refresh_token");
  }, []);

  const storeTokens = useCallback(
    (tokens: { access_token: string; refresh_token: string }) => {
      localStorage.setItem("wikify_access_token", tokens.access_token);
      localStorage.setItem("wikify_refresh_token", tokens.refresh_token);
    },
    []
  );

  const clearTokens = useCallback(() => {
    localStorage.removeItem("wikify_access_token");
    localStorage.removeItem("wikify_refresh_token");
  }, []);

  // ============================================================================
  // 认证操作
  // ============================================================================

  /**
   * 登录
   */
  const loginMutation = useMutation({
    mutationFn: (data: LoginRequest) => apiClient.login(data),
    onSuccess: (response: AuthResponse) => {
      storeTokens(response.tokens);
      setCurrentUser(response.user);
      setAuthenticated(true);

      toast({
        title: "Login Successful",
        description: `Welcome back, ${
          response.user.display_name || response.user.username
        }!`,
      });

      // 刷新相关查询
      queryClient.invalidateQueries({ queryKey: ["auth"] });
      queryClient.invalidateQueries({ queryKey: ["user"] });
    },
    onError: (error: any) => {
      toast({
        title: "Login Failed",
        description: error.message || "Invalid username or password",
        variant: "destructive",
      });
    },
  });

  /**
   * 注册
   */
  const registerMutation = useMutation({
    mutationFn: (data: RegisterRequest) => apiClient.register(data),
    onSuccess: (response: AuthResponse) => {
      storeTokens(response.tokens);
      setCurrentUser(response.user);
      setAuthenticated(true);

      toast({
        title: "Registration Successful",
        description: `Welcome to Wikify, ${
          response.user.display_name || response.user.username
        }!`,
      });

      // 刷新相关查询
      queryClient.invalidateQueries({ queryKey: ["auth"] });
      queryClient.invalidateQueries({ queryKey: ["user"] });
    },
    onError: (error: any) => {
      toast({
        title: "Registration Failed",
        description: error.message || "Failed to create account",
        variant: "destructive",
      });
    },
  });

  /**
   * 登出
   */
  const logout = useCallback(async () => {
    try {
      // 可选：调用后端登出接口
      // await apiClient.logout();
    } catch (error) {
      console.warn("Logout API call failed:", error);
    } finally {
      // 清理本地状态
      clearTokens();
      setCurrentUser(null);
      setAuthenticated(false);

      // 清理查询缓存
      queryClient.clear();

      toast({
        title: "Logged Out",
        description: "You have been successfully logged out.",
      });

      // 重定向到首页
      navigate("/");
    }
  }, [
    clearTokens,
    setCurrentUser,
    setAuthenticated,
    queryClient,
    toast,
    navigate,
  ]);

  /**
   * Token 刷新
   */
  const refreshTokenMutation = useMutation({
    mutationFn: (data: RefreshTokenRequest) => apiClient.refreshToken(data),
    onSuccess: (response: AuthResponse) => {
      storeTokens(response.tokens);
      setCurrentUser(response.user);
      setAuthenticated(true);
    },
    onError: () => {
      // Token 刷新失败，需要重新登录
      logout();
    },
  });

  // ============================================================================
  // 权限检查
  // ============================================================================

  /**
   * 检查用户是否有特定权限
   */
  const hasPermission = useCallback(
    (permission: Permission): boolean => {
      if (!currentUser) return false;
      return (
        currentUser.permissions.includes(permission) || currentUser.is_admin
      );
    },
    [currentUser]
  );

  /**
   * 检查用户是否为管理员
   */
  const isAdmin = useCallback((): boolean => {
    return currentUser?.is_admin || false;
  }, [currentUser]);

  /**
   * 检查是否需要认证
   */
  const requiresAuth = useCallback((authMode: AuthMode): boolean => {
    return authMode !== "open";
  }, []);

  // ============================================================================
  // 初始化
  // ============================================================================

  /**
   * 监听token过期事件
   */
  useEffect(() => {
    const handleTokenExpired = () => {
      console.log("Token expired, logging out user");
      logout();
    };

    window.addEventListener("auth:token-expired", handleTokenExpired);
    return () =>
      window.removeEventListener("auth:token-expired", handleTokenExpired);
  }, [logout]);

  /**
   * 初始化认证状态
   */
  useEffect(() => {
    const initializeAuth = async () => {
      const token = getStoredToken();
      const refreshToken = getStoredRefreshToken();

      if (token && refreshToken) {
        try {
          // 尝试刷新 token 来验证用户状态
          await refreshTokenMutation.mutateAsync({
            refresh_token: refreshToken,
          });
        } catch (error) {
          console.warn("Failed to refresh token on initialization:", error);
          clearTokens();
        }
      }

      setIsInitialized(true);
    };

    if (!isInitialized) {
      initializeAuth();
    }
  }, [
    isInitialized,
    getStoredToken,
    getStoredRefreshToken,
    refreshTokenMutation,
    clearTokens,
  ]);

  // ============================================================================
  // 返回值
  // ============================================================================

  return {
    // 状态
    currentUser,
    isAuthenticated,
    isInitialized,

    // 操作
    login: loginMutation.mutate,
    register: registerMutation.mutate,
    logout,

    // 权限检查
    hasPermission,
    isAdmin,
    requiresAuth,

    // 加载状态
    isLoggingIn: loginMutation.isPending,
    isRegistering: registerMutation.isPending,
    isRefreshing: refreshTokenMutation.isPending,
  };
};

// ============================================================================
// 权限保护 Hook
// ============================================================================

/**
 * 权限保护 Hook
 * 用于保护需要特定权限的组件或页面
 */
export const useRequireAuth = (requiredPermission?: Permission) => {
  const { currentUser, isAuthenticated, isInitialized } = useAuth();
  const navigate = useNavigate();

  useEffect(() => {
    if (!isInitialized) return;

    if (!isAuthenticated) {
      navigate("/login");
      return;
    }

    if (requiredPermission && currentUser) {
      const hasPermission =
        currentUser.permissions.includes(requiredPermission) ||
        currentUser.is_admin;
      if (!hasPermission) {
        navigate("/unauthorized");
        return;
      }
    }
  }, [
    isAuthenticated,
    isInitialized,
    currentUser,
    requiredPermission,
    navigate,
  ]);

  return {
    isAuthorized:
      isAuthenticated &&
      (!requiredPermission ||
        currentUser?.permissions.includes(requiredPermission) ||
        currentUser?.is_admin),
    isLoading: !isInitialized,
  };
};
