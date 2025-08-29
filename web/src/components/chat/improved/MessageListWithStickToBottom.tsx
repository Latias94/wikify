/**
 * 使用use-stick-to-bottom的MessageList组件
 * 展示如何集成第三方滚动库的最佳实践
 */

import { memo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Button } from '@/components/ui/button';
import { ArrowDown, Loader2 } from 'lucide-react';
import { MessageBubble } from '../MessageBubble';
import { UIChatMessage } from '@/types/ui';
import { cn } from '@/lib/utils';

// 注意: 需要先安装 use-stick-to-bottom
// pnpm add use-stick-to-bottom

// import { StickToBottom, useStickToBottomContext } from 'use-stick-to-bottom';

interface MessageListProps {
  messages: UIChatMessage[];
  isLoading?: boolean;
  isConnected?: boolean;
  onRetryMessage?: (message: UIChatMessage) => void;
  onRegenerateMessage?: (message: UIChatMessage) => void;
  onCopyMessage?: (content: string) => void;
  className?: string;
}

// 滚动到底部按钮组件
function ScrollToBottomButton() {
  // const { isAtBottom, scrollToBottom } = useStickToBottomContext();

  // 临时实现，实际使用时取消注释上面的代码
  const isAtBottom = true;
  const scrollToBottom = () => {};

  if (isAtBottom) return null;

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.8, y: 20 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.8, y: 20 }}
      className="absolute bottom-4 left-1/2 transform -translate-x-1/2 z-10"
    >
      <Button
        variant="secondary"
        size="sm"
        onClick={scrollToBottom}
        className="rounded-full shadow-lg hover:shadow-xl transition-shadow"
      >
        <ArrowDown size={16} className="mr-1" />
        回到底部
      </Button>
    </motion.div>
  );
}

const MessageListWithStickToBottom = memo(({
  messages,
  isLoading = false,
  isConnected = true,
  onRetryMessage,
  onRegenerateMessage,
  onCopyMessage,
  className
}: MessageListProps) => {
  return (
    <div className={cn("relative flex-1 overflow-hidden", className)}>
      {/* 
      实际使用时的代码结构:
      
      <StickToBottom 
        className="h-full" 
        resize="smooth" 
        initial="smooth"
      >
        <StickToBottom.Content className="space-y-6 p-4">
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
        </StickToBottom.Content>

        {/* 滚动到底部按钮 */}
        <ScrollToBottomButton />
      </StickToBottom>
      */}

      {/* 临时实现 - 展示结构 */}
      <div className="h-full overflow-y-auto">
        <div className="space-y-6 p-4">
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
                <div className="w-8 h-8 text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium mb-2">开始对话</h3>
              <p className="text-muted-foreground max-w-md">
                向 AI 提问关于代码库的任何问题，我会基于索引的内容为您提供准确的答案。
              </p>
            </motion.div>
          )}
        </div>
      </div>

      {/* 滚动到底部按钮 */}
      <ScrollToBottomButton />
    </div>
  );
});

MessageListWithStickToBottom.displayName = 'MessageListWithStickToBottom';

export { MessageListWithStickToBottom };

// 使用示例和集成指南
export const IntegrationGuide = {
  install: 'pnpm add use-stick-to-bottom',
  
  benefits: [
    '专门为AI聊天应用设计',
    '智能区分用户滚动vs自动滚动',
    'velocity-based spring动画',
    '支持内容缩放而不失去粘性',
    '零依赖，轻量级 (31.8 kB)',
    '支持移动端触摸交互'
  ],
  
  migration: {
    step1: '安装依赖: pnpm add use-stick-to-bottom',
    step2: '取消注释import语句',
    step3: '取消注释StickToBottom组件使用',
    step4: '取消注释useStickToBottomContext使用',
    step5: '移除临时实现代码',
    step6: '测试各种滚动场景'
  },
  
  comparison: {
    before: '自实现滚动逻辑，需要处理复杂的边缘情况',
    after: '使用专业库，自动处理AI聊天的所有滚动场景'
  }
};
