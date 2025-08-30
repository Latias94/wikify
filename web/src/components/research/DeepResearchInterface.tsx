/**
 * 深度研究界面组件
 * 提供智能研究功能的完整用户界面
 */

import React, { useState } from 'react';
import { useDeepResearch } from '@/hooks/use-deep-research';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Separator } from '@/components/ui/separator';
import { ResearchStageViewer } from './ResearchStageViewer';
import { ResearchProgressIndicator } from './ResearchProgressIndicator';
import { ResearchNavigation } from './ResearchNavigation';
import {
  Search,
  Brain,
  Play,
  Square,
  RotateCcw,
  Lightbulb,
  Target,
  Zap,
} from 'lucide-react';

// ============================================================================
// 组件接口
// ============================================================================

interface DeepResearchInterfaceProps {
  repositoryId: string;
  sessionId?: string;
  onResearchComplete?: (conclusion: string) => void;
}

// ============================================================================
// 主组件
// ============================================================================

export const DeepResearchInterface: React.FC<DeepResearchInterfaceProps> = ({
  repositoryId,
  sessionId,
  onResearchComplete,
}) => {
  const [query, setQuery] = useState('');
  const [researchStrategy, setResearchStrategy] = useState<'comprehensive' | 'focused' | 'exploratory'>('comprehensive');

  const {
    // 状态
    status,
    currentIteration,
    maxIterations,
    stages,
    currentStage,
    currentStageIndex,
    progress,
    finalConclusion,
    confidenceScore,
    isAutoProgressing,
    
    // 操作
    startResearch,
    stopResearch,
    resetResearch,
    
    // 导航
    navigateToStage,
    navigateToNextStage,
    navigateToPreviousStage,
    canNavigateNext,
    canNavigatePrevious,
    
    // 状态检查
    isResearching,
    isComplete,
    isStarting,
  } = useDeepResearch();

  // ============================================================================
  // 事件处理
  // ============================================================================

  const handleStartResearch = () => {
    if (!query.trim()) return;

    startResearch({
      repository_id: repositoryId,
      session_id: sessionId,
      query: query.trim(),
      max_iterations: maxIterations,
      research_strategy: researchStrategy,
    });
  };

  const handleReset = () => {
    resetResearch();
    setQuery('');
  };

  // 当研究完成时通知父组件
  React.useEffect(() => {
    if (isComplete && finalConclusion && onResearchComplete) {
      onResearchComplete(finalConclusion);
    }
  }, [isComplete, finalConclusion, onResearchComplete]);

  // ============================================================================
  // 渲染
  // ============================================================================

  return (
    <div className="space-y-6">
      {/* 研究输入区域 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Brain className="h-5 w-5 text-primary" />
            Deep Research
          </CardTitle>
          <CardDescription>
            Conduct multi-iteration intelligent research on your codebase
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* 查询输入 */}
          <div className="space-y-2">
            <Label htmlFor="research-query">Research Question</Label>
            <Input
              id="research-query"
              placeholder="What would you like to research in depth?"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              disabled={isResearching}
              className="text-base"
            />
          </div>

          {/* 研究策略选择 */}
          <div className="space-y-2">
            <Label>Research Strategy</Label>
            <div className="flex gap-2">
              <Button
                variant={researchStrategy === 'comprehensive' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setResearchStrategy('comprehensive')}
                disabled={isResearching}
                className="flex items-center gap-1"
              >
                <Target className="h-3 w-3" />
                Comprehensive
              </Button>
              <Button
                variant={researchStrategy === 'focused' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setResearchStrategy('focused')}
                disabled={isResearching}
                className="flex items-center gap-1"
              >
                <Zap className="h-3 w-3" />
                Focused
              </Button>
              <Button
                variant={researchStrategy === 'exploratory' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setResearchStrategy('exploratory')}
                disabled={isResearching}
                className="flex items-center gap-1"
              >
                <Lightbulb className="h-3 w-3" />
                Exploratory
              </Button>
            </div>
          </div>

          {/* 控制按钮 */}
          <div className="flex gap-2">
            {!isResearching && !isComplete && (
              <Button
                onClick={handleStartResearch}
                disabled={!query.trim() || isStarting}
                className="flex items-center gap-2"
              >
                <Play className="h-4 w-4" />
                {isStarting ? 'Starting...' : 'Start Research'}
              </Button>
            )}
            
            {isResearching && (
              <Button
                onClick={stopResearch}
                variant="destructive"
                className="flex items-center gap-2"
              >
                <Square className="h-4 w-4" />
                Stop Research
              </Button>
            )}
            
            {(isComplete || status === 'failed') && (
              <Button
                onClick={handleReset}
                variant="outline"
                className="flex items-center gap-2"
              >
                <RotateCcw className="h-4 w-4" />
                New Research
              </Button>
            )}
          </div>
        </CardContent>
      </Card>

      {/* 研究进度指示器 */}
      {(isResearching || isComplete) && (
        <ResearchProgressIndicator
          status={status}
          currentIteration={currentIteration}
          maxIterations={maxIterations}
          progress={progress}
          confidenceScore={confidenceScore}
        />
      )}

      {/* 研究阶段导航 */}
      {stages.length > 0 && (
        <ResearchNavigation
          stages={stages}
          currentStageIndex={currentStageIndex}
          onNavigateToStage={navigateToStage}
          onNavigateNext={navigateToNextStage}
          onNavigatePrevious={navigateToPreviousStage}
          canNavigateNext={canNavigateNext}
          canNavigatePrevious={canNavigatePrevious}
        />
      )}

      {/* 当前研究阶段内容 */}
      {currentStage && (
        <ResearchStageViewer
          stage={currentStage}
          isActive={isResearching && currentStageIndex === stages.length - 1}
          showMetadata={true}
        />
      )}

      {/* 最终结论 */}
      {isComplete && finalConclusion && (
        <Card className="border-green-200 bg-green-50 dark:border-green-800 dark:bg-green-950">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-green-700 dark:text-green-300">
              <Lightbulb className="h-5 w-5" />
              Research Complete
            </CardTitle>
            {confidenceScore && (
              <div className="flex items-center gap-2">
                <Badge variant="secondary">
                  Confidence: {Math.round(confidenceScore * 100)}%
                </Badge>
              </div>
            )}
          </CardHeader>
          <CardContent>
            <div className="prose dark:prose-invert max-w-none">
              <p className="text-green-600 dark:text-green-400">
                The deep research process has completed successfully. 
                All findings have been synthesized into a comprehensive conclusion.
              </p>
            </div>
          </CardContent>
        </Card>
      )}

      {/* 研究失败状态 */}
      {status === 'failed' && (
        <Card className="border-red-200 bg-red-50 dark:border-red-800 dark:bg-red-950">
          <CardHeader>
            <CardTitle className="text-red-700 dark:text-red-300">
              Research Failed
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-red-600 dark:text-red-400">
              The research process encountered an error. Please try again with a different query or strategy.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
};

export default DeepResearchInterface;
