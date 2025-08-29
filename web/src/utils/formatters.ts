/**
 * 格式化工具函数
 */

import { format, formatDistanceToNow, isValid, parseISO } from "date-fns";
import { zhCN } from "date-fns/locale";

// ============================================================================
// 日期格式化
// ============================================================================

/**
 * 格式化日期为相对时间
 */
export function formatRelativeTime(date: string | Date): string {
  try {
    const dateObj = typeof date === "string" ? parseISO(date) : date;

    if (!isValid(dateObj)) {
      return "Invalid date";
    }

    return formatDistanceToNow(dateObj, {
      addSuffix: true,
      locale: zhCN,
    });
  } catch (error) {
    console.error("Error formatting relative time:", error);
    return "Unknown time";
  }
}

/**
 * 格式化日期为标准格式
 */
export function formatDate(
  date: string | Date,
  formatStr: string = "yyyy-MM-dd"
): string {
  try {
    const dateObj = typeof date === "string" ? parseISO(date) : date;

    if (!isValid(dateObj)) {
      return "Invalid date";
    }

    return format(dateObj, formatStr, { locale: zhCN });
  } catch (error) {
    console.error("Error formatting date:", error);
    return "Invalid date";
  }
}

/**
 * 格式化日期时间
 */
export function formatDateTime(date: string | Date): string {
  return formatDate(date, "yyyy-MM-dd HH:mm:ss");
}

/**
 * 格式化时间
 */
export function formatTime(date: string | Date): string {
  return formatDate(date, "HH:mm:ss");
}

// ============================================================================
// 文件大小格式化
// ============================================================================

/**
 * 格式化文件大小
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return "0 B";

  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

// ============================================================================
// 数字格式化
// ============================================================================

/**
 * 格式化数字为千分位
 */
export function formatNumber(num: number): string {
  return new Intl.NumberFormat("zh-CN").format(num);
}

/**
 * 格式化百分比
 */
export function formatPercentage(value: number, total: number): string {
  if (total === 0) return "0%";
  const percentage = (value / total) * 100;
  return `${Math.round(percentage)}%`;
}

/**
 * 格式化进度百分比
 */
export function formatProgress(progress: number): string {
  return `${Math.round(Math.max(0, Math.min(100, progress)))}%`;
}

// ============================================================================
// 文本格式化
// ============================================================================

/**
 * 截断文本
 */
export function truncateText(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return `${text.slice(0, maxLength)}...`;
}

/**
 * 格式化文件路径
 */
export function formatFilePath(path: string, maxLength: number = 50): string {
  if (path.length <= maxLength) return path;

  const parts = path.split("/");
  if (parts.length <= 2) return truncateText(path, maxLength);

  const fileName = parts[parts.length - 1];
  const firstPart = parts[0];

  if (fileName.length + firstPart.length + 5 <= maxLength) {
    return `${firstPart}/.../${fileName}`;
  }

  return `.../${fileName}`;
}

/**
 * 格式化仓库名称
 */
export function formatRepositoryName(name: string): string {
  return name
    .replace(/[_-]/g, " ")
    .replace(/\b\w/g, (l) => l.toUpperCase())
    .trim();
}

/**
 * 格式化 Git URL
 */
export function formatGitUrl(url: string): string {
  // 移除 .git 后缀
  const cleanUrl = url.replace(/\.git$/, "");

  // 转换 SSH URL 为 HTTPS
  if (cleanUrl.startsWith("git@github.com:")) {
    return cleanUrl.replace("git@github.com:", "https://github.com/");
  }

  return cleanUrl;
}

// ============================================================================
// 状态格式化
// ============================================================================

/**
 * 格式化仓库状态
 */
export function formatRepositoryStatus(status: string): string {
  const statusMap: Record<string, string> = {
    created: "已创建",
    indexing: "索引中",
    indexed: "已索引",
    failed: "失败",
    archived: "已归档",
  };

  return statusMap[status] || status;
}

/**
 * 格式化仓库类型
 */
export function formatRepositoryType(type: string): string {
  const typeMap: Record<string, string> = {
    local: "本地",
    git: "Git",
    github: "GitHub",
  };

  return typeMap[type] || type;
}

/**
 * 格式化消息角色
 */
export function formatMessageRole(role: string): string {
  const roleMap: Record<string, string> = {
    user: "用户",
    assistant: "助手",
    system: "系统",
  };

  return roleMap[role] || role;
}

// ============================================================================
// 代码格式化
// ============================================================================

/**
 * 检测编程语言
 */
export function detectLanguage(filename: string): string {
  const ext = filename.split(".").pop()?.toLowerCase();

  const languageMap: Record<string, string> = {
    js: "javascript",
    jsx: "javascript",
    ts: "typescript",
    tsx: "typescript",
    py: "python",
    java: "java",
    cpp: "cpp",
    c: "c",
    h: "c",
    hpp: "cpp",
    cs: "csharp",
    php: "php",
    rb: "ruby",
    go: "go",
    rs: "rust",
    swift: "swift",
    kt: "kotlin",
    scala: "scala",
    clj: "clojure",
    hs: "haskell",
    ml: "ocaml",
    fs: "fsharp",
    elm: "elm",
    dart: "dart",
    lua: "lua",
    r: "r",
    sql: "sql",
    sh: "bash",
    bash: "bash",
    zsh: "zsh",
    fish: "fish",
    ps1: "powershell",
    bat: "batch",
    cmd: "batch",
    html: "html",
    xml: "xml",
    svg: "xml",
    vue: "vue",
    svelte: "svelte",
    css: "css",
    scss: "scss",
    sass: "sass",
    less: "less",
    styl: "stylus",
    json: "json",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    ini: "ini",
    conf: "ini",
    config: "ini",
    env: "bash",
    properties: "properties",
    md: "markdown",
    mdx: "markdown",
    rst: "rst",
    txt: "text",
    adoc: "asciidoc",
    org: "org",
  };

  return languageMap[ext || ""] || "text";
}

/**
 * 格式化代码块
 */
export function formatCodeBlock(code: string, language: string): string {
  return `\`\`\`${language}\n${code}\n\`\`\``;
}

// ============================================================================
// URL 格式化
// ============================================================================

/**
 * 格式化 API URL
 */
export function formatApiUrl(endpoint: string): string {
  const baseUrl =
    import.meta.env.VITE_API_BASE_URL || "http://localhost:8080/api";
  return `${baseUrl}${endpoint.startsWith("/") ? "" : "/"}${endpoint}`;
}

/**
 * 格式化 WebSocket URL
 */
export function formatWsUrl(endpoint: string): string {
  const baseUrl = import.meta.env.VITE_WS_BASE_URL || "ws://localhost:8080/ws";
  return `${baseUrl}${endpoint.startsWith("/") ? "" : "/"}${endpoint}`;
}

// ============================================================================
// 验证格式化
// ============================================================================

/**
 * 格式化错误消息
 */
export function formatErrorMessage(error: unknown): string {
  if (typeof error === "string") return error;

  if (error instanceof Error) return error.message;

  if (typeof error === "object" && error !== null) {
    if ("message" in error && typeof error.message === "string") {
      return error.message;
    }

    if ("error" in error && typeof error.error === "string") {
      return error.error;
    }
  }

  return "An unknown error occurred";
}

/**
 * 格式化验证错误
 */
export function formatValidationError(field: string, rule: string): string {
  const messages: Record<string, Record<string, string>> = {
    required: {
      default: "此字段为必填项",
    },
    minLength: {
      default: "输入内容过短",
    },
    maxLength: {
      default: "输入内容过长",
    },
    pattern: {
      default: "输入格式不正确",
    },
    email: {
      default: "请输入有效的邮箱地址",
    },
    url: {
      default: "请输入有效的URL地址",
    },
  };

  return messages[rule]?.[field] || messages[rule]?.default || "输入无效";
}

// ============================================================================
// 源文档格式化
// ============================================================================

/**
 * 估算代码块的行号范围
 * 基于内容的行数来估算
 */
export function estimateLineRange(
  content: string,
  chunkIndex?: number
): { start: number; end: number } {
  const lines = content.split("\n");
  const lineCount = lines.length;

  // 如果有 chunk_index，可以基于此做更好的估算
  // 这里简化处理，实际应该基于文件的总行数和 chunk 位置
  const estimatedStart = chunkIndex ? chunkIndex * 20 + 1 : 1;
  const estimatedEnd = estimatedStart + lineCount - 1;

  return {
    start: Math.max(1, estimatedStart),
    end: Math.max(estimatedStart, estimatedEnd),
  };
}

/**
 * 格式化行号范围显示
 */
export function formatLineRange(startLine?: number, endLine?: number): string {
  if (!startLine && !endLine) {
    return "";
  }

  if (startLine && endLine) {
    if (startLine === endLine) {
      return `第 ${startLine} 行`;
    }
    return `第 ${startLine}-${endLine} 行`;
  }

  if (startLine) {
    return `第 ${startLine}+ 行`;
  }

  return "";
}

/**
 * 生成 IDE 跳转链接
 * 支持 VS Code, WebStorm, Cursor 等
 */
export function generateIDELink(
  filePath: string,
  line?: number
): {
  vscode: string;
  webstorm: string;
  cursor: string;
  sublime: string;
} {
  const lineParam = line ? `:${line}` : "";
  const encodedPath = encodeURIComponent(filePath);

  return {
    vscode: `vscode://file/${filePath}${lineParam}`,
    webstorm: `webstorm://open?file=${encodedPath}&line=${line || 1}`,
    cursor: `cursor://file/${filePath}${lineParam}`,
    sublime: `subl://open?url=file://${filePath}&line=${line || 1}`,
  };
}

/**
 * 检查是否可以在浏览器中打开文件
 */
export function canOpenInBrowser(filePath: string): boolean {
  const webExtensions = [
    "html",
    "htm",
    "md",
    "txt",
    "json",
    "xml",
    "css",
    "js",
    "ts",
    "svg",
  ];
  const ext = filePath.split(".").pop()?.toLowerCase();
  return webExtensions.includes(ext || "");
}

/**
 * 检测 Git 仓库类型并生成浏览器链接
 */
export function generateGitBrowserLink(
  filePath: string,
  line?: number,
  repositoryUrl?: string
): { url: string; platform: string } | null {
  if (!repositoryUrl) return null;

  // 清理 URL
  const cleanUrl = repositoryUrl.replace(/\.git$/, "").replace(/\/$/, "");

  // 检测平台
  let platform = "";
  let baseUrl = "";

  if (cleanUrl.includes("github.com")) {
    platform = "GitHub";
    baseUrl = cleanUrl;
  } else if (cleanUrl.includes("gitlab.com") || cleanUrl.includes("gitlab.")) {
    platform = "GitLab";
    baseUrl = cleanUrl;
  } else if (cleanUrl.includes("bitbucket.org")) {
    platform = "Bitbucket";
    baseUrl = cleanUrl;
  } else if (cleanUrl.includes("gitea.") || cleanUrl.includes("gitea.io")) {
    platform = "Gitea";
    baseUrl = cleanUrl;
  } else {
    // 尝试通用的 Git 平台格式
    platform = "Git";
    baseUrl = cleanUrl;
  }

  // 生成文件链接
  const cleanFilePath = filePath.replace(/^\/+/, ""); // 移除开头的斜杠
  let fileUrl = "";

  switch (platform) {
    case "GitHub":
      fileUrl = `${baseUrl}/blob/main/${cleanFilePath}`;
      if (line) fileUrl += `#L${line}`;
      break;

    case "GitLab":
      fileUrl = `${baseUrl}/-/blob/main/${cleanFilePath}`;
      if (line) fileUrl += `#L${line}`;
      break;

    case "Bitbucket":
      fileUrl = `${baseUrl}/src/main/${cleanFilePath}`;
      if (line) fileUrl += `#lines-${line}`;
      break;

    case "Gitea":
      fileUrl = `${baseUrl}/src/branch/main/${cleanFilePath}`;
      if (line) fileUrl += `#L${line}`;
      break;

    default:
      // 通用格式，大多数 Git 平台都支持
      fileUrl = `${baseUrl}/blob/main/${cleanFilePath}`;
      if (line) fileUrl += `#L${line}`;
  }

  return { url: fileUrl, platform };
}

/**
 * 检查是否为远程 Git 仓库
 */
export function isRemoteRepository(repositoryUrl?: string): boolean {
  if (!repositoryUrl) return false;

  return (
    repositoryUrl.startsWith("http://") ||
    repositoryUrl.startsWith("https://") ||
    repositoryUrl.startsWith("git@")
  );
}

/**
 * 格式化文件路径为可复制的格式
 */
export function formatCopyablePath(filePath: string): string {
  // 移除可能的前缀路径，只保留相对路径
  return filePath.replace(/^.*\/(?=\w)/, "");
}

/**
 * 生成文件信息摘要
 */
export function generateFileInfo(
  filePath: string,
  startLine?: number,
  endLine?: number,
  chunkIndex?: number
): string {
  const fileName = filePath.split("/").pop() || filePath;
  const lineInfo = formatLineRange(startLine, endLine);
  const chunkInfo = chunkIndex !== undefined ? ` (块 ${chunkIndex + 1})` : "";

  return `${fileName}${lineInfo ? ` - ${lineInfo}` : ""}${chunkInfo}`;
}
