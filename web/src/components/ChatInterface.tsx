/**
 * 聊天界面组件 - 重构版本
 * 使用新的子组件和改进的架构
 */

import { useState, useCallback, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { useToast } from "@/hooks/use-toast";
import { 
  ArrowLeft, 
  MessageCircle,
  Wifi,
  WifiOff,
  AlertCircle,
  RefreshCw
} from "lucide-react";
import { useParams, useNavigate } from "react-router-dom";

// Chat components
import { MessageList } from "@/components/chat/MessageList";
import { ChatInput } from "@/components/chat/ChatInput";

// Hooks and stores
import { useChatWebSocket } from "@/hooks/use-websocket";
import { 
  useMessages,
  useCurrentInput,
  useIsConnected,
  useStreamingMessage,
  useChatError,
  useChatStore
} from "@/store/chat-store";
import { 
  useActiveSession,
  useSelectedRepository,
} from "@/store/app-store";

// Utils and types
import { UIChatMessage } from "@/types/ui";
import { validateChatMessage } from "@/utils/validators";

const ChatInterface = () => {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();
  const { toast } = useToast();

  // Store state
  const activeSession = useActiveSession();
  const selectedRepository = useSelectedRepository();
  const messages = useMessages(sessionId || '');
  const currentInput = useCurrentInput();
  const isConnected = useIsConnected();
  const streamingMessage = useStreamingMessage();
  const chatError = useChatError(sessionId || '');

  // Store actions
  const {
    setCurrentInput,
    clearCurrentInput,
    setError: setChatError,
    clearError: clearChatError
  } = useChatStore();

  // WebSocket connection
  const { sendMessage, isConnected: wsConnected, reconnect } = useChatWebSocket(sessionId);

  // Local state
  const [isLoading, setIsLoading] = useState(false);

  // 处理函数
  const handleSendMessage = useCallback(async (message: string) => {
    if (!message.trim() || isLoading || !sessionId) return;
    
    // 验证消息
    const validation = validateChatMessage(message);
    if (!validation.isValid) {
      toast({
        title: "Invalid Message",
        description: validation.error,
        variant: "destructive"
      });
      return;
    }
    
    // 检查连接状态
    if (!wsConnected) {
      toast({
        title: "Connection Error",
        description: "Not connected to chat service. Trying to reconnect...",
        variant: "destructive"
      });
      reconnect();
      return;
    }
    
    setIsLoading(true);
    clearChatError(sessionId);
    
    try {
      // 发送消息
      sendMessage(message.trim());
      clearCurrentInput();
    } catch (error) {
      console.error('Failed to send message:', error);
      setChatError(sessionId, 'Failed to send message');
      toast({
        title: "Send Failed",
        description: "Failed to send message. Please try again.",
        variant: "destructive"
      });
    } finally {
      setIsLoading(false);
    }
  }, [isLoading, sessionId, wsConnected, sendMessage, clearCurrentInput, clearChatError, setChatError, toast, reconnect]);

  const handleInputChange = useCallback((value: string) => {
    setCurrentInput(value);
  }, [setCurrentInput]);

  const handleCopyMessage = useCallback((content: string) => {
    toast({
      title: "已复制",
      description: "消息已复制到剪贴板",
    });
  }, [toast]);

  const handleRetryMessage = useCallback((message: UIChatMessage) => {
    if (message.role === 'user') {
      setCurrentInput(message.content);
    }
  }, [setCurrentInput]);

  const handleRegenerateMessage = useCallback((message: UIChatMessage) => {
    // TODO: 实现重新生成功能
    toast({
      title: "重新生成",
      description: "重新生成功能即将推出",
    });
  }, [toast]);

  const handleGoBack = useCallback(() => {
    navigate('/');
  }, [navigate]);

  const handleReconnect = useCallback(() => {
    reconnect();
    toast({
      title: "重新连接",
      description: "正在尝试重新连接到聊天服务...",
    });
  }, [reconnect, toast]);

  // Effects
  useEffect(() => {
    // 验证会话是否存在
    if (!sessionId) {
      toast({
        title: "Invalid Session",
        description: "No session ID provided",
        variant: "destructive"
      });
      navigate('/');
      return;
    }

    // 在 Wikify 中，sessionId 就是 repository 的 ID
    // 我们不需要预先设置 activeSession，直接使用 sessionId 进行聊天
    console.log('Starting chat with session ID:', sessionId);
  }, [sessionId, navigate, toast]);

  // 如果没有会话ID，显示错误
  if (!sessionId) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <h2 className="text-xl font-semibold mb-2">无效的会话</h2>
          <p className="text-muted-foreground">请选择一个有效的聊天会话。</p>
          <Button onClick={handleGoBack} className="mt-4">
            返回首页
          </Button>
        </div>
      </div>
    );
  }

  // 在 Wikify 中，我们直接使用 sessionId，不需要检查 activeSession

  return (
    <motion.div 
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className="min-h-screen flex flex-col bg-background"
    >
      {/* Header */}
      <header className="sticky top-0 z-50 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-14 items-center gap-4">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleGoBack}
            className="flex items-center gap-2"
          >
            <ArrowLeft className="h-4 w-4" />
            返回
          </Button>
          
          <Separator orientation="vertical" className="h-6" />
          
          <div className="flex items-center gap-2">
            <MessageCircle className="h-5 w-5 text-primary" />
            <h1 className="font-semibold">
              {selectedRepository?.name || activeSession?.name || '聊天会话'}
            </h1>
          </div>
          
          <div className="flex-1" />
          
          {/* 连接状态 */}
          <div className="flex items-center gap-2">
            {wsConnected ? (
              <div className="flex items-center gap-1 text-green-600">
                <Wifi className="h-4 w-4" />
                <span className="text-sm">已连接</span>
              </div>
            ) : (
              <div className="flex items-center gap-1 text-destructive">
                <WifiOff className="h-4 w-4" />
                <span className="text-sm">未连接</span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleReconnect}
                  className="h-6 w-6 p-0 ml-1"
                >
                  <RefreshCw className="h-3 w-3" />
                </Button>
              </div>
            )}
          </div>
        </div>
      </header>

      {/* 错误提示 */}
      <AnimatePresence>
        {chatError && (
          <motion.div
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            className="border-b bg-destructive/10 border-destructive/20"
          >
            <div className="container flex items-center gap-2 py-3">
              <AlertCircle className="h-4 w-4 text-destructive" />
              <span className="text-sm text-destructive">{chatError}</span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => clearChatError(sessionId || '')}
                className="ml-auto h-6 w-6 p-0"
              >
                ×
              </Button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* 主要内容区域 */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* 消息列表 */}
        <MessageList
          messages={messages}
          isLoading={isLoading}
          isConnected={wsConnected}
          onRetryMessage={handleRetryMessage}
          onRegenerateMessage={handleRegenerateMessage}
          onCopyMessage={handleCopyMessage}
          className="flex-1"
        />

        {/* 输入区域 */}
        <div className="border-t bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
          <div className="container py-4">
            <ChatInput
              value={currentInput}
              onChange={handleInputChange}
              onSend={handleSendMessage}
              disabled={!wsConnected}
              isLoading={isLoading}
              isStreaming={!!streamingMessage}
              placeholder="输入您的问题..."
              className="max-w-4xl mx-auto"
            />
          </div>
        </div>
      </div>
    </motion.div>
  );
};

export { ChatInterface };
