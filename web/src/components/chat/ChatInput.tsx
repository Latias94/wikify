/**
 * 聊天输入组件
 * 参考 Vercel AI Chatbot 的多模态输入设计
 */

import { memo, useRef, useCallback, useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { 
  Send, 
  Paperclip, 
  Mic, 
  Square,
  Loader2,
  AlertCircle
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { validateChatMessage } from '@/utils/validators';
import { UI_CONFIG } from '@/lib/constants';

interface ChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSend: (message: string) => void;
  onStop?: () => void;
  disabled?: boolean;
  isLoading?: boolean;
  isStreaming?: boolean;
  placeholder?: string;
  maxLength?: number;
  className?: string;
}

const ChatInput = memo(({
  value,
  onChange,
  onSend,
  onStop,
  disabled = false,
  isLoading = false,
  isStreaming = false,
  placeholder = "输入消息...",
  maxLength = UI_CONFIG.CHAT.MAX_MESSAGES,
  className
}: ChatInputProps) => {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [error, setError] = useState<string | undefined>();
  const [isFocused, setIsFocused] = useState(false);

  // 自动调整高度
  const adjustHeight = useCallback(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = 'auto';
      const scrollHeight = textarea.scrollHeight;
      const maxHeight = 200; // 最大高度
      textarea.style.height = `${Math.min(scrollHeight, maxHeight)}px`;
    }
  }, []);

  // 处理输入变化
  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    onChange(newValue);
    
    // 清除错误
    if (error) {
      setError(undefined);
    }
    
    // 调整高度
    adjustHeight();
  }, [onChange, error, adjustHeight]);

  // 处理发送
  const handleSend = useCallback(() => {
    if (!value.trim() || disabled || isLoading) return;

    // 验证消息
    const validation = validateChatMessage(value);
    if (!validation.isValid) {
      setError(validation.error);
      return;
    }

    onSend(value.trim());
    setError(undefined);
  }, [value, disabled, isLoading, onSend]);

  // 处理键盘事件
  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter') {
      // 检查IME输入状态
      if (e.nativeEvent.isComposing) {
        return;
      }

      if (e.shiftKey) {
        // Shift+Enter 允许换行
        return;
      }

      // Enter 发送消息
      e.preventDefault();
      handleSend();
    }
  }, [handleSend]); // ✅ 添加handleSend依赖

  // 处理停止
  const handleStop = useCallback(() => {
    onStop?.();
  }, [onStop]);

  // 处理文件上传
  const handleFileUpload = useCallback(() => {
    // TODO: 实现文件上传功能
    console.log('File upload clicked');
  }, []);

  // 处理语音输入
  const handleVoiceInput = useCallback(() => {
    // TODO: 实现语音输入功能
    console.log('Voice input clicked');
  }, []);

  // 聚焦处理
  const handleFocus = useCallback(() => {
    setIsFocused(true);
  }, []);

  const handleBlur = useCallback(() => {
    setIsFocused(false);
  }, []);

  // 自动聚焦
  useEffect(() => {
    if (!disabled && !isLoading) {
      textareaRef.current?.focus();
    }
  }, [disabled, isLoading]);

  // 调整高度
  useEffect(() => {
    adjustHeight();
  }, [value, adjustHeight]);

  const canSend = value.trim().length > 0 && !disabled && !isLoading;
  const showStop = isStreaming && onStop;

  return (
    <div className={cn("relative", className)}>
      {/* 错误提示 */}
      <AnimatePresence>
        {error && (
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            className="absolute -top-12 left-0 right-0 flex items-center gap-2 px-3 py-2 bg-destructive/10 border border-destructive/20 rounded-md text-sm text-destructive"
          >
            <AlertCircle size={14} />
            {error}
          </motion.div>
        )}
      </AnimatePresence>

      {/* 输入容器 */}
      <div className={cn(
        "relative flex items-end gap-2 p-2 border rounded-lg bg-background transition-colors",
        isFocused && "ring-2 ring-ring ring-offset-2",
        error && "border-destructive",
        disabled && "opacity-50"
      )}>
        {/* 文本输入区域 */}
        <div className="flex-1 relative">
          <Textarea
            ref={textareaRef}
            value={value}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            onFocus={handleFocus}
            onBlur={handleBlur}
            placeholder={placeholder}
            disabled={disabled}
            maxLength={maxLength}
            className={cn(
              "min-h-[44px] max-h-[200px] resize-none border-0 bg-transparent p-2 focus-visible:ring-0 focus-visible:ring-offset-0",
              "scrollbar-thin scrollbar-thumb-muted scrollbar-track-transparent"
            )}
            style={{ height: 'auto' }}
          />
          
          {/* 字符计数 */}
          {value.length > maxLength * 0.8 && (
            <div className="absolute bottom-1 right-1 text-xs text-muted-foreground">
              {value.length}/{maxLength}
            </div>
          )}
        </div>

        {/* 工具栏 */}
        <div className="flex items-center gap-1 pb-2">
          <TooltipProvider delayDuration={300}>
            {/* 文件上传 */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleFileUpload}
                  disabled={disabled}
                  className="h-8 w-8 p-0"
                >
                  <Paperclip size={16} />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="top">
                上传文件
              </TooltipContent>
            </Tooltip>

            {/* 语音输入 */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleVoiceInput}
                  disabled={disabled}
                  className="h-8 w-8 p-0"
                >
                  <Mic size={16} />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="top">
                语音输入
              </TooltipContent>
            </Tooltip>

            {/* 发送/停止按钮 */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  size="sm"
                  onClick={showStop ? handleStop : handleSend}
                  disabled={!showStop && (!canSend || disabled)}
                  className={cn(
                    "h-8 w-8 p-0 transition-colors",
                    showStop 
                      ? "bg-destructive hover:bg-destructive/90 text-destructive-foreground"
                      : canSend 
                        ? "bg-primary hover:bg-primary/90 text-primary-foreground"
                        : "bg-muted text-muted-foreground"
                  )}
                >
                  {isLoading ? (
                    <Loader2 size={16} className="animate-spin" />
                  ) : showStop ? (
                    <Square size={16} />
                  ) : (
                    <Send size={16} />
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent side="top">
                {showStop ? "停止生成" : "发送消息 (Enter)"}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </div>
    </div>
  );
});

ChatInput.displayName = 'ChatInput';

export { ChatInput };
