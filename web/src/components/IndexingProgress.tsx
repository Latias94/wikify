/**
 * 索引进度组件
 * 显示实时的仓库索引进度
 */

import React, { useState, useEffect } from 'react';
import { Progress } from '@/components/ui/progress';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { AlertCircle, CheckCircle2, FileText, Loader2, X } from 'lucide-react';
import { useIndexProgressWebSocket } from '@/hooks/use-websocket';
import { IndexProgressMessage, IndexCompleteMessage, IndexErrorMessage } from '@/types/websocket';

interface IndexingProgressProps {
  sessionId?: string;
  onComplete?: () => void;
  onError?: (error: string) => void;
  onCancel?: () => void;
  className?: string;
}

interface ProgressState {
  progress: number;
  currentFile?: string;
  filesProcessed: number;
  totalFiles: number;
  status: 'connecting' | 'indexing' | 'completed' | 'error' | 'cancelled';
  error?: string;
  startTime?: Date;
  endTime?: Date;
}

export function IndexingProgress({
  sessionId,
  onComplete,
  onError,
  onCancel,
  className,
}: IndexingProgressProps) {
  const [progressState, setProgressState] = useState<ProgressState>({
    progress: 0,
    filesProcessed: 0,
    totalFiles: 0,
    status: 'connecting',
  });

  // WebSocket 连接
  const { isConnected, disconnect } = useIndexProgressWebSocket(
    // 进度更新回调
    (progress: IndexProgressMessage) => {
      setProgressState(prev => ({
        ...prev,
        progress: progress.progress,
        currentFile: progress.current_file,
        filesProcessed: progress.files_processed,
        totalFiles: progress.total_files,
        status: 'indexing',
        startTime: prev.startTime || new Date(),
      }));
    },
    // 完成回调
    (completion: IndexCompleteMessage) => {
      setProgressState(prev => ({
        ...prev,
        progress: 1.0,
        currentFile: completion.current_file,
        filesProcessed: completion.files_processed,
        totalFiles: completion.total_files,
        status: 'completed',
        endTime: new Date(),
      }));
      onComplete?.();
    },
    // 错误回调
    (error: IndexErrorMessage) => {
      setProgressState(prev => ({
        ...prev,
        status: 'error',
        error: error.message,
        endTime: new Date(),
      }));
      onError?.(error.message);
    }
  );

  // 计算持续时间
  const getDuration = () => {
    if (!progressState.startTime) return null;
    const endTime = progressState.endTime || new Date();
    const duration = Math.round((endTime.getTime() - progressState.startTime.getTime()) / 1000);
    return duration;
  };

  // 格式化文件名
  const formatFileName = (fileName?: string) => {
    if (!fileName) return 'Processing...';
    if (fileName.length > 50) {
      return '...' + fileName.slice(-47);
    }
    return fileName;
  };

  // 获取状态图标
  const getStatusIcon = () => {
    switch (progressState.status) {
      case 'connecting':
        return <Loader2 className="h-4 w-4 animate-spin" />;
      case 'indexing':
        return <Loader2 className="h-4 w-4 animate-spin" />;
      case 'completed':
        return <CheckCircle2 className="h-4 w-4 text-green-500" />;
      case 'error':
        return <AlertCircle className="h-4 w-4 text-red-500" />;
      case 'cancelled':
        return <X className="h-4 w-4 text-gray-500" />;
      default:
        return <FileText className="h-4 w-4" />;
    }
  };

  // 获取状态文本
  const getStatusText = () => {
    switch (progressState.status) {
      case 'connecting':
        return 'Connecting to indexing service...';
      case 'indexing':
        return 'Indexing repository...';
      case 'completed':
        return 'Indexing completed successfully!';
      case 'error':
        return 'Indexing failed';
      case 'cancelled':
        return 'Indexing cancelled';
      default:
        return 'Unknown status';
    }
  };

  // 获取状态颜色
  const getStatusColor = () => {
    switch (progressState.status) {
      case 'connecting':
        return 'default';
      case 'indexing':
        return 'default';
      case 'completed':
        return 'default';
      case 'error':
        return 'destructive';
      case 'cancelled':
        return 'secondary';
      default:
        return 'default';
    }
  };

  // 处理取消
  const handleCancel = () => {
    setProgressState(prev => ({
      ...prev,
      status: 'cancelled',
      endTime: new Date(),
    }));
    disconnect();
    onCancel?.();
  };

  const progressPercentage = Math.round(progressState.progress * 100);
  const duration = getDuration();

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            {getStatusIcon()}
            <CardTitle className="text-lg">Repository Indexing</CardTitle>
          </div>
          <div className="flex items-center space-x-2">
            <Badge variant={getStatusColor() as any}>
              {getStatusText()}
            </Badge>
            {progressState.status === 'indexing' && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleCancel}
                className="h-8"
              >
                Cancel
              </Button>
            )}
          </div>
        </div>
        <CardDescription>
          {sessionId && `Session: ${sessionId}`}
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* 进度条 */}
        <div className="space-y-2">
          <div className="flex justify-between text-sm">
            <span>Progress</span>
            <span>{progressPercentage}%</span>
          </div>
          <Progress 
            value={progressPercentage} 
            className="h-2"
          />
        </div>

        {/* 文件信息 */}
        {progressState.status === 'indexing' && (
          <div className="space-y-2">
            <div className="flex justify-between text-sm">
              <span>Files processed</span>
              <span>{progressState.filesProcessed} / {progressState.totalFiles}</span>
            </div>
            {progressState.currentFile && (
              <div className="text-sm text-muted-foreground">
                <span className="font-medium">Current file:</span>{' '}
                <code className="text-xs bg-muted px-1 py-0.5 rounded">
                  {formatFileName(progressState.currentFile)}
                </code>
              </div>
            )}
          </div>
        )}

        {/* 完成信息 */}
        {progressState.status === 'completed' && (
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span>Total files processed</span>
              <span className="font-medium">{progressState.filesProcessed}</span>
            </div>
            {duration && (
              <div className="flex justify-between">
                <span>Duration</span>
                <span className="font-medium">{duration}s</span>
              </div>
            )}
            {progressState.currentFile && (
              <div className="text-muted-foreground">
                {progressState.currentFile}
              </div>
            )}
          </div>
        )}

        {/* 错误信息 */}
        {progressState.status === 'error' && progressState.error && (
          <div className="p-3 bg-red-50 border border-red-200 rounded-md">
            <div className="flex items-start space-x-2">
              <AlertCircle className="h-4 w-4 text-red-500 mt-0.5 flex-shrink-0" />
              <div className="text-sm text-red-700">
                <div className="font-medium">Indexing failed</div>
                <div className="mt-1">{progressState.error}</div>
              </div>
            </div>
          </div>
        )}

        {/* 连接状态 */}
        {!isConnected() && progressState.status !== 'completed' && progressState.status !== 'error' && (
          <div className="text-sm text-muted-foreground">
            Connecting to progress updates...
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export default IndexingProgress;
