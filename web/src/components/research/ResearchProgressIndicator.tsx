/**
 * 研究进度指示器组件
 * 显示深度研究的进度和状态信息
 */

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import {
  Brain,
  Clock,
  Target,
  CheckCircle,
  AlertCircle,
  Loader2,
  TrendingUp,
} from 'lucide-react';

// ============================================================================
// 组件接口
// ============================================================================

interface ResearchProgressIndicatorProps {
  status: 'idle' | 'planning' | 'researching' | 'completed' | 'failed';
  currentIteration: number;
  maxIterations: number;
  progress: {
    current_stage: string;
    completion_percentage: number;
    estimated_time_remaining?: number;
  };
  confidenceScore?: number;
}

// ============================================================================
// 工具函数
// ============================================================================

const getStatusIcon = (status: string) => {
  switch (status) {
    case 'planning':
      return <Brain className="h-4 w-4 animate-pulse" />;
    case 'researching':
      return <Loader2 className="h-4 w-4 animate-spin" />;
    case 'completed':
      return <CheckCircle className="h-4 w-4 text-green-500" />;
    case 'failed':
      return <AlertCircle className="h-4 w-4 text-red-500" />;
    default:
      return <Target className="h-4 w-4" />;
  }
};

const getStatusColor = (status: string) => {
  switch (status) {
    case 'planning':
      return 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200';
    case 'researching':
      return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
    case 'completed':
      return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
    case 'failed':
      return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200';
    default:
      return 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200';
  }
};

const getStatusLabel = (status: string) => {
  switch (status) {
    case 'planning':
      return 'Planning Research';
    case 'researching':
      return 'Researching';
    case 'completed':
      return 'Completed';
    case 'failed':
      return 'Failed';
    default:
      return 'Idle';
  }
};

const formatTimeRemaining = (seconds: number) => {
  if (seconds < 60) {
    return `${Math.round(seconds)}s`;
  } else if (seconds < 3600) {
    return `${Math.round(seconds / 60)}m`;
  } else {
    return `${Math.round(seconds / 3600)}h`;
  }
};

// ============================================================================
// 迭代进度组件
// ============================================================================

interface IterationProgressProps {
  currentIteration: number;
  maxIterations: number;
}

const IterationProgress: React.FC<IterationProgressProps> = ({
  currentIteration,
  maxIterations,
}) => {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between text-sm">
        <span className="text-muted-foreground">Research Iterations</span>
        <span className="font-medium">
          {currentIteration} / {maxIterations}
        </span>
      </div>
      <div className="flex gap-1">
        {Array.from({ length: maxIterations }, (_, index) => (
          <div
            key={index}
            className={`h-2 flex-1 rounded-full transition-colors ${
              index < currentIteration
                ? 'bg-primary'
                : index === currentIteration
                ? 'bg-primary/50 animate-pulse'
                : 'bg-muted'
            }`}
          />
        ))}
      </div>
    </div>
  );
};

// ============================================================================
// 主组件
// ============================================================================

export const ResearchProgressIndicator: React.FC<ResearchProgressIndicatorProps> = ({
  status,
  currentIteration,
  maxIterations,
  progress,
  confidenceScore,
}) => {
  const isActive = status === 'planning' || status === 'researching';
  const isComplete = status === 'completed';
  const isFailed = status === 'failed';

  return (
    <Card className={`transition-all duration-200 ${isActive ? 'border-primary/50' : ''}`}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`p-2 rounded-full ${getStatusColor(status)}`}>
              {getStatusIcon(status)}
            </div>
            <div>
              <CardTitle className="text-lg">Research Progress</CardTitle>
              <CardDescription>
                {progress.current_stage || getStatusLabel(status)}
              </CardDescription>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Badge variant={isComplete ? 'default' : 'secondary'}>
              {getStatusLabel(status)}
            </Badge>
            {confidenceScore && (
              <Badge variant="outline" className="flex items-center gap-1">
                <TrendingUp className="h-3 w-3" />
                {Math.round(confidenceScore * 100)}%
              </Badge>
            )}
          </div>
        </div>
      </CardHeader>

      <Separator />

      <CardContent className="pt-4 space-y-4">
        {/* 整体进度条 */}
        <div className="space-y-2">
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">Overall Progress</span>
            <span className="font-medium">{Math.round(progress.completion_percentage)}%</span>
          </div>
          <Progress 
            value={progress.completion_percentage} 
            className="h-2"
          />
        </div>

        {/* 迭代进度 */}
        <IterationProgress
          currentIteration={currentIteration}
          maxIterations={maxIterations}
        />

        {/* 时间估计 */}
        {progress.estimated_time_remaining && isActive && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Clock className="h-4 w-4" />
            <span>
              Estimated time remaining: {formatTimeRemaining(progress.estimated_time_remaining)}
            </span>
          </div>
        )}

        {/* 详细状态信息 */}
        {isActive && (
          <div className="space-y-2">
            <div className="text-sm font-medium">Current Activity:</div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
              {status === 'planning' && (
                <>
                  <div className="flex items-center gap-2 p-2 bg-blue-50 dark:bg-blue-950 rounded">
                    <div className="w-2 h-2 bg-blue-500 rounded-full animate-pulse" />
                    <span>Analyzing research scope</span>
                  </div>
                  <div className="flex items-center gap-2 p-2 bg-blue-50 dark:bg-blue-950 rounded">
                    <div className="w-2 h-2 bg-blue-500 rounded-full animate-pulse" />
                    <span>Creating research plan</span>
                  </div>
                </>
              )}
              
              {status === 'researching' && (
                <>
                  <div className="flex items-center gap-2 p-2 bg-yellow-50 dark:bg-yellow-950 rounded">
                    <div className="w-2 h-2 bg-yellow-500 rounded-full animate-pulse" />
                    <span>Exploring codebase patterns</span>
                  </div>
                  <div className="flex items-center gap-2 p-2 bg-yellow-50 dark:bg-yellow-950 rounded">
                    <div className="w-2 h-2 bg-yellow-500 rounded-full animate-pulse" />
                    <span>Analyzing relationships</span>
                  </div>
                  <div className="flex items-center gap-2 p-2 bg-yellow-50 dark:bg-yellow-950 rounded">
                    <div className="w-2 h-2 bg-yellow-500 rounded-full animate-pulse" />
                    <span>Synthesizing findings</span>
                  </div>
                  <div className="flex items-center gap-2 p-2 bg-yellow-50 dark:bg-yellow-950 rounded">
                    <div className="w-2 h-2 bg-yellow-500 rounded-full animate-pulse" />
                    <span>Preparing next iteration</span>
                  </div>
                </>
              )}
            </div>
          </div>
        )}

        {/* 完成状态 */}
        {isComplete && (
          <div className="p-3 bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 rounded-lg">
            <div className="flex items-center gap-2 text-green-700 dark:text-green-300">
              <CheckCircle className="h-4 w-4" />
              <span className="font-medium">Research completed successfully!</span>
            </div>
            <p className="text-sm text-green-600 dark:text-green-400 mt-1">
              All research iterations have been completed and findings have been synthesized.
            </p>
          </div>
        )}

        {/* 失败状态 */}
        {isFailed && (
          <div className="p-3 bg-red-50 dark:bg-red-950 border border-red-200 dark:border-red-800 rounded-lg">
            <div className="flex items-center gap-2 text-red-700 dark:text-red-300">
              <AlertCircle className="h-4 w-4" />
              <span className="font-medium">Research failed</span>
            </div>
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              The research process encountered an error. Please try again.
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
};

export default ResearchProgressIndicator;
