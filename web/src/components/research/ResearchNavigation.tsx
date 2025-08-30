/**
 * 研究导航组件
 * 提供研究阶段之间的导航功能
 */

import React from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { CompactResearchStageViewer } from './ResearchStageViewer';
import { ResearchStage } from '@/types/api';
import {
  ChevronLeft,
  ChevronRight,
  SkipBack,
  SkipForward,
  Navigation,
  List,
} from 'lucide-react';

// ============================================================================
// 组件接口
// ============================================================================

interface ResearchNavigationProps {
  stages: ResearchStage[];
  currentStageIndex: number;
  onNavigateToStage: (index: number) => void;
  onNavigateNext: () => void;
  onNavigatePrevious: () => void;
  canNavigateNext: boolean;
  canNavigatePrevious: boolean;
}

// ============================================================================
// 导航控制组件
// ============================================================================

interface NavigationControlsProps {
  currentStageIndex: number;
  totalStages: number;
  onNavigateNext: () => void;
  onNavigatePrevious: () => void;
  onNavigateFirst: () => void;
  onNavigateLast: () => void;
  canNavigateNext: boolean;
  canNavigatePrevious: boolean;
}

const NavigationControls: React.FC<NavigationControlsProps> = ({
  currentStageIndex,
  totalStages,
  onNavigateNext,
  onNavigatePrevious,
  onNavigateFirst,
  onNavigateLast,
  canNavigateNext,
  canNavigatePrevious,
}) => {
  return (
    <div className="flex items-center justify-between">
      <div className="flex items-center gap-1">
        <Button
          variant="outline"
          size="sm"
          onClick={onNavigateFirst}
          disabled={!canNavigatePrevious}
          className="h-8 w-8 p-0"
          title="First stage"
        >
          <SkipBack className="h-3 w-3" />
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={onNavigatePrevious}
          disabled={!canNavigatePrevious}
          className="h-8 w-8 p-0"
          title="Previous stage"
        >
          <ChevronLeft className="h-3 w-3" />
        </Button>
      </div>

      <div className="flex items-center gap-2">
        <Badge variant="secondary" className="text-xs">
          {currentStageIndex + 1} of {totalStages}
        </Badge>
      </div>

      <div className="flex items-center gap-1">
        <Button
          variant="outline"
          size="sm"
          onClick={onNavigateNext}
          disabled={!canNavigateNext}
          className="h-8 w-8 p-0"
          title="Next stage"
        >
          <ChevronRight className="h-3 w-3" />
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={onNavigateLast}
          disabled={!canNavigateNext}
          className="h-8 w-8 p-0"
          title="Last stage"
        >
          <SkipForward className="h-3 w-3" />
        </Button>
      </div>
    </div>
  );
};

// ============================================================================
// 阶段列表组件
// ============================================================================

interface StageListProps {
  stages: ResearchStage[];
  currentStageIndex: number;
  onNavigateToStage: (index: number) => void;
}

const StageList: React.FC<StageListProps> = ({
  stages,
  currentStageIndex,
  onNavigateToStage,
}) => {
  return (
    <div className="space-y-2">
      {stages.map((stage, index) => (
        <CompactResearchStageViewer
          key={stage.id}
          stage={stage}
          isActive={index === currentStageIndex}
          onClick={() => onNavigateToStage(index)}
        />
      ))}
    </div>
  );
};

// ============================================================================
// 主组件
// ============================================================================

export const ResearchNavigation: React.FC<ResearchNavigationProps> = ({
  stages,
  currentStageIndex,
  onNavigateToStage,
  onNavigateNext,
  onNavigatePrevious,
  canNavigateNext,
  canNavigatePrevious,
}) => {
  const [showStageList, setShowStageList] = React.useState(false);

  const handleNavigateFirst = () => {
    onNavigateToStage(0);
  };

  const handleNavigateLast = () => {
    onNavigateToStage(stages.length - 1);
  };

  if (stages.length === 0) {
    return null;
  }

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base flex items-center gap-2">
            <Navigation className="h-4 w-4" />
            Research Navigation
          </CardTitle>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setShowStageList(!showStageList)}
            className="flex items-center gap-1"
          >
            <List className="h-3 w-3" />
            {showStageList ? 'Hide' : 'Show'} Stages
          </Button>
        </div>
      </CardHeader>

      <Separator />

      <CardContent className="pt-4 space-y-4">
        {/* 导航控制 */}
        <NavigationControls
          currentStageIndex={currentStageIndex}
          totalStages={stages.length}
          onNavigateNext={onNavigateNext}
          onNavigatePrevious={onNavigatePrevious}
          onNavigateFirst={handleNavigateFirst}
          onNavigateLast={handleNavigateLast}
          canNavigateNext={canNavigateNext}
          canNavigatePrevious={canNavigatePrevious}
        />

        {/* 当前阶段信息 */}
        {stages[currentStageIndex] && (
          <div className="p-3 bg-accent/50 rounded-lg">
            <div className="flex items-center justify-between">
              <div>
                <h4 className="font-medium text-sm">
                  {stages[currentStageIndex].title}
                </h4>
                <p className="text-xs text-muted-foreground mt-1">
                  Iteration {stages[currentStageIndex].iteration} • {' '}
                  {new Date(stages[currentStageIndex].timestamp).toLocaleString()}
                </p>
              </div>
              <Badge variant="outline" className="text-xs">
                {stages[currentStageIndex].type}
              </Badge>
            </div>
          </div>
        )}

        {/* 阶段列表 */}
        {showStageList && (
          <div className="space-y-2">
            <Separator />
            <div className="text-sm font-medium text-muted-foreground">
              All Research Stages
            </div>
            <StageList
              stages={stages}
              currentStageIndex={currentStageIndex}
              onNavigateToStage={onNavigateToStage}
            />
          </div>
        )}

        {/* 进度指示器 */}
        <div className="space-y-2">
          <div className="text-xs text-muted-foreground">Stage Progress</div>
          <div className="flex gap-1">
            {stages.map((_, index) => (
              <button
                key={index}
                onClick={() => onNavigateToStage(index)}
                className={`h-2 flex-1 rounded-full transition-all duration-200 ${
                  index === currentStageIndex
                    ? 'bg-primary'
                    : index < currentStageIndex
                    ? 'bg-primary/60'
                    : 'bg-muted hover:bg-muted-foreground/20'
                }`}
                title={`Go to ${stages[index].title}`}
              />
            ))}
          </div>
        </div>

        {/* 键盘快捷键提示 */}
        <div className="text-xs text-muted-foreground">
          <div className="flex items-center gap-4">
            <span>← Previous</span>
            <span>→ Next</span>
            <span>Home First</span>
            <span>End Last</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};

// ============================================================================
// 键盘导航支持
// ============================================================================

export const useResearchKeyboardNavigation = (
  onNavigateNext: () => void,
  onNavigatePrevious: () => void,
  onNavigateFirst: () => void,
  onNavigateLast: () => void,
  canNavigateNext: boolean,
  canNavigatePrevious: boolean,
) => {
  React.useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // 只在没有焦点在输入元素时处理键盘事件
      if (
        document.activeElement?.tagName === 'INPUT' ||
        document.activeElement?.tagName === 'TEXTAREA' ||
        document.activeElement?.contentEditable === 'true'
      ) {
        return;
      }

      switch (event.key) {
        case 'ArrowLeft':
          if (canNavigatePrevious) {
            event.preventDefault();
            onNavigatePrevious();
          }
          break;
        case 'ArrowRight':
          if (canNavigateNext) {
            event.preventDefault();
            onNavigateNext();
          }
          break;
        case 'Home':
          if (canNavigatePrevious) {
            event.preventDefault();
            onNavigateFirst();
          }
          break;
        case 'End':
          if (canNavigateNext) {
            event.preventDefault();
            onNavigateLast();
          }
          break;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [
    onNavigateNext,
    onNavigatePrevious,
    onNavigateFirst,
    onNavigateLast,
    canNavigateNext,
    canNavigatePrevious,
  ]);
};

export default ResearchNavigation;
