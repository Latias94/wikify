/**
 * 消息列表组件
 * 显示聊天消息列表，支持虚拟滚动和动画
 */

import { memo, useRef, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { ArrowDown, Loader2 } from 'lucide-react';
import { MessageBubble } from './MessageBubble';
import { UIChatMessage } from '@/types/ui';
import { cn } from '@/lib/utils';
import { UI_CONFIG } from '@/lib/constants';

interface MessageListProps {
  messages: UIChatMessage[];
  isLoading?: boolean;
  isConnected?: boolean;
  autoScroll?: boolean;
  onRetryMessage?: (message: UIChatMessage) => void;
  onRegenerateMessage?: (message: UIChatMessage) => void;
  onCopyMessage?: (content: string) => void;
  className?: string;
}

const MessageList = memo(({
  messages,
  isLoading = false,
  isConnected = true,
  autoScroll = true,
  onRetryMessage,
  onRegenerateMessage,
  onCopyMessage,
  className
}: MessageListProps) => {
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const lastScrollTop = useRef(0);
  const userScrolled = useRef(false);

  // 滚动到底部
  const scrollToBottom = useCallback((smooth = true) => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ 
        behavior: smooth ? 'smooth' : 'auto',
        block: 'end'
      });
    }
  }, []);

  // 检查是否接近底部
  const isNearBottom = useCallback(() => {
    const scrollArea = scrollAreaRef.current;
    if (!scrollArea) return true;
    
    const { scrollTop, scrollHeight, clientHeight } = scrollArea;
    const threshold = UI_CONFIG.CHAT.AUTO_SCROLL_THRESHOLD;
    
    return scrollHeight - scrollTop - clientHeight < threshold;
  }, []);

  // 处理滚动事件
  const handleScroll = useCallback(() => {
    const scrollArea = scrollAreaRef.current;
    if (!scrollArea) return;
    
    const { scrollTop } = scrollArea;
    
    // 检测用户是否主动滚动
    if (Math.abs(scrollTop - lastScrollTop.current) > 10) {
      userScrolled.current = !isNearBottom();
    }
    
    lastScrollTop.current = scrollTop;
  }, [isNearBottom]);

  // 自动滚动到底部
  useEffect(() => {
    if (autoScroll && !userScrolled.current) {
      scrollToBottom();
    }
  }, [messages, autoScroll, scrollToBottom]);

  // 重置用户滚动状态
  useEffect(() => {
    if (isNearBottom()) {
      userScrolled.current = false;
    }
  }, [isNearBottom]);

  // 连接状态变化时滚动到底部
  useEffect(() => {
    if (isConnected && messages.length > 0) {
      scrollToBottom(false);
    }
  }, [isConnected, scrollToBottom, messages.length]);

  const handleScrollToBottom = useCallback(() => {
    userScrolled.current = false;
    scrollToBottom();
  }, [scrollToBottom]);

  const showScrollButton = userScrolled.current && messages.length > 0;

  return (
    <div className={cn("relative flex-1 overflow-hidden", className)}>
      <ScrollArea 
        ref={scrollAreaRef}
        className="h-full px-4"
        onScrollCapture={handleScroll}
      >
        <div className="space-y-6 py-4">
          {/* 连接状态提示 */}
          {!isConnected && (
            <motion.div
              initial={{ opacity: 0, y: -20 }}
              animate={{ opacity: 1, y: 0 }}
              className="flex items-center justify-center py-4"
            >
              <div className="flex items-center gap-2 px-4 py-2 bg-destructive/10 border border-destructive/20 rounded-full text-sm text-destructive">
                <div className="w-2 h-2 bg-destructive rounded-full animate-pulse" />
                连接已断开，正在重连...
              </div>
            </motion.div>
          )}

          {/* 消息列表 */}
          <AnimatePresence mode="popLayout">
            {messages.map((message, index) => (
              <MessageBubble
                key={message.id}
                message={message}
                isLast={index === messages.length - 1}
                onCopy={onCopyMessage}
                onRetry={onRetryMessage}
                onRegenerate={onRegenerateMessage}
              />
            ))}
          </AnimatePresence>

          {/* 加载指示器 */}
          {isLoading && (
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
              className="flex items-center justify-center py-4"
            >
              <div className="flex items-center gap-2 text-muted-foreground">
                <Loader2 size={16} className="animate-spin" />
                <span className="text-sm">AI 正在思考...</span>
              </div>
            </motion.div>
          )}

          {/* 空状态 */}
          {messages.length === 0 && !isLoading && (
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              className="flex flex-col items-center justify-center py-12 text-center"
            >
              <div className="w-16 h-16 bg-muted rounded-full flex items-center justify-center mb-4">
                <MessageBubble className="w-8 h-8 text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium mb-2">开始对话</h3>
              <p className="text-muted-foreground max-w-md">
                向 AI 提问关于代码库的任何问题，我会基于索引的内容为您提供准确的答案。
              </p>
            </motion.div>
          )}

          {/* 滚动锚点 */}
          <div ref={messagesEndRef} />
        </div>
      </ScrollArea>

      {/* 滚动到底部按钮 */}
      <AnimatePresence>
        {showScrollButton && (
          <motion.div
            initial={{ opacity: 0, scale: 0.8, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.8, y: 20 }}
            className="absolute bottom-4 right-4"
          >
            <Button
              variant="secondary"
              size="sm"
              onClick={handleScrollToBottom}
              className="rounded-full shadow-lg hover:shadow-xl transition-shadow"
            >
              <ArrowDown size={16} className="mr-1" />
              回到底部
            </Button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
});

MessageList.displayName = 'MessageList';

export { MessageList };
