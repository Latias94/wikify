/**
 * UI 组件相关类型定义
 */

import { Repository, Session, ChatMessage as ApiChatMessage, SourceDocument } from './api';

// ============================================================================
// 主题相关类型
// ============================================================================

export type Theme = 'light' | 'dark' | 'system';

export interface ThemeConfig {
  theme: Theme;
  systemTheme?: 'light' | 'dark';
}

// ============================================================================
// 通知相关类型
// ============================================================================

export type NotificationType = 'success' | 'error' | 'warning' | 'info';

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  description?: string;
  duration?: number;
  action?: {
    label: string;
    onClick: () => void;
  };
}

// ============================================================================
// 加载状态类型
// ============================================================================

export interface LoadingState {
  isLoading: boolean;
  error?: string;
  progress?: number;
}

export interface AsyncState<T = any> extends LoadingState {
  data?: T;
  lastUpdated?: Date;
}

// ============================================================================
// 仓库管理相关类型
// ============================================================================

export interface RepositoryCardProps {
  repository: Repository;
  onChat: (repository: Repository) => void;
  onRefresh: (repository: Repository) => void;
  onDelete: (repository: Repository) => void;
}

export interface AddRepositoryFormData {
  repo_path: string;
  repo_type: 'local' | 'remote';
  name?: string;
  description?: string;
}

export interface RepositoryStats {
  totalFiles: number;
  indexedFiles: number;
  lastIndexed?: string;
  size?: string;
}

// ============================================================================
// 聊天界面相关类型
// ============================================================================

export interface UIChatMessage extends Omit<ApiChatMessage, 'timestamp'> {
  timestamp: Date;
  isStreaming?: boolean;
  isError?: boolean;
  sources?: SourceDocument[];
}

export interface ChatInterfaceState {
  messages: UIChatMessage[];
  isLoading: boolean;
  isConnected: boolean;
  currentSession?: Session;
  error?: string;
}

export interface MessageBubbleProps {
  message: UIChatMessage;
  onCopy: (content: string) => void;
  onRetry?: (message: UIChatMessage) => void;
}

export interface ChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSend: (message: string) => void;
  disabled?: boolean;
  placeholder?: string;
  maxLength?: number;
}

// ============================================================================
// 会话管理相关类型
// ============================================================================

export interface SessionListProps {
  sessions: Session[];
  currentSession?: Session;
  onSelect: (session: Session) => void;
  onCreate: () => void;
  onDelete: (session: Session) => void;
  onRename: (session: Session, newName: string) => void;
}

export interface SessionCardProps {
  session: Session;
  isActive: boolean;
  onClick: () => void;
  onDelete: () => void;
  onRename: (newName: string) => void;
}

// ============================================================================
// Wiki 相关类型
// ============================================================================

export interface WikiGenerationState {
  isGenerating: boolean;
  progress: number;
  currentStep: string;
  totalSteps: number;
  error?: string;
}

export interface WikiViewerProps {
  wikiId: string;
  title: string;
  content: string;
  onExport: (format: 'markdown' | 'html' | 'pdf') => void;
}

// ============================================================================
// 文件浏览器相关类型
// ============================================================================

export interface FileTreeProps {
  repositoryId: string;
  onFileSelect: (filePath: string) => void;
  selectedFile?: string;
}

export interface FileViewerProps {
  filePath: string;
  content: string;
  language?: string;
  onClose: () => void;
}

// ============================================================================
// 搜索相关类型
// ============================================================================

export interface SearchState {
  query: string;
  results: SearchResult[];
  isSearching: boolean;
  filters: SearchFilters;
}

export interface SearchResult {
  id: string;
  title: string;
  content: string;
  filePath: string;
  score: number;
  highlights: string[];
}

export interface SearchFilters {
  fileTypes: string[];
  dateRange?: {
    start: Date;
    end: Date;
  };
  repositories: string[];
}

// ============================================================================
// 设置相关类型
// ============================================================================

export interface UserSettings {
  theme: Theme;
  language: string;
  notifications: {
    enabled: boolean;
    sound: boolean;
    desktop: boolean;
  };
  chat: {
    showTimestamps: boolean;
    showSources: boolean;
    autoScroll: boolean;
    maxMessages: number;
  };
  editor: {
    fontSize: number;
    tabSize: number;
    wordWrap: boolean;
    lineNumbers: boolean;
  };
}

export interface SettingsFormProps {
  settings: UserSettings;
  onSave: (settings: UserSettings) => void;
  onReset: () => void;
}

// ============================================================================
// 导航相关类型
// ============================================================================

export interface NavigationItem {
  id: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  path: string;
  badge?: string | number;
  children?: NavigationItem[];
}

export interface BreadcrumbItem {
  label: string;
  path?: string;
  isActive?: boolean;
}

// ============================================================================
// 表单相关类型
// ============================================================================

export interface FormFieldProps<T = any> {
  name: string;
  label: string;
  value: T;
  onChange: (value: T) => void;
  error?: string;
  disabled?: boolean;
  required?: boolean;
  placeholder?: string;
  description?: string;
}

export interface FormState<T = Record<string, any>> {
  values: T;
  errors: Partial<Record<keyof T, string>>;
  touched: Partial<Record<keyof T, boolean>>;
  isSubmitting: boolean;
  isValid: boolean;
}

// ============================================================================
// 模态框相关类型
// ============================================================================

export interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  size?: 'sm' | 'md' | 'lg' | 'xl' | 'full';
  closeOnOverlayClick?: boolean;
  closeOnEscape?: boolean;
}

export interface ConfirmDialogProps extends ModalProps {
  message: string;
  confirmText?: string;
  cancelText?: string;
  variant?: 'default' | 'destructive';
  onConfirm: () => void;
}

// ============================================================================
// 数据表格相关类型
// ============================================================================

export interface TableColumn<T = any> {
  key: keyof T;
  title: string;
  width?: string | number;
  sortable?: boolean;
  render?: (value: any, record: T, index: number) => React.ReactNode;
}

export interface TableProps<T = any> {
  data: T[];
  columns: TableColumn<T>[];
  loading?: boolean;
  pagination?: {
    current: number;
    pageSize: number;
    total: number;
    onChange: (page: number, pageSize: number) => void;
  };
  selection?: {
    selectedRowKeys: string[];
    onChange: (selectedRowKeys: string[], selectedRows: T[]) => void;
  };
  onSort?: (key: string, direction: 'asc' | 'desc') => void;
}

// ============================================================================
// 工具类型
// ============================================================================

export type Optional<T, K extends keyof T> = Omit<T, K> & Partial<Pick<T, K>>;
export type RequiredFields<T, K extends keyof T> = T & Required<Pick<T, K>>;

export interface ComponentProps {
  className?: string;
  children?: React.ReactNode;
}

export interface WithLoading {
  loading?: boolean;
}

export interface WithError {
  error?: string | Error;
}
