/**
 * 进度显示相关类型定义
 */

// ============================================================================
// 进度类型枚举
// ============================================================================

export type ProgressType = "indexing" | "wiki_generation" | "rag_query" | "research";

export type ProgressStatus = 
  | "idle" 
  | "connecting" 
  | "running" 
  | "completed" 
  | "error" 
  | "cancelled";

// ============================================================================
// 基础进度状态
// ============================================================================

export interface BaseProgressState {
  id: string;
  type: ProgressType;
  status: ProgressStatus;
  progress: number; // 0.0-1.0
  startTime?: Date;
  endTime?: Date;
  error?: string;
}

// ============================================================================
// 索引进度状态
// ============================================================================

export interface IndexingProgressState extends BaseProgressState {
  type: "indexing";
  repositoryId: string;
  currentFile?: string;
  filesProcessed: number;
  totalFiles: number;
  processingRate?: number; // 文件/秒
}

// ============================================================================
// Wiki 生成进度状态
// ============================================================================

export interface WikiGenerationProgressState extends BaseProgressState {
  type: "wiki_generation";
  repositoryId: string;
  currentStep: string;
  totalSteps: number;
  completedSteps: number;
  stepDetails?: string;
  wikiId?: string;
  pagesCount?: number;
  sectionsCount?: number;
}

// ============================================================================
// RAG 查询进度状态
// ============================================================================

export interface RagQueryProgressState extends BaseProgressState {
  type: "rag_query";
  repositoryId: string;
  queryId: string;
  currentPhase: "embedding" | "retrieval" | "generation" | "streaming";
  phaseDetails?: string;
  isStreaming?: boolean;
  tokensGenerated?: number;
}

// ============================================================================
// 研究进度状态
// ============================================================================

export interface ResearchProgressState extends BaseProgressState {
  type: "research";
  repositoryId: string;
  researchId: string;
  currentStage: string;
  totalStages: number;
  completedStages: number;
  stageDetails?: string;
  documentsProcessed?: number;
  totalDocuments?: number;
}

// ============================================================================
// 联合进度状态类型
// ============================================================================

export type ProgressState = 
  | IndexingProgressState 
  | WikiGenerationProgressState 
  | RagQueryProgressState 
  | ResearchProgressState;

// ============================================================================
// 进度显示配置
// ============================================================================

export interface ProgressDisplayConfig {
  showDetails: boolean;
  showTimeEstimate: boolean;
  showCancelButton: boolean;
  variant: "card" | "inline" | "minimal";
  size: "sm" | "md" | "lg";
  animated: boolean;
}

// ============================================================================
// 进度事件回调
// ============================================================================

export interface ProgressCallbacks {
  onStart?: (state: ProgressState) => void;
  onProgress?: (state: ProgressState) => void;
  onComplete?: (state: ProgressState) => void;
  onError?: (state: ProgressState) => void;
  onCancel?: (state: ProgressState) => void;
}

// ============================================================================
// 进度管理器接口
// ============================================================================

export interface ProgressManager {
  // 状态管理
  getProgress: (id: string) => ProgressState | undefined;
  getAllProgress: () => ProgressState[];
  getProgressByType: (type: ProgressType) => ProgressState[];
  getProgressByRepository: (repositoryId: string) => ProgressState[];
  
  // 进度操作
  startProgress: (state: Omit<ProgressState, "id" | "startTime">) => string;
  updateProgress: (id: string, updates: Partial<ProgressState>) => void;
  completeProgress: (id: string, result?: any) => void;
  errorProgress: (id: string, error: string) => void;
  cancelProgress: (id: string) => void;
  clearProgress: (id: string) => void;
  clearAllProgress: () => void;
  
  // 事件监听
  subscribe: (callback: (states: ProgressState[]) => void) => () => void;
  subscribeToProgress: (id: string, callback: (state: ProgressState) => void) => () => void;
}

// ============================================================================
// 进度组件属性
// ============================================================================

export interface ProgressComponentProps {
  progressId?: string;
  repositoryId?: string;
  type?: ProgressType;
  config?: Partial<ProgressDisplayConfig>;
  callbacks?: ProgressCallbacks;
  className?: string;
}

// ============================================================================
// 进度统计信息
// ============================================================================

export interface ProgressStats {
  total: number;
  running: number;
  completed: number;
  failed: number;
  cancelled: number;
  byType: Record<ProgressType, number>;
  byRepository: Record<string, number>;
}

// ============================================================================
// 进度历史记录
// ============================================================================

export interface ProgressHistoryEntry {
  id: string;
  type: ProgressType;
  repositoryId?: string;
  startTime: Date;
  endTime?: Date;
  duration?: number; // 毫秒
  status: ProgressStatus;
  error?: string;
  metadata?: Record<string, any>;
}

// ============================================================================
// 进度通知配置
// ============================================================================

export interface ProgressNotificationConfig {
  enabled: boolean;
  showStart: boolean;
  showProgress: boolean;
  showComplete: boolean;
  showError: boolean;
  sound: boolean;
  desktop: boolean;
  progressInterval?: number; // 进度通知间隔（秒）
}
