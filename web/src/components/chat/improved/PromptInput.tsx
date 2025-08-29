/**
 * 改进的输入组件 - 参考Vercel AI Chatbot设计
 * 模块化、可复用的输入组件系统
 */

import { 
  forwardRef, 
  useCallback, 
  type ComponentProps, 
  type HTMLAttributes,
  type KeyboardEventHandler 
} from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';
import { Send, Square, Loader2 } from 'lucide-react';

// ============================================================================
// 基础表单容器
// ============================================================================

export type PromptInputProps = HTMLAttributes<HTMLFormElement>;

export const PromptInput = ({ className, ...props }: PromptInputProps) => (
  <form
    className={cn(
      'w-full overflow-hidden rounded-xl border bg-background shadow-sm',
      'transition-all duration-200 hover:border-primary/20',
      'focus-within:border-primary/30 focus-within:shadow-lg focus-within:shadow-primary/10',
      className,
    )}
    {...props}
  />
);

// ============================================================================
// 文本输入区域
// ============================================================================

export type PromptInputTextareaProps = ComponentProps<typeof Textarea> & {
  minHeight?: number;
  maxHeight?: number;
  disableAutoResize?: boolean;
};

export const PromptInputTextarea = forwardRef<
  HTMLTextAreaElement,
  PromptInputTextareaProps
>(({
  onChange,
  className,
  placeholder = '输入您的问题...',
  minHeight = 48,
  maxHeight = 200,
  disableAutoResize = false,
  ...props
}, ref) => {
  const handleKeyDown: KeyboardEventHandler<HTMLTextAreaElement> = (e) => {
    if (e.key === 'Enter') {
      // 检查IME输入状态 - 支持中文输入
      if (e.nativeEvent.isComposing) {
        return;
      }

      if (e.shiftKey) {
        // Shift+Enter 允许换行
        return;
      }

      // Enter 提交表单
      e.preventDefault();
      const form = e.currentTarget.form;
      if (form) {
        form.requestSubmit();
      }
    }
  };

  return (
    <Textarea
      ref={ref}
      className={cn(
        'w-full resize-none rounded-none border-none p-3 shadow-none outline-none ring-0',
        'bg-transparent focus-visible:ring-0',
        disableAutoResize ? 'field-sizing-fixed' : 'field-sizing-content',
        className,
      )}
      style={{
        minHeight: `${minHeight}px`,
        maxHeight: `${maxHeight}px`,
      }}
      name="message"
      onChange={onChange}
      onKeyDown={handleKeyDown}
      placeholder={placeholder}
      {...props}
    />
  );
});

PromptInputTextarea.displayName = 'PromptInputTextarea';

// ============================================================================
// 工具栏容器
// ============================================================================

export type PromptInputToolbarProps = HTMLAttributes<HTMLDivElement>;

export const PromptInputToolbar = ({
  className,
  ...props
}: PromptInputToolbarProps) => (
  <div
    className={cn('flex items-center justify-between p-2 border-t', className)}
    {...props}
  />
);

// ============================================================================
// 工具按钮组
// ============================================================================

export type PromptInputToolsProps = HTMLAttributes<HTMLDivElement>;

export const PromptInputTools = ({
  className,
  ...props
}: PromptInputToolsProps) => (
  <div
    className={cn('flex items-center gap-1', className)}
    {...props}
  />
);

// ============================================================================
// 提交按钮
// ============================================================================

export type PromptInputSubmitProps = ComponentProps<typeof Button> & {
  status?: 'idle' | 'loading' | 'streaming' | 'error';
  canSend?: boolean;
};

export const PromptInputSubmit = ({
  className,
  variant = 'default',
  size = 'sm',
  status = 'idle',
  canSend = false,
  children,
  disabled,
  ...props
}: PromptInputSubmitProps) => {
  let Icon = <Send size={16} />;
  let buttonVariant = variant;
  let isDisabled = disabled || !canSend;

  if (status === 'loading') {
    Icon = <Loader2 size={16} className="animate-spin" />;
    isDisabled = true;
  } else if (status === 'streaming') {
    Icon = <Square size={16} />;
    buttonVariant = 'destructive';
    isDisabled = false;
  }

  return (
    <Button
      className={cn(
        'h-8 w-8 p-0 transition-colors',
        canSend && status === 'idle' 
          ? 'bg-primary hover:bg-primary/90 text-primary-foreground'
          : 'bg-muted text-muted-foreground',
        className
      )}
      size={size}
      type="submit"
      variant={buttonVariant}
      disabled={isDisabled}
      {...props}
    >
      {children ?? Icon}
    </Button>
  );
};

// ============================================================================
// 组合示例组件
// ============================================================================

interface ImprovedChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: (value: string) => void;
  onStop?: () => void;
  disabled?: boolean;
  isLoading?: boolean;
  isStreaming?: boolean;
  placeholder?: string;
  className?: string;
}

export const ImprovedChatInput = ({
  value,
  onChange,
  onSubmit,
  onStop,
  disabled = false,
  isLoading = false,
  isStreaming = false,
  placeholder,
  className
}: ImprovedChatInputProps) => {
  const handleSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    if (!value.trim() || disabled || isLoading) return;
    
    if (isStreaming && onStop) {
      onStop();
    } else {
      onSubmit(value.trim());
    }
  }, [value, disabled, isLoading, isStreaming, onSubmit, onStop]);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    onChange(e.target.value);
  }, [onChange]);

  const canSend = value.trim().length > 0 && !disabled && !isLoading;
  const status = isLoading ? 'loading' : isStreaming ? 'streaming' : 'idle';

  return (
    <PromptInput className={className} onSubmit={handleSubmit}>
      <PromptInputTextarea
        value={value}
        onChange={handleChange}
        placeholder={placeholder}
        disabled={disabled}
      />
      
      <PromptInputToolbar>
        <PromptInputTools>
          {/* 这里可以添加其他工具按钮，如文件上传等 */}
        </PromptInputTools>
        
        <PromptInputSubmit
          status={status}
          canSend={canSend}
        />
      </PromptInputToolbar>
    </PromptInput>
  );
};
