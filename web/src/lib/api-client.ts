/**
 * API 客户端
 * 基于 axios 实现统一的 HTTP 请求处理
 */

import axios, {
  AxiosInstance,
  AxiosRequestConfig,
  AxiosResponse,
  AxiosError,
} from "axios";
import {
  Repository,
  InitializeRepositoryRequest,
  InitializeRepositoryResponse,
  RepositoriesResponse,
  Session,
  SessionsResponse,
  ChatQueryRequest,
  ChatQueryResponse,
  QueryHistoryResponse,
  GenerateWikiRequest,
  GenerateWikiResponse,
  WikiStructure,
  FileTreeNode,
  FileContentResponse,
  HealthResponse,
  Config,
  ApiError,
  ApiResponse,
  PaginationParams,
  PaginatedResponse,
} from "@/types/api";
import { backendConnection } from "@/lib/backend-connection";

// ============================================================================
// Configuration and Constants
// ============================================================================

const DEFAULT_API_BASE_URL = "http://localhost:8080/api";
const REQUEST_TIMEOUT = 30000; // 30 seconds
const MAX_RETRIES = 3;

// Get current API base URL from backend connection manager
function getCurrentApiBaseUrl(): string {
  const currentEndpoint = backendConnection.getCurrentEndpoint();
  return (
    currentEndpoint?.apiUrl ||
    import.meta.env.VITE_API_BASE_URL ||
    DEFAULT_API_BASE_URL
  );
}

// ============================================================================
// 错误处理
// ============================================================================

export class ApiClientError extends Error {
  constructor(
    message: string,
    public status?: number,
    public code?: string,
    public details?: any
  ) {
    super(message);
    this.name = "ApiClientError";
  }
}

/**
 * 处理 API 错误响应
 */
function handleApiError(error: AxiosError): never {
  if (error.response) {
    // 服务器返回错误响应
    const { status, data } = error.response;
    const apiError = data as ApiError;

    throw new ApiClientError(
      apiError?.message || error.message,
      status,
      apiError?.error,
      apiError?.details
    );
  } else if (error.request) {
    // 网络错误
    throw new ApiClientError(
      "Network error: Unable to reach the server",
      0,
      "NETWORK_ERROR"
    );
  } else {
    // 其他错误
    throw new ApiClientError(error.message, 0, "UNKNOWN_ERROR");
  }
}

// ============================================================================
// API 客户端类
// ============================================================================

export class ApiClient {
  private instance: AxiosInstance;
  private retryCount = new Map<string, number>();

  constructor(baseURL?: string) {
    this.instance = axios.create({
      baseURL: baseURL || getCurrentApiBaseUrl(),
      timeout: REQUEST_TIMEOUT,
      headers: {
        "Content-Type": "application/json",
      },
    });

    this.setupInterceptors();
  }

  /**
   * Update the base URL for API requests
   */
  updateBaseURL(baseURL: string) {
    this.instance.defaults.baseURL = baseURL;
  }

  /**
   * Get current base URL
   */
  getBaseURL(): string {
    return this.instance.defaults.baseURL || getCurrentApiBaseUrl();
  }

  /**
   * 设置请求和响应拦截器
   */
  private setupInterceptors() {
    // 请求拦截器
    this.instance.interceptors.request.use(
      (config) => {
        // 添加请求时间戳
        config.metadata = { startTime: Date.now() };

        // 添加认证头（如果需要）
        const token = localStorage.getItem("auth_token");
        if (token) {
          config.headers.Authorization = `Bearer ${token}`;
        }

        console.log(
          `🚀 API Request: ${config.method?.toUpperCase()} ${config.url}`
        );
        return config;
      },
      (error) => {
        console.error("❌ Request Error:", error);
        return Promise.reject(error);
      }
    );

    // 响应拦截器
    this.instance.interceptors.response.use(
      (response) => {
        const duration =
          Date.now() - (response.config.metadata?.startTime || 0);
        console.log(
          `✅ API Response: ${response.config.method?.toUpperCase()} ${
            response.config.url
          } (${duration}ms)`
        );
        return response;
      },
      async (error: AxiosError) => {
        const config = error.config;
        const requestKey = `${config?.method}-${config?.url}`;

        // 重试逻辑
        if (this.shouldRetry(error) && config) {
          const retryCount = this.retryCount.get(requestKey) || 0;

          if (retryCount < MAX_RETRIES) {
            this.retryCount.set(requestKey, retryCount + 1);
            console.log(
              `🔄 Retrying request (${retryCount + 1}/${MAX_RETRIES}): ${
                config.url
              }`
            );

            // 指数退避
            await new Promise((resolve) =>
              setTimeout(resolve, Math.pow(2, retryCount) * 1000)
            );
            return this.instance.request(config);
          } else {
            this.retryCount.delete(requestKey);
          }
        }

        console.error(
          `❌ API Error: ${config?.method?.toUpperCase()} ${config?.url}`,
          error
        );
        return Promise.reject(error);
      }
    );
  }

  /**
   * 判断是否应该重试请求
   */
  private shouldRetry(error: AxiosError): boolean {
    if (!error.response) return true; // 网络错误，重试

    const status = error.response.status;
    // 只对服务器错误和特定客户端错误重试
    return status >= 500 || status === 408 || status === 429;
  }

  /**
   * 通用请求方法
   */
  private async request<T>(config: AxiosRequestConfig): Promise<T> {
    try {
      const response: AxiosResponse<T> = await this.instance.request(config);
      return response.data;
    } catch (error) {
      handleApiError(error as AxiosError);
    }
  }

  // ============================================================================
  // 系统 API
  // ============================================================================

  /**
   * 健康检查
   */
  async healthCheck(): Promise<HealthResponse> {
    return this.request<HealthResponse>({
      method: "GET",
      url: "/health",
    });
  }

  /**
   * 获取配置
   */
  async getConfig(): Promise<Config> {
    return this.request<Config>({
      method: "GET",
      url: "/config",
    });
  }

  /**
   * 更新配置
   */
  async updateConfig(config: Partial<Config>): Promise<Config> {
    return this.request<Config>({
      method: "POST",
      url: "/config",
      data: config,
    });
  }

  // ============================================================================
  // 仓库管理 API
  // ============================================================================

  /**
   * 获取仓库列表
   */
  async getRepositories(): Promise<RepositoriesResponse> {
    return this.request<RepositoriesResponse>({
      method: "GET",
      url: "/repositories",
    });
  }

  /**
   * 初始化仓库
   */
  async initializeRepository(
    data: InitializeRepositoryRequest
  ): Promise<InitializeRepositoryResponse> {
    return this.request<InitializeRepositoryResponse>({
      method: "POST",
      url: "/repositories",
      data,
    });
  }

  /**
   * 获取仓库信息
   */
  async getRepository(sessionId: string): Promise<Repository> {
    return this.request<Repository>({
      method: "GET",
      url: `/repositories/${sessionId}`,
    });
  }

  /**
   * 删除仓库
   */
  async deleteRepository(repositoryId: string): Promise<void> {
    return this.request<void>({
      method: "DELETE",
      url: `/repositories/${repositoryId}`,
    });
  }

  // ============================================================================
  // 会话管理 API
  // ============================================================================

  /**
   * 获取会话列表
   */
  async getSessions(): Promise<SessionsResponse> {
    return this.request<SessionsResponse>({
      method: "GET",
      url: "/sessions",
    });
  }

  /**
   * 创建新会话
   */
  async createSession(repositoryId: string, name?: string): Promise<Session> {
    return this.request<Session>({
      method: "POST",
      url: "/sessions",
      data: { repository_id: repositoryId, name },
    });
  }

  /**
   * 删除会话
   */
  async deleteSession(sessionId: string): Promise<void> {
    return this.request<void>({
      method: "DELETE",
      url: `/sessions/${sessionId}`,
    });
  }

  // ============================================================================
  // 聊天 API
  // ============================================================================

  /**
   * 发送聊天消息
   */
  async sendChatMessage(data: ChatQueryRequest): Promise<ChatQueryResponse> {
    return this.request<ChatQueryResponse>({
      method: "POST",
      url: "/chat",
      data,
    });
  }

  /**
   * 获取查询历史
   */
  async getQueryHistory(
    repositoryId: string,
    params?: PaginationParams
  ): Promise<QueryHistoryResponse> {
    return this.request<QueryHistoryResponse>({
      method: "GET",
      url: `/history/${repositoryId}`,
      params,
    });
  }

  // ============================================================================
  // Wiki API
  // ============================================================================

  /**
   * 生成 Wiki
   */
  async generateWiki(data: GenerateWikiRequest): Promise<GenerateWikiResponse> {
    return this.request<GenerateWikiResponse>({
      method: "POST",
      url: "/wiki/generate",
      data,
    });
  }

  /**
   * 获取 Wiki 内容
   */
  async getWiki(sessionId: string): Promise<WikiStructure> {
    return this.request<WikiStructure>({
      method: "GET",
      url: `/wiki/${sessionId}`,
    });
  }

  /**
   * 导出 Wiki
   */
  async exportWiki(
    sessionId: string,
    format: "markdown" | "html" | "pdf"
  ): Promise<Blob> {
    const response = await this.instance.request({
      method: "POST",
      url: `/wiki/${sessionId}/export`,
      data: { format },
      responseType: "blob",
    });
    return response.data;
  }

  // ============================================================================
  // 文件 API
  // ============================================================================

  /**
   * 获取文件树
   */
  async getFileTree(repositoryId: string): Promise<FileTreeNode[]> {
    return this.request<FileTreeNode[]>({
      method: "POST",
      url: "/files/tree",
      data: { repository_id: repositoryId },
    });
  }

  /**
   * 获取文件内容
   */
  async getFileContent(
    repositoryId: string,
    filePath: string
  ): Promise<FileContentResponse> {
    return this.request<FileContentResponse>({
      method: "POST",
      url: "/files/content",
      data: { repository_id: repositoryId, file_path: filePath },
    });
  }
}

// ============================================================================
// React Query 集成辅助函数
// ============================================================================

/**
 * 创建 React Query 的 mutation 配置
 */
export function createMutationConfig<TData, TVariables>(
  mutationFn: (variables: TVariables) => Promise<TData>,
  options?: {
    onSuccess?: (data: TData, variables: TVariables) => void;
    onError?: (error: ApiClientError, variables: TVariables) => void;
  }
) {
  return {
    mutationFn,
    onSuccess: options?.onSuccess,
    onError: options?.onError,
  };
}

/**
 * 创建 React Query 的 query 配置
 */
export function createQueryConfig<TData>(
  queryKey: string[],
  queryFn: () => Promise<TData>,
  options?: {
    staleTime?: number;
    cacheTime?: number;
    refetchOnWindowFocus?: boolean;
  }
) {
  return {
    queryKey,
    queryFn,
    staleTime: options?.staleTime ?? 5 * 60 * 1000, // 5 minutes
    cacheTime: options?.cacheTime ?? 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: options?.refetchOnWindowFocus ?? false,
  };
}

// ============================================================================
// 导出单例实例
// ============================================================================

export const apiClient = new ApiClient();
export default apiClient;
