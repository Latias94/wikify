/**
 * 研究阶段查看器组件
 * 显示单个研究阶段的内容和元数据
 */

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { ResearchStage, ResearchStageType } from '@/types/api';
import { Markdown } from '@/components/Markdown';
import {
  FileText,
  Search,
  CheckCircle,
  Clock,
  Download,
  Copy,
  Loader2,
} from 'lucide-react';

// ============================================================================
// 组件接口
// ============================================================================

interface ResearchStageViewerProps {
  stage: ResearchStage;
  isActive?: boolean;
  showMetadata?: boolean;
  onDownload?: () => void;
  onCopy?: () => void;
}

// ============================================================================
// 工具函数
// ============================================================================

const getStageIcon = (type: ResearchStageType) => {
  switch (type) {
    case 'plan':
      return <FileText className="h-4 w-4" />;
    case 'update':
      return <Search className="h-4 w-4" />;
    case 'conclusion':
      return <CheckCircle className="h-4 w-4" />;
    default:
      return <FileText className="h-4 w-4" />;
  }
};

const getStageColor = (type: ResearchStageType) => {
  switch (type) {
    case 'plan':
      return 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200';
    case 'update':
      return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
    case 'conclusion':
      return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
    default:
      return 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200';
  }
};

const formatTimestamp = (timestamp: string) => {
  return new Date(timestamp).toLocaleString();
};

// ============================================================================
// 主组件
// ============================================================================

export const ResearchStageViewer: React.FC<ResearchStageViewerProps> = ({
  stage,
  isActive = false,
  showMetadata = true,
  onDownload,
  onCopy,
}) => {
  const handleCopyContent = async () => {
    try {
      await navigator.clipboard.writeText(stage.content);
      onCopy?.();
    } catch (error) {
      console.error('Failed to copy content:', error);
    }
  };

  const handleDownloadContent = () => {
    const blob = new Blob([stage.content], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `research-${stage.type}-${stage.iteration}-${new Date().toISOString().slice(0, 19).replace(/:/g, '-')}.md`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    onDownload?.();
  };

  return (
    <Card className={`transition-all duration-200 ${isActive ? 'ring-2 ring-primary ring-opacity-50' : ''}`}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`p-2 rounded-full ${getStageColor(stage.type)}`}>
              {getStageIcon(stage.type)}
            </div>
            <div>
              <CardTitle className="text-lg flex items-center gap-2">
                {stage.title}
                {isActive && (
                  <Loader2 className="h-4 w-4 animate-spin text-primary" />
                )}
              </CardTitle>
              {showMetadata && (
                <CardDescription className="flex items-center gap-4 mt-1">
                  <span className="flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {formatTimestamp(stage.timestamp)}
                  </span>
                  <Badge variant="outline" className="text-xs">
                    Iteration {stage.iteration}
                  </Badge>
                  {stage.confidence && (
                    <Badge variant="secondary" className="text-xs">
                      Confidence: {Math.round(stage.confidence * 100)}%
                    </Badge>
                  )}
                </CardDescription>
              )}
            </div>
          </div>

          {/* 操作按钮 */}
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopyContent}
              className="h-8 w-8 p-0"
              title="Copy content"
            >
              <Copy className="h-3 w-3" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleDownloadContent}
              className="h-8 w-8 p-0"
              title="Download as markdown"
            >
              <Download className="h-3 w-3" />
            </Button>
          </div>
        </div>
      </CardHeader>

      <Separator />

      <CardContent className="pt-4">
        {/* 内容区域 */}
        <div className="prose dark:prose-invert max-w-none">
          <Markdown content={stage.content} />
        </div>

        {/* 活动指示器 */}
        {isActive && (
          <div className="mt-4 p-3 bg-primary/5 border border-primary/20 rounded-lg">
            <div className="flex items-center gap-2 text-sm text-primary">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>Research in progress...</span>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
};

// ============================================================================
// 紧凑版研究阶段查看器
// ============================================================================

interface CompactResearchStageViewerProps {
  stage: ResearchStage;
  isActive?: boolean;
  onClick?: () => void;
}

export const CompactResearchStageViewer: React.FC<CompactResearchStageViewerProps> = ({
  stage,
  isActive = false,
  onClick,
}) => {
  return (
    <div
      className={`p-3 border rounded-lg cursor-pointer transition-all duration-200 hover:bg-accent/50 ${
        isActive ? 'border-primary bg-primary/5' : 'border-border'
      }`}
      onClick={onClick}
    >
      <div className="flex items-center gap-3">
        <div className={`p-1.5 rounded-full ${getStageColor(stage.type)}`}>
          {getStageIcon(stage.type)}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h4 className="font-medium text-sm truncate">{stage.title}</h4>
            {isActive && (
              <Loader2 className="h-3 w-3 animate-spin text-primary flex-shrink-0" />
            )}
          </div>
          <div className="flex items-center gap-2 mt-1">
            <Badge variant="outline" className="text-xs">
              Iteration {stage.iteration}
            </Badge>
            <span className="text-xs text-muted-foreground">
              {formatTimestamp(stage.timestamp)}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ResearchStageViewer;
