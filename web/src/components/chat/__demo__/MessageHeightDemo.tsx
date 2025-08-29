/**
 * MessageBubble 高度稳定性演示组件
 * 用于测试和展示修复后的布局稳定性
 */

import { useState } from 'react';
import { MessageBubble } from '../MessageBubble';
import { UIChatMessage } from '@/types/ui';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

const demoMessages: UIChatMessage[] = [
  {
    id: 'demo-1',
    role: 'user',
    content: '请解释一下React的useEffect钩子是如何工作的？',
    timestamp: new Date(Date.now() - 300000),
    isStreaming: false,
    isError: false,
  },
  {
    id: 'demo-2',
    role: 'assistant',
    content: `useEffect 是 React 中最重要的钩子之一，它允许你在函数组件中执行副作用操作。

## 基本语法

\`\`\`javascript
useEffect(() => {
  // 副作用逻辑
  return () => {
    // 清理逻辑（可选）
  };
}, [dependencies]); // 依赖数组（可选）
\`\`\`

## 主要特点

1. **副作用处理**：处理数据获取、订阅、手动DOM操作等
2. **生命周期模拟**：可以模拟类组件的生命周期方法
3. **依赖优化**：通过依赖数组控制执行时机

这个钩子让函数组件具备了处理副作用的能力，是现代React开发的核心工具。`,
    timestamp: new Date(Date.now() - 240000),
    isStreaming: false,
    isError: false,
  },
  {
    id: 'demo-3',
    role: 'user',
    content: '能给个具体的例子吗？',
    timestamp: new Date(Date.now() - 180000),
    isStreaming: false,
    isError: false,
  },
  {
    id: 'demo-4',
    role: 'assistant',
    content: `当然！这里是一个实际的例子：

\`\`\`javascript
import { useState, useEffect } from 'react';

function UserProfile({ userId }) {
  const [user, setUser] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // 数据获取副作用
    async function fetchUser() {
      setLoading(true);
      try {
        const response = await fetch(\`/api/users/\${userId}\`);
        const userData = await response.json();
        setUser(userData);
      } catch (error) {
        console.error('获取用户数据失败:', error);
      } finally {
        setLoading(false);
      }
    }

    fetchUser();
  }, [userId]); // 当 userId 变化时重新执行

  if (loading) return <div>加载中...</div>;
  if (!user) return <div>用户不存在</div>;

  return (
    <div>
      <h1>{user.name}</h1>
      <p>{user.email}</p>
    </div>
  );
}
\`\`\`

这个例子展示了useEffect的典型用法：当组件挂载或userId变化时获取用户数据。`,
    timestamp: new Date(Date.now() - 120000),
    isStreaming: false,
    isError: false,
  },
];

export function MessageHeightDemo() {
  const [showDemo, setShowDemo] = useState(false);
  const [highlightChanges, setHighlightChanges] = useState(false);

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>MessageBubble 高度稳定性演示</CardTitle>
          <CardDescription>
            展示修复后的消息组件在Actions显示/隐藏时保持布局稳定
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-4">
            <Button 
              onClick={() => setShowDemo(!showDemo)}
              variant={showDemo ? "secondary" : "default"}
            >
              {showDemo ? '隐藏演示' : '显示演示'}
            </Button>
            <Button 
              onClick={() => setHighlightChanges(!highlightChanges)}
              variant={highlightChanges ? "secondary" : "outline"}
            >
              {highlightChanges ? '关闭高亮' : '高亮变化区域'}
            </Button>
          </div>

          {showDemo && (
            <div className="border rounded-lg p-4 bg-muted/20">
              <h3 className="text-lg font-semibold mb-4">修复效果对比</h3>
              
              <div className="space-y-4">
                <div className="text-sm text-muted-foreground mb-2">
                  💡 <strong>修复要点：</strong>
                  <ul className="list-disc list-inside mt-1 space-y-1">
                    <li>移除了AnimatePresence，避免DOM元素的添加/移除</li>
                    <li>为MessageActions设置固定的最小宽度和高度</li>
                    <li>使用opacity和pointer-events控制可见性</li>
                    <li>为消息元信息区域设置最小高度</li>
                  </ul>
                </div>

                <div 
                  className={`space-y-6 ${highlightChanges ? 'ring-2 ring-primary ring-offset-2' : ''}`}
                  style={{ 
                    transition: highlightChanges ? 'all 0.3s ease' : 'none',
                  }}
                >
                  {demoMessages.map((message, index) => (
                    <div key={message.id} className="relative">
                      {highlightChanges && (
                        <div className="absolute -inset-2 bg-primary/5 rounded-lg pointer-events-none" />
                      )}
                      <MessageBubble
                        message={message}
                        isLast={index === demoMessages.length - 1}
                        onCopy={(content) => {
                          navigator.clipboard.writeText(content);
                          console.log('已复制:', content.substring(0, 50) + '...');
                        }}
                        onRetry={(msg) => console.log('重试消息:', msg.id)}
                        onRegenerate={(msg) => console.log('重新生成:', msg.id)}
                      />
                    </div>
                  ))}
                </div>
              </div>

              <div className="mt-6 p-4 bg-green-50 dark:bg-green-950/20 rounded-lg border border-green-200 dark:border-green-800">
                <h4 className="font-semibold text-green-800 dark:text-green-200 mb-2">
                  ✅ 修复验证
                </h4>
                <div className="text-sm text-green-700 dark:text-green-300 space-y-1">
                  <p>• 鼠标悬停时消息高度保持稳定</p>
                  <p>• Actions区域始终占用固定空间</p>
                  <p>• 布局不会因为Actions的显示/隐藏而跳动</p>
                  <p>• 滚动位置保持稳定</p>
                </div>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>技术实现细节</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4 text-sm">
            <div>
              <h4 className="font-semibold mb-2">1. 布局稳定性策略</h4>
              <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                <li>为Actions容器设置固定的最小宽度 (120px) 和高度 (28px)</li>
                <li>使用 <code>justify-end</code> 确保Actions右对齐</li>
                <li>为消息元信息区域设置最小高度 (20px)</li>
              </ul>
            </div>

            <div>
              <h4 className="font-semibold mb-2">2. 可见性控制</h4>
              <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                <li>使用 <code>opacity</code> 而非 <code>display</code> 控制可见性</li>
                <li>添加 <code>pointer-events-none</code> 禁用隐藏状态下的交互</li>
                <li>使用 <code>transition-opacity</code> 提供平滑过渡效果</li>
              </ul>
            </div>

            <div>
              <h4 className="font-semibold mb-2">3. 参考最佳实践</h4>
              <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                <li>借鉴 Vercel AI Chatbot 的固定布局空间策略</li>
                <li>避免使用 AnimatePresence 导致的DOM变化</li>
                <li>优先使用CSS transition而非复杂动画</li>
              </ul>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
