/**
 * API 数据类型定义
 * 与后端 Rust API 保持一致
 */

// ============================================================================
// 基础类型
// ============================================================================

export type RepoType = "local" | "git" | "github";
export type RepositoryStatus =
  | "created"
  | "indexing"
  | "indexed"
  | "failed"
  | "archived";

export type WikiStatus =
  | "not_generated"
  | "generating"
  | "generated"
  | "failed";
export type MessageRole = "user" | "assistant" | "system";

// ============================================================================
// 仓库相关类型
// ============================================================================

/**
 * 仓库信息
 */
export interface Repository {
  id: string;
  name: string;
  description?: string;
  repo_path: string;
  repo_type: RepoType;
  status: RepositoryStatus;
  wiki_status?: WikiStatus;
  created_at: string;
  last_indexed_at?: string;
  wiki_generated_at?: string;
  metadata?: Record<string, any>;
}

/**
 * 初始化仓库请求 - 与后端 InitializeRepositoryRequest 对齐
 */
export interface InitializeRepositoryRequest {
  repository: string; // 仓库URL或本地路径
  repo_type?: string | null; // "github", "local", etc.
  access_token?: string | null;
  auto_generate_wiki?: boolean | null; // 是否在索引完成后自动生成wiki，默认为true
}

/**
 * 仓库初始化响应
 */
export interface InitializeRepositoryResponse {
  repository_id: string;
  status: string;
  message: string;
}

/**
 * 仓库列表响应
 */
export interface RepositoriesResponse {
  repositories: Repository[];
}

// ============================================================================
// 用户和会话相关类型
// ============================================================================

// ============================================================================
// 聊天相关类型
// ============================================================================

/**
 * 聊天消息
 */
export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: string;
  sources?: SourceDocument[];
}

/**
 * 源文档信息
 */
export interface SourceDocument {
  file_path: string;
  content: string;
  similarity_score: number;
  // 扩展字段：位置信息
  start_line?: number;
  end_line?: number;
  chunk_index?: number;
  // 元数据
  metadata?: Record<string, any>;
}

/**
 * 聊天查询请求
 */
export interface ChatQueryRequest {
  repository_id: string;
  question: string;
  context?: string;
}

/**
 * 聊天查询响应
 */
export interface ChatQueryResponse {
  answer: string;
  sources: SourceDocument[];
  repository_id: string;
  timestamp: string;
}

/**
 * 查询历史记录
 */
export interface QueryHistory {
  id: string;
  user_id: string;
  repository_id: string;
  question: string;
  answer: string;
  sources: SourceDocument[];
  created_at: string;
  response_time_ms?: number;
  similarity_threshold?: number;
  chunks_retrieved?: number;
}

/**
 * 查询历史响应
 */
export interface QueryHistoryResponse {
  history: QueryHistory[];
}

// ============================================================================
// Wiki 相关类型
// ============================================================================

/**
 * Wiki 页面
 */
export interface WikiPage {
  id: string;
  title: string;
  content: string;
  description: string;
  importance: "Critical" | "High" | "Medium" | "Low";
  file_paths: string[];
  related_pages: string[];
  parent_section?: string;
  tags: string[];
  reading_time: number;
  generated_at: string;
  source_documents: DocumentInfo[];
}

/**
 * 文档信息
 */
export interface DocumentInfo {
  path: string;
  title: string;
  relevance_score: number;
}

/**
 * Wiki 章节
 */
export interface WikiSection {
  id: string;
  title: string;
  description: string;
  pages: string[]; // 包含的页面ID列表
  subsections: string[]; // 子章节ID列表
  importance: "Critical" | "High" | "Medium" | "Low";
  order: number;
}

/**
 * Wiki 结构
 */
export interface WikiStructure {
  id: string;
  title: string;
  description: string;
  pages: WikiPage[];
  sections: WikiSection[];
}

/**
 * Wiki 生成配置
 */
export interface WikiGenerationConfig {
  language?: string;
  max_pages?: number;
  include_diagrams?: boolean;
  comprehensive_view?: boolean;
}

/**
 * Wiki 生成请求
 */
export interface GenerateWikiRequest {
  repository_id: string;
  config: WikiGenerationConfig;
}

/**
 * Wiki 生成响应
 */
export interface GenerateWikiResponse {
  wiki_id: string;
  status: string;
  pages_count: number;
  sections_count: number;
}

// ============================================================================
// 文件相关类型
// ============================================================================

/**
 * 文件树节点
 */
export interface FileTreeNode {
  name: string;
  path: string;
  type: "file" | "directory";
  size?: number;
  children?: FileTreeNode[];
}

/**
 * 文件内容响应
 */
export interface FileContentResponse {
  path: string;
  content: string;
  file_type: string;
  size: number;
}

// ============================================================================
// 智能研究相关类型
// ============================================================================

/**
 * 研究阶段类型
 */
export type ResearchStageType = "plan" | "update" | "conclusion";

/**
 * 研究阶段
 */
export interface ResearchStage {
  id: string;
  title: string;
  content: string;
  iteration: number;
  type: ResearchStageType;
  timestamp: string;
  confidence?: number;
}

/**
 * 深度研究请求
 */
export interface DeepResearchRequest {
  repository_id: string;
  query: string;
  max_iterations?: number;
  research_strategy?: "comprehensive" | "focused" | "exploratory";
}

/**
 * 深度研究响应
 */
export interface DeepResearchResponse {
  research_id: string;
  repository_id: string;
  status: "planning" | "researching" | "completed" | "failed";
  current_iteration: number;
  max_iterations: number;
  stages: ResearchStage[];
  progress: {
    current_stage: string;
    completion_percentage: number;
    estimated_time_remaining?: number;
  };
  final_conclusion?: string;
  confidence_score?: number;
}

/**
 * 研究进度更新
 */
export interface ResearchProgressUpdate {
  research_id: string;
  stage: ResearchStage;
  progress: {
    current_stage: string;
    completion_percentage: number;
    estimated_time_remaining?: number;
  };
  is_complete: boolean;
}

// ============================================================================
// 系统相关类型
// ============================================================================

/**
 * 健康检查响应
 */
export interface HealthResponse {
  status: "healthy" | "unhealthy";
  timestamp: string;
  version: string;
}

/**
 * 配置信息
 */
export interface Config {
  llm: {
    provider: string;
    model: string;
    api_key?: string;
  };
  embedding: {
    provider: string;
    model: string;
    api_key?: string;
  };
  repository: {
    max_size_mb: number;
    excluded_dirs: string[];
  };
}

// ============================================================================
// 认证相关类型
// ============================================================================

/**
 * 权限模式
 */
export type AuthMode = "open" | "private" | "enterprise";

/**
 * 用户权限
 */
export type Permission =
  | "Query"
  | "GenerateWiki"
  | "DeepResearch"
  | "Export"
  | "Admin";

/**
 * 用户类型
 */
export type UserType = "Anonymous" | "Registered" | "Admin";

/**
 * 认证状态响应
 */
export interface AuthStatusResponse {
  auth_mode: AuthMode;
  auth_required: boolean;
  registration_enabled: boolean;
  features: {
    research_engine: boolean;
    wiki_generation: boolean;
    multi_language: boolean;
  };
}

/**
 * 用户注册请求
 */
export interface RegisterRequest {
  username: string;
  email: string;
  password: string;
  display_name?: string;
}

/**
 * 用户登录请求
 */
export interface LoginRequest {
  username: string;
  password: string;
}

/**
 * 认证响应
 */
export interface AuthResponse {
  user: UserInfo;
  tokens: TokenPair;
}

/**
 * 用户信息
 */
export interface UserInfo {
  id: string;
  username: string;
  email?: string;
  display_name?: string;
  user_type: UserType;
  permissions: Permission[];
  is_admin: boolean;
  created_at: string;
  last_login?: string;
}

/**
 * Token 对
 */
export interface TokenPair {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  token_type: string;
}

/**
 * Token 刷新请求
 */
export interface RefreshTokenRequest {
  refresh_token: string;
}

/**
 * API Key 创建请求
 */
export interface CreateApiKeyRequest {
  name: string;
  permissions: Permission[];
  expires_at?: string;
}

/**
 * API Key 响应
 */
export interface ApiKeyResponse {
  id: string;
  key?: string; // 只在创建时返回
  name: string;
  permissions: Permission[];
  created_at: string;
  expires_at?: string;
  last_used?: string;
}

/**
 * API Key 列表响应
 */
export interface ApiKeysResponse {
  api_keys: ApiKeyResponse[];
}

// ============================================================================
// 错误类型
// ============================================================================

/**
 * API 错误响应
 */
export interface ApiError {
  error: string;
  message: string;
  details?: Record<string, any>;
}

// ============================================================================
// 通用响应类型
// ============================================================================

/**
 * 通用 API 响应包装器
 */
export interface ApiResponse<T = any> {
  data?: T;
  error?: ApiError;
  success: boolean;
}

/**
 * 分页参数
 */
export interface PaginationParams {
  page?: number;
  limit?: number;
  sort?: string;
  order?: "asc" | "desc";
}

/**
 * 分页响应
 */
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  limit: number;
  has_next: boolean;
  has_prev: boolean;
}
