/**
 * 消息操作组件
 * 提供复制、重试、重新生成等操作
 */

import { memo, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { 
  Copy, 
  RefreshCw, 
  ThumbsUp, 
  ThumbsDown,
  MoreHorizontal,
  Edit3,
  Trash2
} from 'lucide-react';
import { UIChatMessage } from '@/types/ui';
import { cn } from '@/lib/utils';

interface MessageActionsProps {
  message: UIChatMessage;
  onCopy?: () => void;
  onRetry?: () => void;
  onRegenerate?: () => void;
  onEdit?: () => void;
  onDelete?: () => void;
  onFeedback?: (type: 'positive' | 'negative') => void;
  className?: string;
}

const MessageActions = memo(({
  message,
  onCopy,
  onRetry,
  onRegenerate,
  onEdit,
  onDelete,
  onFeedback,
  className
}: MessageActionsProps) => {
  const handleFeedback = useCallback((type: 'positive' | 'negative') => {
    onFeedback?.(type);
  }, [onFeedback]);

  const actions = [
    // 复制操作 - 所有消息都支持
    {
      key: 'copy',
      icon: Copy,
      label: '复制',
      onClick: onCopy,
      show: true,
    },
    // 重试操作 - 仅用户消息支持
    {
      key: 'retry',
      icon: RefreshCw,
      label: '重试',
      onClick: onRetry,
      show: message.role === 'user' && !!onRetry,
    },
    // 重新生成 - 仅 AI 消息支持
    {
      key: 'regenerate',
      icon: RefreshCw,
      label: '重新生成',
      onClick: onRegenerate,
      show: message.role === 'assistant' && !!onRegenerate && !message.isStreaming,
    },
    // 编辑操作 - 仅用户消息支持
    {
      key: 'edit',
      icon: Edit3,
      label: '编辑',
      onClick: onEdit,
      show: message.role === 'user' && !!onEdit,
    },
    // 删除操作
    {
      key: 'delete',
      icon: Trash2,
      label: '删除',
      onClick: onDelete,
      show: !!onDelete,
      variant: 'destructive' as const,
    },
  ].filter(action => action.show);

  const feedbackActions = [
    {
      key: 'thumbs-up',
      icon: ThumbsUp,
      label: '有用',
      onClick: () => handleFeedback('positive'),
      show: message.role === 'assistant' && !!onFeedback,
    },
    {
      key: 'thumbs-down',
      icon: ThumbsDown,
      label: '无用',
      onClick: () => handleFeedback('negative'),
      show: message.role === 'assistant' && !!onFeedback,
    },
  ].filter(action => action.show);

  if (actions.length === 0 && feedbackActions.length === 0) {
    return null;
  }

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ duration: 0.15 }}
      className={cn(
        "flex items-center gap-1",
        className
      )}
    >
      <TooltipProvider delayDuration={300}>
        {/* 主要操作 */}
        {actions.map((action) => (
          <Tooltip key={action.key}>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                onClick={action.onClick}
                className={cn(
                  "h-7 w-7 p-0 hover:bg-muted",
                  action.variant === 'destructive' && "hover:bg-destructive/10 hover:text-destructive"
                )}
              >
                <action.icon size={12} />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="top" className="text-xs">
              {action.label}
            </TooltipContent>
          </Tooltip>
        ))}

        {/* 分隔符 */}
        {actions.length > 0 && feedbackActions.length > 0 && (
          <div className="w-px h-4 bg-border mx-1" />
        )}

        {/* 反馈操作 */}
        {feedbackActions.map((action) => (
          <Tooltip key={action.key}>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                onClick={action.onClick}
                className="h-7 w-7 p-0 hover:bg-muted"
              >
                <action.icon size={12} />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="top" className="text-xs">
              {action.label}
            </TooltipContent>
          </Tooltip>
        ))}
      </TooltipProvider>
    </motion.div>
  );
});

MessageActions.displayName = 'MessageActions';

export { MessageActions };
