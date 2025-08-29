/**
 * MessageBubble 组件测试
 * 测试高度稳定性和布局一致性
 */

import { render, screen, fireEvent } from '@testing-library/react';
import { MessageBubble } from '../MessageBubble';
import { UIChatMessage } from '@/types/ui';

// Mock toast hook
jest.mock('@/hooks/use-toast', () => ({
  useToast: () => ({
    toast: jest.fn(),
  }),
}));

const mockMessage: UIChatMessage = {
  id: 'test-message-1',
  role: 'assistant',
  content: 'This is a test message',
  timestamp: new Date(),
  isStreaming: false,
  isError: false,
};

const mockUserMessage: UIChatMessage = {
  id: 'test-message-2',
  role: 'user',
  content: 'This is a user message',
  timestamp: new Date(),
  isStreaming: false,
  isError: false,
};

describe('MessageBubble', () => {
  it('should render message content correctly', () => {
    render(<MessageBubble message={mockMessage} />);
    expect(screen.getByText('This is a test message')).toBeInTheDocument();
  });

  it('should maintain consistent height when actions appear', () => {
    const { container } = render(<MessageBubble message={mockMessage} />);
    const messageElement = container.querySelector('[data-testid="message-assistant"]');
    
    // 获取初始高度
    const initialHeight = messageElement?.getBoundingClientRect().height;
    
    // 触发hover显示actions
    if (messageElement) {
      fireEvent.mouseEnter(messageElement);
    }
    
    // 检查高度是否保持一致
    const heightAfterHover = messageElement?.getBoundingClientRect().height;
    expect(heightAfterHover).toBe(initialHeight);
  });

  it('should show actions on hover for assistant messages', () => {
    const { container } = render(<MessageBubble message={mockMessage} />);
    const messageElement = container.querySelector('[data-testid="message-assistant"]');
    
    if (messageElement) {
      fireEvent.mouseEnter(messageElement);
    }
    
    // Actions应该可见但可能透明度为0
    const actionsContainer = container.querySelector('.min-w-\\[120px\\]');
    expect(actionsContainer).toBeInTheDocument();
  });

  it('should show actions for last message regardless of hover', () => {
    render(<MessageBubble message={mockMessage} isLast={true} />);
    
    // 最后一条消息的actions应该始终可见
    const actionsContainer = screen.getByRole('button', { name: /复制/i });
    expect(actionsContainer).toBeInTheDocument();
  });

  it('should handle user messages correctly', () => {
    render(<MessageBubble message={mockUserMessage} />);
    expect(screen.getByText('This is a user message')).toBeInTheDocument();
    expect(screen.getByTestId('message-user')).toBeInTheDocument();
  });

  it('should call onCopy when copy button is clicked', () => {
    const onCopy = jest.fn();
    render(<MessageBubble message={mockMessage} onCopy={onCopy} isLast={true} />);
    
    const copyButton = screen.getByRole('button', { name: /复制/i });
    fireEvent.click(copyButton);
    
    expect(onCopy).toHaveBeenCalledWith('This is a test message');
  });

  it('should show retry option for user messages', () => {
    const onRetry = jest.fn();
    render(<MessageBubble message={mockUserMessage} onRetry={onRetry} isLast={true} />);
    
    const retryButton = screen.getByRole('button', { name: /重试/i });
    expect(retryButton).toBeInTheDocument();
  });

  it('should show regenerate option for assistant messages', () => {
    const onRegenerate = jest.fn();
    render(<MessageBubble message={mockMessage} onRegenerate={onRegenerate} isLast={true} />);
    
    const regenerateButton = screen.getByRole('button', { name: /重新生成/i });
    expect(regenerateButton).toBeInTheDocument();
  });

  it('should handle streaming messages', () => {
    const streamingMessage = { ...mockMessage, isStreaming: true };
    render(<MessageBubble message={streamingMessage} />);
    
    // 应该显示流式内容组件
    expect(screen.getByText('This is a test message')).toBeInTheDocument();
  });

  it('should handle error messages', () => {
    const errorMessage = { ...mockMessage, isError: true };
    render(<MessageBubble message={errorMessage} />);
    
    // 错误消息应该有特殊样式
    const messageElement = screen.getByTestId('message-assistant');
    expect(messageElement).toHaveClass('group/message');
  });
});
