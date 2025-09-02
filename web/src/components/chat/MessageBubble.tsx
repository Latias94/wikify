/**
 * 消息气泡组件
 * 参考 Vercel AI Chatbot 的设计
 */

import { memo, useState, useCallback, forwardRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { 
  User, 
  Bot, 
  Copy, 
  RefreshCw, 
  ThumbsUp, 
  ThumbsDown,
  MoreHorizontal,
  FileText,
  ExternalLink
} from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { UIChatMessage } from '@/types/ui';
import { formatRelativeTime } from '@/utils/formatters';
import { StreamingContent } from './StreamingContent';
import { MessageActions } from './MessageActions';
import { SourceDocuments } from './SourceDocuments';
import { cn } from '@/lib/utils';

interface MessageBubbleProps {
  message: UIChatMessage;
  isLast?: boolean;
  onCopy?: (content: string) => void;
  onRetry?: (message: UIChatMessage) => void;
  onRegenerate?: (message: UIChatMessage) => void;
  className?: string;
}

const MessageBubble = memo(forwardRef<HTMLDivElement, MessageBubbleProps>(({
  message,
  isLast = false,
  onCopy,
  onRetry,
  onRegenerate,
  className
}, ref) => {
  const { toast } = useToast();
  const [showActions, setShowActions] = useState(false);
  const [showSources, setShowSources] = useState(false);

  // 防护性检查：确保 message 对象存在且有必要的字段
  if (!message || !message.content) {
    console.warn('MessageBubble: Invalid message object', message);
    return null;
  }

  const handleCopy = useCallback(() => {
    const content = message?.content || '';
    if (!content) {
      toast({
        title: "复制失败",
        description: "消息内容为空",
        variant: "destructive"
      });
      return;
    }

    navigator.clipboard.writeText(content).then(() => {
      toast({
        title: "已复制",
        description: "消息已复制到剪贴板",
      });
      onCopy?.(content);
    }).catch(() => {
      toast({
        title: "复制失败",
        description: "无法复制消息到剪贴板",
        variant: "destructive"
      });
    });
  }, [message?.content, toast, onCopy]);

  const handleRetry = useCallback(() => {
    onRetry?.(message);
  }, [message, onRetry]);

  const handleRegenerate = useCallback(() => {
    onRegenerate?.(message);
  }, [message, onRegenerate]);

  const toggleSources = useCallback(() => {
    setShowSources(prev => !prev);
  }, []);

  return (
    <motion.div
      ref={ref}
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{
        duration: 0.3,
        ease: [0.25, 0.46, 0.45, 0.94]
      }}
      className={cn(
        "px-4 mx-auto w-full max-w-4xl group/message",
        className
      )}
      data-testid={`message-${message.role}`}
      data-role={message.role}
      onMouseEnter={() => setShowActions(true)}
      onMouseLeave={() => setShowActions(false)}
    >
      <div className={cn(
        "flex gap-4 w-full",
        message.role === 'user' && "ml-auto max-w-2xl"
      )}>
        {/* 头像 */}
        <Avatar className="size-8 shrink-0 ring-1 ring-border">
          <AvatarFallback className={cn(
            "text-xs font-medium",
            message.role === 'user' 
              ? "bg-primary text-primary-foreground" 
              : "bg-muted text-muted-foreground"
          )}>
            {message.role === 'user' ? (
              <User size={14} />
            ) : (
              <Bot size={14} />
            )}
          </AvatarFallback>
        </Avatar>

        {/* 消息内容 */}
        <div className="flex flex-col gap-2 w-full min-w-0 overflow-hidden">
          {/* 消息主体 */}
          <div className={cn(
            "relative rounded-lg px-4 py-3 w-full overflow-hidden",
            message.role === 'user'
              ? "bg-primary text-primary-foreground ml-auto"
              : "bg-muted/50 text-foreground",
            message.isError && "bg-destructive/10 border border-destructive/20"
          )}>
            <StreamingContent
              content={message.content || ''}
              className="prose prose-sm dark:prose-invert max-w-none"
            />

            {/* 流式指示器 */}
            {message.isStreaming && (
              <motion.div
                className="inline-block w-2 h-4 bg-current ml-1"
                animate={{ opacity: [1, 0, 1] }}
                transition={{ 
                  duration: 1.2, 
                  repeat: Infinity,
                  ease: "easeInOut"
                }}
              />
            )}
          </div>

          {/* 源文档 */}
          {message.sources && message.sources.length > 0 && (
            <div className="space-y-2 w-full max-w-full overflow-hidden">
              <Button
                variant="ghost"
                size="sm"
                onClick={toggleSources}
                className="h-auto p-1 text-xs text-muted-foreground hover:text-foreground"
              >
                <FileText size={12} className="mr-1" />
                {message.sources.length} 个相关文档
                {showSources ? " (隐藏)" : " (显示)"}
              </Button>

              <AnimatePresence>
                {showSources && (
                  <SourceDocuments
                    sources={message.sources}
                    className="ml-0 w-full max-w-full"
                  />
                )}
              </AnimatePresence>
            </div>
          )}

          {/* 消息元信息 */}
          <div className="flex items-center justify-between text-xs text-muted-foreground min-h-[20px]">
            <span>{formatRelativeTime(message.timestamp)}</span>

            {/* 消息操作 - 始终保留空间，只控制透明度 */}
            <div className="flex items-center">
              <MessageActions
                message={message}
                onCopy={handleCopy}
                onRetry={message.role === 'user' ? handleRetry : undefined}
                onRegenerate={message.role === 'assistant' ? handleRegenerate : undefined}
                className={cn(
                  "transition-opacity duration-200 ease-in-out",
                  (showActions || isLast)
                    ? "opacity-100"
                    : "opacity-0 pointer-events-none"
                )}
              />
            </div>
          </div>
        </div>
      </div>
    </motion.div>
  );
}));

MessageBubble.displayName = 'MessageBubble';

export { MessageBubble };
