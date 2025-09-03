/**
 * 流式深度研究界面组件
 * 使用 SSE 实现实时研究进度更新
 */

import React, { useState } from 'react';
import { useStreamingResearch } from '@/hooks/use-streaming-research';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Separator } from '@/components/ui/separator';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Search,
  Brain,
  Play,
  Square,
  RotateCcw,
  Lightbulb,
  Target,
  Zap,
  CheckCircle,
  XCircle,
  Clock,
  AlertCircle,
} from 'lucide-react';

// ============================================================================
// 组件接口
// ============================================================================

interface StreamingResearchInterfaceProps {
  repositoryId: string;
  onResearchComplete?: (result: any) => void;
}

// ============================================================================
// 主组件
// ============================================================================

export const StreamingResearchInterface: React.FC<StreamingResearchInterfaceProps> = ({
  repositoryId,
  onResearchComplete,
}) => {
  const [query, setQuery] = useState('');
  const [maxIterations, setMaxIterations] = useState(5);
  const [maxSources, setMaxSources] = useState(10);

  const {
    researchId,
    status,
    originalQuery,
    currentIteration,
    maxIterations: actualMaxIterations,
    progress,
    progressPercentage,
    currentResponse,
    lastUpdated,
    finalResult,
    error,
    isActive,
    isComplete,
    isFailed,
    isCancelled,
    startStreamingResearch,
    stopResearch,
    resetResearch,
  } = useStreamingResearch();

  // ============================================================================
  // 事件处理
  // ============================================================================

  const handleStartResearch = async () => {
    if (!query.trim()) return;

    await startStreamingResearch(repositoryId, query.trim(), {
      max_iterations: maxIterations,
      max_sources_per_iteration: maxSources,
    });
  };

  const handleStopResearch = () => {
    stopResearch();
  };

  const handleResetResearch = () => {
    resetResearch();
    setQuery('');
  };

  // 当研究完成时调用回调
  React.useEffect(() => {
    if (isComplete && finalResult && onResearchComplete) {
      onResearchComplete(finalResult);
    }
  }, [isComplete, finalResult, onResearchComplete]);

  // ============================================================================
  // 状态图标
  // ============================================================================

  const getStatusIcon = () => {
    switch (status) {
      case "idle":
        return <Search className="h-5 w-5 text-muted-foreground" />;
      case "starting":
        return <Clock className="h-5 w-5 text-blue-500 animate-spin" />;
      case "researching":
        return <Brain className="h-5 w-5 text-blue-500 animate-pulse" />;
      case "completed":
        return <CheckCircle className="h-5 w-5 text-green-500" />;
      case "failed":
        return <XCircle className="h-5 w-5 text-red-500" />;
      case "cancelled":
        return <AlertCircle className="h-5 w-5 text-yellow-500" />;
      default:
        return <Search className="h-5 w-5 text-muted-foreground" />;
    }
  };

  const getStatusText = () => {
    switch (status) {
      case "idle":
        return "准备开始";
      case "starting":
        return "正在启动...";
      case "researching":
        return "研究中...";
      case "completed":
        return "研究完成";
      case "failed":
        return "研究失败";
      case "cancelled":
        return "已取消";
      default:
        return "未知状态";
    }
  };

  const getStatusColor = () => {
    switch (status) {
      case "idle":
        return "secondary";
      case "starting":
      case "researching":
        return "default";
      case "completed":
        return "default";
      case "failed":
        return "destructive";
      case "cancelled":
        return "secondary";
      default:
        return "secondary";
    }
  };

  // ============================================================================
  // 渲染
  // ============================================================================

  return (
    <div className="space-y-6">
      {/* 研究配置 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Brain className="h-5 w-5" />
            流式深度研究
          </CardTitle>
          <CardDescription>
            使用实时流式响应进行智能深度研究，获得即时反馈
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* 研究问题输入 */}
          <div className="space-y-2">
            <Label htmlFor="research-query">研究问题</Label>
            <Input
              id="research-query"
              placeholder="请输入您想要深入研究的问题..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              disabled={isActive}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  handleStartResearch();
                }
              }}
            />
          </div>

          {/* 配置选项 */}
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="max-iterations">最大迭代次数</Label>
              <Input
                id="max-iterations"
                type="number"
                min="1"
                max="10"
                value={maxIterations}
                onChange={(e) => setMaxIterations(parseInt(e.target.value) || 5)}
                disabled={isActive}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="max-sources">每次最大源数量</Label>
              <Input
                id="max-sources"
                type="number"
                min="1"
                max="20"
                value={maxSources}
                onChange={(e) => setMaxSources(parseInt(e.target.value) || 10)}
                disabled={isActive}
              />
            </div>
          </div>

          {/* 控制按钮 */}
          <div className="flex gap-2">
            {!isActive && !isComplete && (
              <Button
                onClick={handleStartResearch}
                disabled={!query.trim()}
                className="flex items-center gap-2"
              >
                <Play className="h-4 w-4" />
                开始研究
              </Button>
            )}

            {isActive && (
              <Button
                onClick={handleStopResearch}
                variant="destructive"
                className="flex items-center gap-2"
              >
                <Square className="h-4 w-4" />
                停止研究
              </Button>
            )}

            {(isComplete || isFailed || isCancelled) && (
              <Button
                onClick={handleResetResearch}
                variant="outline"
                className="flex items-center gap-2"
              >
                <RotateCcw className="h-4 w-4" />
                重新开始
              </Button>
            )}
          </div>
        </CardContent>
      </Card>

      {/* 研究状态和进度 */}
      {(isActive || isComplete || isFailed || isCancelled) && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              {getStatusIcon()}
              研究状态
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* 状态信息 */}
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <Badge variant={getStatusColor() as any}>
                    {getStatusText()}
                  </Badge>
                  {researchId && (
                    <span className="text-sm text-muted-foreground">
                      ID: {researchId.slice(0, 8)}...
                    </span>
                  )}
                </div>
                {originalQuery && (
                  <p className="text-sm text-muted-foreground">
                    研究问题: {originalQuery}
                  </p>
                )}
              </div>
              {lastUpdated && (
                <span className="text-xs text-muted-foreground">
                  更新于: {new Date(lastUpdated).toLocaleTimeString()}
                </span>
              )}
            </div>

            {/* 进度条 */}
            {isActive && (
              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span>进度: {progressPercentage}%</span>
                  <span>迭代: {currentIteration}/{actualMaxIterations}</span>
                </div>
                <Progress value={progressPercentage} className="w-full" />
              </div>
            )}

            {/* 当前响应 */}
            {currentResponse && (
              <div className="space-y-2">
                <Label>当前响应</Label>
                <ScrollArea className="h-32 w-full rounded border p-3">
                  <p className="text-sm whitespace-pre-wrap">{currentResponse}</p>
                </ScrollArea>
              </div>
            )}

            {/* 错误信息 */}
            {error && (
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            {/* 最终结果 */}
            {isComplete && finalResult && (
              <div className="space-y-2">
                <Label>研究结果</Label>
                <ScrollArea className="h-48 w-full rounded border p-3">
                  <pre className="text-sm whitespace-pre-wrap">
                    {JSON.stringify(finalResult, null, 2)}
                  </pre>
                </ScrollArea>
              </div>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
};
