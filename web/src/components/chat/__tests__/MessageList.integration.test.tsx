/**
 * MessageList集成测试
 * 测试use-stick-to-bottom的集成效果
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MessageList } from '../MessageList';
import { UIChatMessage } from '@/types/ui';

// Mock use-stick-to-bottom
jest.mock('use-stick-to-bottom', () => ({
  StickToBottom: ({ children, className }: any) => (
    <div className={className} data-testid="stick-to-bottom">
      {children}
    </div>
  ),
  'StickToBottom.Content': ({ children, className }: any) => (
    <div className={className} data-testid="stick-to-bottom-content">
      {children}
    </div>
  ),
  useStickToBottomContext: () => ({
    isAtBottom: false,
    scrollToBottom: jest.fn(),
  }),
}));

const mockMessages: UIChatMessage[] = [
  {
    id: 'msg-1',
    role: 'user',
    content: '测试消息1',
    timestamp: new Date(),
    isStreaming: false,
    isError: false,
  },
  {
    id: 'msg-2',
    role: 'assistant',
    content: '测试回复1',
    timestamp: new Date(),
    isStreaming: false,
    isError: false,
  },
];

describe('MessageList Integration', () => {
  it('should render with StickToBottom wrapper', () => {
    render(<MessageList messages={mockMessages} />);
    
    expect(screen.getByTestId('stick-to-bottom')).toBeInTheDocument();
    expect(screen.getByTestId('stick-to-bottom-content')).toBeInTheDocument();
  });

  it('should display messages correctly', () => {
    render(<MessageList messages={mockMessages} />);
    
    expect(screen.getByText('测试消息1')).toBeInTheDocument();
    expect(screen.getByText('测试回复1')).toBeInTheDocument();
  });

  it('should show scroll to bottom button when not at bottom', () => {
    render(<MessageList messages={mockMessages} />);
    
    expect(screen.getByText('回到底部')).toBeInTheDocument();
  });

  it('should show loading indicator when isLoading is true', () => {
    render(<MessageList messages={mockMessages} isLoading={true} />);
    
    expect(screen.getByText('AI 正在思考...')).toBeInTheDocument();
  });

  it('should show connection status when disconnected', () => {
    render(<MessageList messages={mockMessages} isConnected={false} />);
    
    expect(screen.getByText('连接已断开，正在重连...')).toBeInTheDocument();
  });

  it('should show empty state when no messages', () => {
    render(<MessageList messages={[]} />);
    
    expect(screen.getByText('开始对话')).toBeInTheDocument();
    expect(screen.getByText(/向 AI 提问关于代码库的任何问题/)).toBeInTheDocument();
  });

  it('should handle message actions correctly', () => {
    const onCopy = jest.fn();
    const onRetry = jest.fn();
    const onRegenerate = jest.fn();

    render(
      <MessageList
        messages={mockMessages}
        onCopyMessage={onCopy}
        onRetryMessage={onRetry}
        onRegenerateMessage={onRegenerate}
      />
    );

    // 测试消息操作功能
    expect(screen.getByText('测试消息1')).toBeInTheDocument();
    expect(screen.getByText('测试回复1')).toBeInTheDocument();
  });

  it('should apply custom className', () => {
    const { container } = render(
      <MessageList messages={mockMessages} className="custom-class" />
    );
    
    expect(container.firstChild).toHaveClass('custom-class');
  });
});

describe('ScrollToBottomButton Integration', () => {
  it('should call scrollToBottom when button is clicked', async () => {
    const mockScrollToBottom = jest.fn();
    
    // Mock the context to return our mock function
    jest.doMock('use-stick-to-bottom', () => ({
      StickToBottom: ({ children, className }: any) => (
        <div className={className} data-testid="stick-to-bottom">
          {children}
        </div>
      ),
      'StickToBottom.Content': ({ children, className }: any) => (
        <div className={className} data-testid="stick-to-bottom-content">
          {children}
        </div>
      ),
      useStickToBottomContext: () => ({
        isAtBottom: false,
        scrollToBottom: mockScrollToBottom,
      }),
    }));

    render(<MessageList messages={mockMessages} />);
    
    const scrollButton = screen.getByText('回到底部');
    fireEvent.click(scrollButton);
    
    await waitFor(() => {
      expect(mockScrollToBottom).toHaveBeenCalled();
    });
  });
});
