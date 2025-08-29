/**
 * 滚动到底部Hook - 参考Vercel AI Chatbot实现
 * 提供智能的滚动控制和状态管理
 */

import { useCallback, useEffect, useRef, useState } from 'react';

interface UseScrollToBottomOptions {
  threshold?: number;
  behavior?: ScrollBehavior;
  debounceMs?: number;
}

interface UseScrollToBottomReturn {
  isAtBottom: boolean;
  scrollToBottom: (behavior?: ScrollBehavior) => void;
  scrollContainerRef: React.RefObject<HTMLElement>;
  messagesEndRef: React.RefObject<HTMLElement>;
}

export function useScrollToBottom({
  threshold = 100,
  behavior = 'smooth',
  debounceMs = 100
}: UseScrollToBottomOptions = {}): UseScrollToBottomReturn {
  const scrollContainerRef = useRef<HTMLElement>(null);
  const messagesEndRef = useRef<HTMLElement>(null);
  const [isAtBottom, setIsAtBottom] = useState(true);
  const scrollTimeoutRef = useRef<NodeJS.Timeout>();

  // 检查是否在底部
  const checkIsAtBottom = useCallback(() => {
    const container = scrollContainerRef.current;
    if (!container) return true;

    const { scrollTop, scrollHeight, clientHeight } = container;
    const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
    
    return distanceFromBottom <= threshold;
  }, [threshold]);

  // 滚动到底部
  const scrollToBottom = useCallback((scrollBehavior: ScrollBehavior = behavior) => {
    const endElement = messagesEndRef.current;
    if (endElement) {
      endElement.scrollIntoView({ 
        behavior: scrollBehavior,
        block: 'end',
        inline: 'nearest'
      });
    }
  }, [behavior]);

  // 防抖的滚动检查
  const debouncedCheckIsAtBottom = useCallback(() => {
    if (scrollTimeoutRef.current) {
      clearTimeout(scrollTimeoutRef.current);
    }
    
    scrollTimeoutRef.current = setTimeout(() => {
      const newIsAtBottom = checkIsAtBottom();
      setIsAtBottom(newIsAtBottom);
    }, debounceMs);
  }, [checkIsAtBottom, debounceMs]);

  // 监听滚动事件
  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const handleScroll = () => {
      debouncedCheckIsAtBottom();
    };

    container.addEventListener('scroll', handleScroll, { passive: true });
    
    // 初始检查
    debouncedCheckIsAtBottom();

    return () => {
      container.removeEventListener('scroll', handleScroll);
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current);
      }
    };
  }, [debouncedCheckIsAtBottom]);

  // 监听容器大小变化
  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const resizeObserver = new ResizeObserver(() => {
      // 如果之前在底部，保持在底部
      if (isAtBottom) {
        scrollToBottom('auto');
      }
    });

    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
    };
  }, [isAtBottom, scrollToBottom]);

  return {
    isAtBottom,
    scrollToBottom,
    scrollContainerRef,
    messagesEndRef
  };
}

// 简化版本的Hook，用于基本场景
export function useAutoScrollToBottom(dependencies: React.DependencyList = []) {
  const { scrollToBottom, messagesEndRef } = useScrollToBottom();

  useEffect(() => {
    scrollToBottom();
  }, dependencies);

  return { scrollToBottom, messagesEndRef };
}

// 智能滚动Hook - 只在用户没有主动滚动时自动滚动
export function useSmartScrollToBottom() {
  const { isAtBottom, scrollToBottom, scrollContainerRef, messagesEndRef } = useScrollToBottom();
  const [userHasScrolled, setUserHasScrolled] = useState(false);
  const lastScrollTopRef = useRef(0);

  // 检测用户主动滚动
  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const handleScroll = () => {
      const { scrollTop } = container;
      const scrollDelta = Math.abs(scrollTop - lastScrollTopRef.current);
      
      // 如果滚动距离超过阈值，认为是用户主动滚动
      if (scrollDelta > 10) {
        setUserHasScrolled(!isAtBottom);
      }
      
      lastScrollTopRef.current = scrollTop;
    };

    container.addEventListener('scroll', handleScroll, { passive: true });
    
    return () => {
      container.removeEventListener('scroll', handleScroll);
    };
  }, [isAtBottom]);

  // 智能滚动函数
  const smartScrollToBottom = useCallback(() => {
    if (!userHasScrolled || isAtBottom) {
      scrollToBottom();
      setUserHasScrolled(false);
    }
  }, [userHasScrolled, isAtBottom, scrollToBottom]);

  // 强制滚动到底部（重置用户滚动状态）
  const forceScrollToBottom = useCallback(() => {
    scrollToBottom();
    setUserHasScrolled(false);
  }, [scrollToBottom]);

  return {
    isAtBottom,
    userHasScrolled,
    smartScrollToBottom,
    forceScrollToBottom,
    scrollContainerRef,
    messagesEndRef
  };
}
