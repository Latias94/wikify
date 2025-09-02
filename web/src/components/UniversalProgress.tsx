/**
 * 通用进度显示组件
 * 支持所有类型的进度显示：索引、Wiki生成、RAG查询、研究等
 */

import React, { useEffect, useState } from 'react';
import { Progress } from '@/components/ui/progress';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { 
  AlertCircle, 
  CheckCircle2, 
  FileText, 
  Loader2, 
  X, 
  BookOpen, 
  Brain, 
  Search,
  Clock,
  Zap
} from 'lucide-react';

import { useProgressStore } from '@/store/progress-store';
import {
  ProgressState,
  ProgressType,
  ProgressStatus,
  ProgressDisplayConfig,
  ProgressCallbacks,
  IndexingProgressState,
  WikiGenerationProgressState,
  RagQueryProgressState,
  ResearchProgressState
} from '@/types/progress';

// ============================================================================
// 组件属性接口
// ============================================================================

interface UniversalProgressProps {
  progressId?: string;
  repositoryId?: string;
  type?: ProgressType;
  config?: Partial<ProgressDisplayConfig>;
  callbacks?: ProgressCallbacks;
  className?: string;
}

// ============================================================================
// 默认配置
// ============================================================================

const defaultConfig: ProgressDisplayConfig = {
  showDetails: true,
  showTimeEstimate: true,
  showCancelButton: true,
  variant: "card",
  size: "md",
  animated: true,
};

// ============================================================================
// 主组件
// ============================================================================

export function UniversalProgress({
  progressId,
  repositoryId,
  type,
  config = {},
  callbacks = {},
  className,
}: UniversalProgressProps) {
  const finalConfig = { ...defaultConfig, ...config };
  
  // 状态管理
  const { 
    getProgress, 
    getProgressByRepository, 
    getProgressByType,
    cancelProgress 
  } = useProgressStore();
  
  const [progressStates, setProgressStates] = useState<ProgressState[]>([]);

  // ============================================================================
  // 数据获取逻辑
  // ============================================================================

  useEffect(() => {
    const updateStates = () => {
      let states: ProgressState[] = [];

      if (progressId) {
        // 获取特定进度
        const state = getProgress(progressId);
        states = state ? [state] : [];
      } else if (repositoryId && type) {
        // 获取特定仓库的特定类型进度
        states = getProgressByRepository(repositoryId).filter(s => s.type === type);
      } else if (repositoryId) {
        // 获取特定仓库的所有进度
        states = getProgressByRepository(repositoryId);
      } else if (type) {
        // 获取特定类型的所有进度
        states = getProgressByType(type);
      }

      setProgressStates(states);
    };

    // 初始加载
    updateStates();

    // 订阅更新
    const unsubscribe = useProgressStore.subscribe(() => {
      updateStates();
    });

    return unsubscribe;
  }, [progressId, repositoryId, type, getProgress, getProgressByRepository, getProgressByType]);

  // ============================================================================
  // 事件处理
  // ============================================================================

  useEffect(() => {
    progressStates.forEach(state => {
      switch (state.status) {
        case "running":
          if (state.startTime && !state.endTime) {
            callbacks.onStart?.(state);
          } else {
            callbacks.onProgress?.(state);
          }
          break;
        case "completed":
          callbacks.onComplete?.(state);
          break;
        case "error":
          callbacks.onError?.(state);
          break;
        case "cancelled":
          callbacks.onCancel?.(state);
          break;
      }
    });
  }, [progressStates, callbacks]);

  // ============================================================================
  // 工具函数
  // ============================================================================

  const getTypeIcon = (type: ProgressType) => {
    const iconClass = `h-4 w-4 ${finalConfig.animated ? 'transition-all duration-200' : ''}`;
    
    switch (type) {
      case "indexing":
        return <FileText className={iconClass} />;
      case "wiki_generation":
        return <BookOpen className={iconClass} />;
      case "rag_query":
        return <Search className={iconClass} />;
      case "research":
        return <Brain className={iconClass} />;
      default:
        return <Loader2 className={`${iconClass} animate-spin`} />;
    }
  };

  const getStatusIcon = (status: ProgressStatus) => {
    const iconClass = `h-4 w-4 ${finalConfig.animated ? 'transition-all duration-200' : ''}`;
    
    switch (status) {
      case "connecting":
      case "running":
        return <Loader2 className={`${iconClass} animate-spin`} />;
      case "completed":
        return <CheckCircle2 className={`${iconClass} text-green-500`} />;
      case "error":
        return <AlertCircle className={`${iconClass} text-red-500`} />;
      case "cancelled":
        return <X className={`${iconClass} text-gray-500`} />;
      default:
        return <Loader2 className={`${iconClass} animate-spin`} />;
    }
  };

  const getTypeLabel = (type: ProgressType) => {
    switch (type) {
      case "indexing":
        return "Indexing";
      case "wiki_generation":
        return "Wiki Generation";
      case "rag_query":
        return "RAG Query";
      case "research":
        return "Research";
      default:
        return "Processing";
    }
  };

  const getStatusText = (state: ProgressState) => {
    switch (state.status) {
      case "connecting":
        return "Connecting...";
      case "running":
        return getRunningText(state);
      case "completed":
        return "Completed";
      case "error":
        return "Failed";
      case "cancelled":
        return "Cancelled";
      default:
        return "Unknown";
    }
  };

  const getRunningText = (state: ProgressState) => {
    switch (state.type) {
      case "indexing":
        return "Indexing files...";
      case "wiki_generation":
        return "Generating wiki...";
      case "rag_query":
        return "Processing query...";
      case "research":
        return "Researching...";
      default:
        return "Processing...";
    }
  };

  const getProgressDetails = (state: ProgressState) => {
    switch (state.type) {
      case "indexing":
        const indexingState = state as IndexingProgressState;
        return {
          primary: `${indexingState.filesProcessed || 0} / ${indexingState.totalFiles || 0} files`,
          secondary: indexingState.currentFile,
          rate: indexingState.processingRate ? `${indexingState.processingRate.toFixed(1)} files/s` : undefined,
        };
      case "wiki_generation":
        const wikiState = state as WikiGenerationProgressState;
        return {
          primary: `Step ${wikiState.completedSteps || 0} / ${wikiState.totalSteps || 0}`,
          secondary: wikiState.currentStep,
          extra: wikiState.stepDetails,
        };
      case "rag_query":
        const ragState = state as RagQueryProgressState;
        return {
          primary: `Phase: ${ragState.currentPhase || 'unknown'}`,
          secondary: ragState.phaseDetails,
          extra: ragState.tokensGenerated ? `${ragState.tokensGenerated} tokens` : undefined,
        };
      case "research":
        const researchState = state as ResearchProgressState;
        return {
          primary: `Stage ${researchState.completedStages || 0} / ${researchState.totalStages || 0}`,
          secondary: researchState.currentStage,
          extra: researchState.documentsProcessed ?
            `${researchState.documentsProcessed} / ${researchState.totalDocuments || 0} docs` : undefined,
        };
      default:
        return {
          primary: `${Math.round(state.progress * 100)}%`,
          secondary: undefined,
        };
    }
  };

  const getDuration = (state: ProgressState) => {
    if (!state.startTime) return null;
    const endTime = state.endTime || new Date();
    const duration = Math.round((endTime.getTime() - state.startTime.getTime()) / 1000);
    return duration;
  };

  const formatDuration = (seconds: number) => {
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  };

  const handleCancel = (state: ProgressState) => {
    cancelProgress(state.id);
    callbacks.onCancel?.(state);
  };

  // ============================================================================
  // 渲染逻辑
  // ============================================================================

  if (progressStates.length === 0) {
    return null;
  }

  // 如果只有一个进度状态，直接渲染
  if (progressStates.length === 1) {
    return renderSingleProgress(progressStates[0]);
  }

  // 多个进度状态，渲染列表
  return (
    <div className={`space-y-2 ${className}`}>
      {progressStates.map(state => renderSingleProgress(state, true))}
    </div>
  );

  function renderSingleProgress(state: ProgressState, isInList = false) {
    const progressPercentage = Math.round(state.progress * 100);
    const duration = getDuration(state);
    const details = getProgressDetails(state);

    // 内联模式
    if (finalConfig.variant === "inline" || isInList) {
      return (
        <div key={state.id} className={`${isInList ? '' : className}`}>
          {/* 标题行 */}
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center space-x-2">
              {getTypeIcon(state.type)}
              <span className="text-sm font-medium">{getTypeLabel(state.type)}</span>
              <Badge variant={state.status === "error" ? "destructive" : "default"} className="text-xs">
                {getStatusText(state)}
              </Badge>
            </div>
            {finalConfig.showCancelButton && state.status === "running" && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleCancel(state)}
                className="h-6 text-xs"
              >
                Cancel
              </Button>
            )}
          </div>

          {/* 进度条 */}
          <div className="space-y-1">
            <div className="flex justify-between text-xs">
              <span>{details.primary}</span>
              <span>{progressPercentage}%</span>
            </div>
            <Progress
              value={progressPercentage}
              className="h-1.5"
            />
          </div>

          {/* 详细信息 */}
          {finalConfig.showDetails && (
            <div className="mt-2 space-y-1">
              {details.secondary && (
                <div className="text-xs text-muted-foreground truncate">
                  {details.secondary}
                </div>
              )}
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>{details.extra || details.rate}</span>
                {finalConfig.showTimeEstimate && duration && (
                  <span>{formatDuration(duration)}</span>
                )}
              </div>
            </div>
          )}

          {/* 错误信息 */}
          {state.status === "error" && state.error && (
            <div className="mt-2 p-2 bg-red-50 border border-red-200 rounded text-xs text-red-700">
              <div className="flex items-start space-x-1">
                <AlertCircle className="h-3 w-3 text-red-500 mt-0.5 flex-shrink-0" />
                <div>{state.error}</div>
              </div>
            </div>
          )}
        </div>
      );
    }

    // 卡片模式
    return (
      <Card key={state.id} className={className}>
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              {getStatusIcon(state.status)}
              <CardTitle className="text-lg">{getTypeLabel(state.type)}</CardTitle>
            </div>
            <div className="flex items-center space-x-2">
              <Badge variant={state.status === "error" ? "destructive" : "default"}>
                {getStatusText(state)}
              </Badge>
              {finalConfig.showCancelButton && state.status === "running" && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handleCancel(state)}
                  className="h-8"
                >
                  Cancel
                </Button>
              )}
            </div>
          </div>
          {repositoryId && (
            <CardDescription>Repository: {repositoryId}</CardDescription>
          )}
        </CardHeader>

        <CardContent className="space-y-4">
          {/* 进度条 */}
          <div className="space-y-2">
            <div className="flex justify-between text-sm">
              <span>Progress</span>
              <span>{progressPercentage}%</span>
            </div>
            <Progress value={progressPercentage} className="h-2" />
          </div>

          {/* 详细信息 */}
          {finalConfig.showDetails && state.status === "running" && (
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span>Status</span>
                <span>{details.primary}</span>
              </div>
              {details.secondary && (
                <div className="text-sm text-muted-foreground">
                  <span className="font-medium">Current:</span>{' '}
                  <code className="text-xs bg-muted px-1 py-0.5 rounded">
                    {details.secondary}
                  </code>
                </div>
              )}
              {details.extra && (
                <div className="text-sm text-muted-foreground">
                  {details.extra}
                </div>
              )}
            </div>
          )}

          {/* 完成信息 */}
          {state.status === "completed" && (
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span>Result</span>
                <span className="font-medium">{details.primary}</span>
              </div>
              {finalConfig.showTimeEstimate && duration && (
                <div className="flex justify-between">
                  <span>Duration</span>
                  <span className="font-medium">{formatDuration(duration)}</span>
                </div>
              )}
            </div>
          )}

          {/* 错误信息 */}
          {state.status === "error" && state.error && (
            <div className="p-3 bg-red-50 border border-red-200 rounded-md">
              <div className="flex items-start space-x-2">
                <AlertCircle className="h-4 w-4 text-red-500 mt-0.5 flex-shrink-0" />
                <div className="text-sm text-red-700">
                  <div className="font-medium">Operation failed</div>
                  <div className="mt-1">{state.error}</div>
                </div>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    );
  }
}

export default UniversalProgress;

// ============================================================================
// 进度管理面板组件
// ============================================================================

interface ProgressPanelProps {
  repositoryId?: string;
  className?: string;
}

export function ProgressPanel({ repositoryId, className }: ProgressPanelProps) {
  const { getAllProgress, getProgressByRepository, getProgressStats } = useProgressStore();
  const [stats, setStats] = useState(getProgressStats());
  const [progressStates, setProgressStates] = useState<ProgressState[]>([]);

  useEffect(() => {
    const updateData = () => {
      setStats(getProgressStats());
      setProgressStates(repositoryId ? getProgressByRepository(repositoryId) : getAllProgress());
    };

    updateData();
    const unsubscribe = useProgressStore.subscribe(updateData);
    return unsubscribe;
  }, [repositoryId, getAllProgress, getProgressByRepository, getProgressStats]);

  if (progressStates.length === 0) {
    return null;
  }

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg flex items-center gap-2">
            <Zap className="h-5 w-5" />
            Active Progress
          </CardTitle>
          <div className="flex items-center gap-2">
            {stats.running > 0 && (
              <Badge variant="default" className="text-xs">
                {stats.running} running
              </Badge>
            )}
            <Badge variant="secondary" className="text-xs">
              {stats.total} total
            </Badge>
          </div>
        </div>
        {repositoryId && (
          <CardDescription>Repository: {repositoryId}</CardDescription>
        )}
      </CardHeader>

      <CardContent className="space-y-3">
        {progressStates.map(state => (
          <UniversalProgress
            key={state.id}
            progressId={state.id}
            config={{
              variant: "inline",
              showDetails: true,
              showTimeEstimate: true,
              showCancelButton: true,
            }}
          />
        ))}
      </CardContent>
    </Card>
  );
}
