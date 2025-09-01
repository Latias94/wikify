/**
 * React Query hooks for API operations
 * 封装所有 API 调用的 React Query hooks
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  apiClient,
  createQueryConfig,
  createMutationConfig,
} from "@/lib/api-client";
import { useAppStore } from "@/store/app-store";
import { useToast } from "@/hooks/use-toast";
import {
  Repository,
  InitializeRepositoryRequest,
  ChatQueryRequest,
  GenerateWikiRequest,
  PaginationParams,
} from "@/types/api";

// ============================================================================
// Query Keys
// ============================================================================

export const queryKeys = {
  // 系统相关
  health: ["health"] as const,
  config: ["config"] as const,

  // 仓库相关
  repositories: ["repositories"] as const,
  repository: (id: string) => ["repositories", id] as const,

  // 聊天相关
  queryHistory: (repositoryId: string, params?: PaginationParams) =>
    ["queryHistory", repositoryId, params] as const,

  // Wiki 相关
  wiki: (repositoryId: string) => ["wiki", repositoryId] as const,

  // 文件相关
  fileTree: (repositoryId: string) => ["fileTree", repositoryId] as const,
  fileContent: (repositoryId: string, filePath: string) =>
    ["fileContent", repositoryId, filePath] as const,
};

// ============================================================================
// 系统 API Hooks
// ============================================================================

/**
 * 健康检查
 */
export function useHealthCheck() {
  return useQuery(
    createQueryConfig(queryKeys.health, () => apiClient.healthCheck(), {
      staleTime: 30 * 1000, // 30 seconds
      refetchOnWindowFocus: false,
    })
  );
}

/**
 * 获取配置
 */
export function useConfig() {
  return useQuery(
    createQueryConfig(queryKeys.config, () => apiClient.getConfig(), {
      staleTime: 5 * 60 * 1000, // 5 minutes
    })
  );
}

/**
 * 更新配置
 */
export function useUpdateConfig() {
  const queryClient = useQueryClient();
  const { toast } = useToast();

  return useMutation(
    createMutationConfig(
      (config: Parameters<typeof apiClient.updateConfig>[0]) =>
        apiClient.updateConfig(config),
      {
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: queryKeys.config });
          toast({
            title: "Configuration updated",
            description: "Your settings have been saved successfully.",
          });
        },
        onError: (error) => {
          toast({
            title: "Failed to update configuration",
            description: error.message,
            variant: "destructive",
          });
        },
      }
    )
  );
}

// ============================================================================
// 仓库 API Hooks
// ============================================================================

/**
 * 获取认证状态
 */
export function useAuthStatus() {
  return useQuery(
    createQueryConfig(["auth", "status"], () => apiClient.getAuthStatus(), {
      staleTime: 5 * 60 * 1000, // 5 minutes
      retry: 1,
    })
  );
}

/**
 * 获取仓库列表
 */
export function useRepositories() {
  const setRepositories = useAppStore((state) => state.setRepositories);
  const setLoading = useAppStore((state) => state.setLoading);
  const setError = useAppStore((state) => state.setError);

  return useQuery(
    createQueryConfig(
      queryKeys.repositories,
      async () => {
        setLoading("repositories", true);
        try {
          const response = await apiClient.getRepositories();
          setRepositories(response.repositories);
          setError("repositories", undefined);
          return response;
        } catch (error) {
          const errorMessage =
            error instanceof Error ? error.message : "Unknown error";
          setError("repositories", errorMessage);
          throw error;
        } finally {
          setLoading("repositories", false);
        }
      },
      {
        staleTime: 2 * 60 * 1000, // 2 minutes
      }
    )
  );
}

/**
 * 初始化仓库
 */
export function useInitializeRepository() {
  const queryClient = useQueryClient();
  const addRepository = useAppStore((state) => state.addRepository);
  const { toast } = useToast();

  return useMutation({
    mutationFn: (data: InitializeRepositoryRequest) =>
      apiClient.initializeRepository(data),
    onSuccess: (response, variables) => {
      // 刷新仓库列表
      queryClient.invalidateQueries({ queryKey: queryKeys.repositories });

      toast({
        title: "Repository indexing started",
        description: `${variables.repository} has been added and indexing has started. You can monitor the progress below.`,
      });
    },
    onError: (error: any) => {
      let title = "Failed to add repository";
      let description = error.message;

      // Handle specific error cases
      if (error.status === 409) {
        title = "Repository already being indexed";
        description =
          "This repository is currently being indexed. Please wait for it to complete or try again later.";
      }

      toast({
        title,
        description,
        variant: "destructive",
      });
    },
    // 防止重复请求的配置
    retry: false,
    gcTime: 0, // 立即清理缓存
  });
}

/**
 * 获取仓库信息
 */
export function useRepository(repositoryId: string) {
  return useQuery(
    createQueryConfig(
      queryKeys.repository(repositoryId),
      () => apiClient.getRepository(repositoryId),
      {
        enabled: !!repositoryId,
        staleTime: 5 * 60 * 1000, // 5 minutes
      }
    )
  );
}

/**
 * 重新索引仓库
 */
export function useReindexRepository() {
  const queryClient = useQueryClient();
  const { toast } = useToast();

  return useMutation({
    mutationFn: (repositoryId: string) =>
      apiClient.reindexRepository(repositoryId),
    onSuccess: (response, repositoryId) => {
      // 刷新仓库列表
      queryClient.invalidateQueries({ queryKey: queryKeys.repositories });

      toast({
        title: "Repository reindexing started",
        description:
          "The repository is being reindexed. You can monitor the progress.",
      });
    },
    onError: (error: any) => {
      let title = "Failed to reindex repository";
      let description = error.message;

      // Handle specific error cases
      if (error.status === 409) {
        title = "Repository is being indexed";
        description =
          "The repository is currently being indexed. Please wait for it to complete.";
      } else if (error.status === 404) {
        title = "Repository not found";
        description =
          "The repository session was not found. Please try refreshing the page.";
      }

      toast({
        title,
        description,
        variant: "destructive",
      });
    },
    // 防止重复请求的配置
    retry: false,
    gcTime: 0,
  });
}

/**
 * 删除仓库
 */
export function useDeleteRepository() {
  const queryClient = useQueryClient();
  const removeRepository = useAppStore((state) => state.removeRepository);
  const { toast } = useToast();

  return useMutation(
    createMutationConfig(
      (repositoryId: string) => apiClient.deleteRepository(repositoryId),
      {
        onSuccess: (_, repositoryId) => {
          // 更新本地状态
          removeRepository(repositoryId);

          // 刷新相关查询
          queryClient.invalidateQueries({ queryKey: queryKeys.repositories });
          queryClient.removeQueries({
            queryKey: queryKeys.repository(repositoryId),
          });

          toast({
            title: "Repository deleted",
            description: "The repository has been removed successfully.",
          });
        },
        onError: (error) => {
          toast({
            title: "Failed to delete repository",
            description: error.message,
            variant: "destructive",
          });
        },
      }
    )
  );
}

// ============================================================================
// 聊天 API Hooks
// ============================================================================

/**
 * 发送聊天消息
 */
export function useSendChatMessage() {
  const { toast } = useToast();

  return useMutation(
    createMutationConfig(
      (data: ChatQueryRequest) => apiClient.sendChatMessage(data),
      {
        onError: (error) => {
          toast({
            title: "Failed to send message",
            description: error.message,
            variant: "destructive",
          });
        },
      }
    )
  );
}

/**
 * 获取查询历史
 */
export function useQueryHistory(
  repositoryId: string,
  params?: PaginationParams
) {
  return useQuery(
    createQueryConfig(
      queryKeys.queryHistory(repositoryId, params),
      () => apiClient.getQueryHistory(repositoryId, params),
      {
        enabled: !!repositoryId,
        staleTime: 5 * 60 * 1000, // 5 minutes
      }
    )
  );
}

// ============================================================================
// Wiki API Hooks
// ============================================================================

/**
 * 生成 Wiki
 */
export function useGenerateWiki() {
  const { toast } = useToast();

  return useMutation(
    createMutationConfig(
      (data: GenerateWikiRequest) => apiClient.generateWiki(data),
      {
        onSuccess: () => {
          toast({
            title: "Wiki generation started",
            description:
              "Your wiki is being generated. This may take a few minutes.",
          });
        },
        onError: (error) => {
          toast({
            title: "Failed to generate wiki",
            description: error.message,
            variant: "destructive",
          });
        },
      }
    )
  );
}

/**
 * 获取 Wiki 内容
 */
export function useWiki(repositoryId: string) {
  return useQuery(
    createQueryConfig(
      queryKeys.wiki(repositoryId),
      () => apiClient.getWiki(repositoryId),
      {
        enabled: !!repositoryId,
        staleTime: 10 * 60 * 1000, // 10 minutes
      }
    )
  );
}

// ============================================================================
// 文件 API Hooks
// ============================================================================

/**
 * 获取文件树
 */
export function useFileTree(repositoryId: string) {
  return useQuery(
    createQueryConfig(
      queryKeys.fileTree(repositoryId),
      () => apiClient.getFileTree(repositoryId),
      {
        enabled: !!repositoryId,
        staleTime: 5 * 60 * 1000, // 5 minutes
      }
    )
  );
}

/**
 * 获取文件内容
 */
export function useFileContent(repositoryId: string, filePath: string) {
  return useQuery(
    createQueryConfig(
      queryKeys.fileContent(repositoryId, filePath),
      () => apiClient.getFileContent(repositoryId, filePath),
      {
        enabled: !!repositoryId && !!filePath,
        staleTime: 10 * 60 * 1000, // 10 minutes
      }
    )
  );
}

// ============================================================================
// 导出 Wiki 功能
// ============================================================================

/**
 * 导出 Wiki
 */
export function useExportWiki() {
  const { toast } = useToast();

  return useMutation(
    createMutationConfig(
      ({
        sessionId,
        format,
      }: {
        sessionId: string;
        format: "markdown" | "html" | "pdf";
      }) => apiClient.exportWiki(sessionId, format),
      {
        onSuccess: (blob, { format }) => {
          // 创建下载链接
          const url = window.URL.createObjectURL(blob);
          const a = document.createElement("a");
          a.href = url;
          a.download = `wiki.${format}`;
          document.body.appendChild(a);
          a.click();
          window.URL.revokeObjectURL(url);
          document.body.removeChild(a);

          toast({
            title: "Wiki exported",
            description: `Wiki has been exported as ${format.toUpperCase()}.`,
          });
        },
        onError: (error) => {
          toast({
            title: "Failed to export wiki",
            description: error.message,
            variant: "destructive",
          });
        },
      }
    )
  );
}
