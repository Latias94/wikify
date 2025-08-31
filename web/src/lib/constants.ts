/**
 * 应用常量定义
 */

// ============================================================================
// API 配置
// ============================================================================

export const API_CONFIG = {
  BASE_URL: import.meta.env.VITE_API_BASE_URL || "http://localhost:8080/api",
  WS_BASE_URL: import.meta.env.VITE_WS_BASE_URL || "ws://localhost:8080/ws",
  TIMEOUT: 30000,
  MAX_RETRIES: 3,
} as const;

// ============================================================================
// WebSocket 配置
// ============================================================================

export const WS_CONFIG = {
  RECONNECT_INTERVAL: 3000,
  MAX_RECONNECT_ATTEMPTS: 5,
  HEARTBEAT_INTERVAL: 30000,
  CONNECTION_TIMEOUT: 10000,
} as const;

// ============================================================================
// UI 配置
// ============================================================================

export const UI_CONFIG = {
  CHAT: {
    MAX_MESSAGES: 100,
    AUTO_SCROLL_THRESHOLD: 100,
    TYPING_INDICATOR_TIMEOUT: 3000,
  },
  REPOSITORY: {
    MAX_NAME_LENGTH: 100,
    MAX_DESCRIPTION_LENGTH: 500,
  },
  PAGINATION: {
    DEFAULT_PAGE_SIZE: 20,
    MAX_PAGE_SIZE: 100,
  },
} as const;

// ============================================================================
// 状态常量
// ============================================================================

export const REPOSITORY_STATUS = {
  CREATED: "created",
  INDEXING: "indexing",
  INDEXED: "indexed",
  FAILED: "failed",
  ARCHIVED: "archived",
} as const;

export const REPOSITORY_TYPE = {
  LOCAL: "local",
  GIT: "git",
  GITHUB: "github",
} as const;

export const MESSAGE_ROLE = {
  USER: "user",
  ASSISTANT: "assistant",
  SYSTEM: "system",
} as const;

export const WS_MESSAGE_TYPE = {
  CHAT: "Chat",
  CHAT_RESPONSE: "ChatResponse",
  CHAT_ERROR: "ChatError",
  WIKI_GENERATE: "WikiGenerate",
  WIKI_PROGRESS: "WikiProgress",
  WIKI_COMPLETE: "WikiComplete",
  WIKI_ERROR: "WikiError",
  INDEX_PROGRESS: "IndexProgress",
  INDEX_COMPLETE: "IndexComplete",
  INDEX_ERROR: "IndexError",
  PING: "Ping",
  PONG: "Pong",
} as const;

// ============================================================================
// 错误消息
// ============================================================================

export const ERROR_MESSAGES = {
  NETWORK_ERROR: "Network error: Unable to reach the server",
  TIMEOUT_ERROR: "Request timeout: The server took too long to respond",
  UNAUTHORIZED: "Unauthorized: Please check your credentials",
  FORBIDDEN: "Forbidden: You do not have permission to perform this action",
  NOT_FOUND: "Not found: The requested resource was not found",
  SERVER_ERROR: "Server error: An internal server error occurred",
  UNKNOWN_ERROR: "An unknown error occurred",

  // WebSocket 错误
  WS_CONNECTION_FAILED: "Failed to connect to chat service",
  WS_CONNECTION_LOST: "Connection to chat service was lost",
  WS_RECONNECT_FAILED: "Failed to reconnect to chat service",

  // 仓库相关错误
  REPOSITORY_ADD_FAILED: "Failed to add repository",
  REPOSITORY_DELETE_FAILED: "Failed to delete repository",
  REPOSITORY_NOT_READY: "Repository is not ready for chat",

  // 聊天相关错误
  CHAT_SEND_FAILED: "Failed to send message",
  CHAT_HISTORY_LOAD_FAILED: "Failed to load chat history",

  // 会话相关错误
  SESSION_CREATE_FAILED: "Failed to create chat session",
  SESSION_DELETE_FAILED: "Failed to delete session",
} as const;

// ============================================================================
// 成功消息
// ============================================================================

export const SUCCESS_MESSAGES = {
  REPOSITORY_ADDED: "Repository added successfully",
  REPOSITORY_DELETED: "Repository deleted successfully",
  SESSION_CREATED: "Chat session created successfully",
  SESSION_DELETED: "Session deleted successfully",
  WIKI_GENERATION_STARTED: "Wiki generation started",
  WIKI_GENERATED: "Wiki generated successfully",
  WIKI_EXPORTED: "Wiki exported successfully",
  INDEX_COMPLETED: "Repository indexing completed",
} as const;

// ============================================================================
// 路由路径
// ============================================================================

export const ROUTES = {
  HOME: "/",
  CHAT: "/chat/:repositoryId",
  REPOSITORY: "/repository/:repositoryId",
  WIKI: "/wiki/:repositoryId",
  SETTINGS: "/settings",
  NOT_FOUND: "/404",
} as const;

// ============================================================================
// 本地存储键
// ============================================================================

export const STORAGE_KEYS = {
  AUTH_TOKEN: "wikify_auth_token",
  USER_SETTINGS: "wikify_user_settings",
  THEME: "wikify_theme",
  SIDEBAR_COLLAPSED: "wikify_sidebar_collapsed",
  CHAT_DRAFT: "wikify_chat_draft_",
} as const;

// ============================================================================
// 文件类型
// ============================================================================

export const FILE_TYPES = {
  CODE: [
    "js",
    "jsx",
    "ts",
    "tsx",
    "py",
    "java",
    "cpp",
    "c",
    "h",
    "hpp",
    "cs",
    "php",
    "rb",
    "go",
    "rs",
    "swift",
    "kt",
    "scala",
    "clj",
    "hs",
    "ml",
    "fs",
    "elm",
    "dart",
    "lua",
    "r",
    "sql",
    "sh",
    "bash",
    "zsh",
    "fish",
    "ps1",
    "bat",
    "cmd",
  ],
  MARKUP: ["html", "xml", "svg", "jsx", "tsx", "vue", "svelte"],
  STYLE: ["css", "scss", "sass", "less", "styl"],
  CONFIG: [
    "json",
    "yaml",
    "yml",
    "toml",
    "ini",
    "conf",
    "config",
    "env",
    "properties",
    "plist",
  ],
  DOCUMENTATION: ["md", "mdx", "rst", "txt", "adoc", "org"],
  DATA: ["csv", "tsv", "json", "xml", "yaml", "yml"],
} as const;

// ============================================================================
// 主题配置
// ============================================================================

export const THEME_CONFIG = {
  LIGHT: "light",
  DARK: "dark",
  SYSTEM: "system",
} as const;

// ============================================================================
// 动画配置
// ============================================================================

export const ANIMATION_CONFIG = {
  DURATION: {
    FAST: 150,
    NORMAL: 300,
    SLOW: 500,
  },
  EASING: {
    EASE_IN: "ease-in",
    EASE_OUT: "ease-out",
    EASE_IN_OUT: "ease-in-out",
  },
} as const;

// ============================================================================
// 验证规则
// ============================================================================

export const VALIDATION_RULES = {
  REPOSITORY: {
    NAME: {
      MIN_LENGTH: 1,
      MAX_LENGTH: 100,
      PATTERN: /^[a-zA-Z0-9\-_\s]+$/,
    },
    DESCRIPTION: {
      MAX_LENGTH: 500,
    },
    PATH: {
      MIN_LENGTH: 1,
      MAX_LENGTH: 1000,
    },
  },
  CHAT: {
    MESSAGE: {
      MIN_LENGTH: 1,
      MAX_LENGTH: 10000,
    },
  },
  SESSION: {
    NAME: {
      MIN_LENGTH: 1,
      MAX_LENGTH: 100,
    },
  },
} as const;

// ============================================================================
// 导出类型
// ============================================================================

export type RepositoryStatus =
  (typeof REPOSITORY_STATUS)[keyof typeof REPOSITORY_STATUS];
export type RepositoryType =
  (typeof REPOSITORY_TYPE)[keyof typeof REPOSITORY_TYPE];
export type MessageRole = (typeof MESSAGE_ROLE)[keyof typeof MESSAGE_ROLE];
export type WSMessageType =
  (typeof WS_MESSAGE_TYPE)[keyof typeof WS_MESSAGE_TYPE];
export type Theme = (typeof THEME_CONFIG)[keyof typeof THEME_CONFIG];
