import { useState, useRef, useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent } from "@/components/ui/card";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useToast } from "@/hooks/use-toast";
import {
  Send,
  ArrowLeft,
  User,
  Bot,
  Copy,
  MessageCircle,
  Wifi,
  WifiOff,
  Loader2,
  AlertCircle,
  Settings,
  FileText,
  ExternalLink,
  RefreshCw
} from "lucide-react";
import { useParams, useNavigate } from "react-router-dom";

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
  useAppStore
} from "@/store/app-store";

// Utils and types
import { UIChatMessage } from "@/types/ui";
import { formatRelativeTime, formatDateTime } from "@/utils/formatters";
import { validateChatMessage } from "@/utils/validators";
import { UI_CONFIG } from "@/lib/constants";

const ChatInterface = () => {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();
  const { toast } = useToast();

  // Refs
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

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

    // 检查会话是否匹配
    if (activeSession?.id !== sessionId) {
      toast({
        title: "Session Not Found",
        description: "The requested chat session was not found",
        variant: "destructive"
      });
      navigate('/');
      return;
    }
  }, [sessionId, activeSession, navigate, toast]);

  // 自动滚动到底部
  useEffect(() => {
    scrollToBottom();
  }, [messages, streamingMessage]);

  // 聚焦输入框
  useEffect(() => {
    if (inputRef.current && !isLoading) {
      inputRef.current.focus();
    }
  }, [isLoading]);

  // 处理函数
  const scrollToBottom = useCallback(() => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, []);

  const handleSendMessage = useCallback(async () => {
    if (!currentInput.trim() || isLoading || !sessionId) return;

    // 验证消息
    const validation = validateChatMessage(currentInput);
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
      sendMessage(currentInput.trim());
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
  }, [currentInput, isLoading, sessionId, wsConnected, sendMessage, clearCurrentInput, clearChatError, setChatError, toast, reconnect]);

  const handleInputChange = useCallback((value: string) => {
    setCurrentInput(value);
  }, [setCurrentInput]);

  const handleKeyPress = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  }, [handleSendMessage]);

  const handleCopyMessage = useCallback((content: string) => {
    navigator.clipboard.writeText(content).then(() => {
      toast({
        title: "Copied",
        description: "Message copied to clipboard",
      });
    }).catch(() => {
      toast({
        title: "Copy Failed",
        description: "Failed to copy message to clipboard",
        variant: "destructive"
      });
    });
  }, [toast]);

  const handleRetryMessage = useCallback((message: UIChatMessage) => {
    if (message.role === 'user') {
      setCurrentInput(message.content);
      if (inputRef.current) {
        inputRef.current.focus();
      }
    }
  }, [setCurrentInput]);

  const handleGoBack = useCallback(() => {
    navigate('/');
  }, [navigate]);

  const handleReconnect = useCallback(() => {
    reconnect();
    toast({
      title: "Reconnecting",
      description: "Attempting to reconnect to chat service...",
    });
  }, [reconnect, toast]);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const handleSendMessage = async () => {
    if (!newMessage.trim() || isLoading) return;

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      type: 'user',
      content: newMessage,
      timestamp: new Date()
    };

    setMessages(prev => [...prev, userMessage]);
    setNewMessage('');
    setIsLoading(true);

    // Simulate AI response
    setTimeout(() => {
      const aiMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        type: 'ai',
        content: `I understand you're asking about: "${newMessage}". Let me analyze the codebase and provide you with detailed information...

This is a simulated response. In a real implementation, this would connect to your Wikify backend to process the repository and generate intelligent responses about your code.`,
        timestamp: new Date()
      };
      
      setMessages(prev => [...prev, aiMessage]);
      setIsLoading(false);
    }, 1500);
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    toast({
      title: "Copied to clipboard",
      description: "Message content has been copied"
    });
  };

  const formatMessage = (content: string) => {
    // Simple markdown-like formatting for code blocks
    const parts = content.split(/```(\w+)?\n([\s\S]*?)```/);
    
    return parts.map((part, index) => {
      if (index % 3 === 2) {
        // This is code content
        const language = parts[index - 1] || 'text';
        return (
          <div key={index} className="relative my-4">
            <div className="flex items-center justify-between px-4 py-2 bg-muted rounded-t-lg border-b">
              <span className="text-xs font-medium text-muted-foreground">{language}</span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => copyToClipboard(part)}
                className="h-6 w-6 p-0"
              >
                <Copy className="h-3 w-3" />
              </Button>
            </div>
            <pre className="bg-card border border-t-0 rounded-b-lg p-4 overflow-x-auto">
              <code className="text-sm">{part}</code>
            </pre>
          </div>
        );
      } else if (index % 3 === 0) {
        // Regular text
        return part ? (
          <div key={index} className="whitespace-pre-wrap">
            {part}
          </div>
        ) : null;
      }
      return null;
    });
  };

  return (
    <div className="min-h-screen flex flex-col">
      {/* Header */}
      <header className="sticky top-0 z-50 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-14 items-center gap-4">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => navigate('/')}
            className="flex items-center gap-2"
          >
            <ArrowLeft className="h-4 w-4" />
            Back
          </Button>
          
          <Separator orientation="vertical" className="h-6" />
          
          <div className="flex items-center gap-2">
            <MessageCircle className="h-5 w-5 text-primary" />
            <h1 className="font-semibold">wikify-rust</h1>
          </div>
          
          <div className="flex-1" />
          
          <div className="flex items-center gap-2">
            {isConnected ? (
              <div className="flex items-center gap-1 text-success">
                <Wifi className="h-4 w-4" />
                <span className="text-sm">Connected</span>
              </div>
            ) : (
              <div className="flex items-center gap-1 text-destructive">
                <WifiOff className="h-4 w-4" />
                <span className="text-sm">Disconnected</span>
              </div>
            )}
          </div>
        </div>
      </header>

      {/* Chat Messages */}
      <main className="flex-1 container py-6">
        <div className="max-w-4xl mx-auto space-y-4">
          {messages.map((message) => (
            <div
              key={message.id}
              className={`flex gap-3 ${
                message.type === 'user' ? 'justify-end' : 'justify-start'
              }`}
            >
              {message.type !== 'user' && (
                <Avatar className="w-8 h-8 mt-1">
                  <AvatarFallback className="bg-primary text-primary-foreground">
                    {message.type === 'ai' ? <Bot className="h-4 w-4" /> : 'S'}
                  </AvatarFallback>
                </Avatar>
              )}
              
              <Card className={`max-w-2xl shadow-soft ${
                message.type === 'user' 
                  ? 'bg-primary text-primary-foreground' 
                  : message.type === 'system'
                  ? 'bg-muted'
                  : 'bg-card'
              }`}>
                <CardContent className="p-4">
                  <div className="space-y-2">
                    {message.type === 'system' ? (
                      <div className="flex items-center gap-2">
                        <Badge variant="secondary" className="text-xs">
                          System
                        </Badge>
                        <span className="text-sm">{message.content}</span>
                      </div>
                    ) : (
                      <div className="prose prose-sm max-w-none">
                        {formatMessage(message.content)}
                      </div>
                    )}
                    
                    <div className="flex items-center justify-between text-xs opacity-70 mt-2">
                      <span>
                        {message.timestamp.toLocaleTimeString([], { 
                          hour: '2-digit', 
                          minute: '2-digit' 
                        })}
                      </span>
                      
                      {message.type !== 'system' && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => copyToClipboard(message.content)}
                          className="h-5 w-5 p-0 opacity-50 hover:opacity-100"
                        >
                          <Copy className="h-3 w-3" />
                        </Button>
                      )}
                    </div>
                  </div>
                </CardContent>
              </Card>
              
              {message.type === 'user' && (
                <Avatar className="w-8 h-8 mt-1">
                  <AvatarFallback className="bg-secondary text-secondary-foreground">
                    <User className="h-4 w-4" />
                  </AvatarFallback>
                </Avatar>
              )}
            </div>
          ))}
          
          {isLoading && (
            <div className="flex gap-3 justify-start">
              <Avatar className="w-8 h-8 mt-1">
                <AvatarFallback className="bg-primary text-primary-foreground">
                  <Bot className="h-4 w-4" />
                </AvatarFallback>
              </Avatar>
              
              <Card className="max-w-2xl shadow-soft bg-card">
                <CardContent className="p-4">
                  <div className="flex items-center gap-2">
                    <div className="flex space-x-1">
                      <div className="w-2 h-2 bg-primary rounded-full animate-bounce" />
                      <div className="w-2 h-2 bg-primary rounded-full animate-bounce [animation-delay:0.1s]" />
                      <div className="w-2 h-2 bg-primary rounded-full animate-bounce [animation-delay:0.2s]" />
                    </div>
                    <span className="text-sm text-muted-foreground">AI is thinking...</span>
                  </div>
                </CardContent>
              </Card>
            </div>
          )}
          
          <div ref={messagesEndRef} />
        </div>
      </main>

      {/* Message Input */}
      <footer className="sticky bottom-0 border-t bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container py-4">
          <div className="max-w-4xl mx-auto">
            <div className="flex gap-2">
              <Input
                placeholder="Ask about this repository..."
                value={newMessage}
                onChange={(e) => setNewMessage(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    handleSendMessage();
                  }
                }}
                className="flex-1"
                disabled={!isConnected || isLoading}
              />
              <Button
                onClick={handleSendMessage}
                disabled={!newMessage.trim() || !isConnected || isLoading}
                size="sm"
                className="px-4"
              >
                <Send className="h-4 w-4" />
              </Button>
            </div>
            
            <div className="flex items-center justify-between mt-2 text-xs text-muted-foreground">
              <span>Press Enter to send, Shift+Enter for new line</span>
              <span>{newMessage.length} characters</span>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
};

export default ChatInterface;
