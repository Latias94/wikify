/**
 * Markdown渲染效果演示组件
 * 展示Streamdown的各种功能
 */

import { useState } from 'react';
import { StreamingContent } from '../StreamingContent';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';

const markdownExamples = {
  basic: `# Wikify Markdown 支持演示

## 基础语法

这是一个段落，包含 **粗体文本**、*斜体文本* 和 \`行内代码\`。

### 列表

**无序列表：**
- 项目 1
- 项目 2
  - 嵌套项目 2.1
  - 嵌套项目 2.2
- 项目 3

**有序列表：**
1. 第一步
2. 第二步
3. 第三步

### 引用

> 这是一个引用块。
> 
> 它可以包含多个段落，并且支持其他Markdown语法。
> 
> — 引用来源

### 链接和图片

[访问 Wikify 项目](https://github.com/your-org/wikify)

---

这是一个水平分割线。`,

  code: `# 代码高亮演示

## TypeScript 代码

\`\`\`typescript
interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  isStreaming?: boolean;
}

const renderMessage = (message: ChatMessage): JSX.Element => {
  return (
    <StreamingContent 
      content={message.content}
      className="prose prose-sm dark:prose-invert max-w-none"
    />
  );
};

// 使用泛型的工具函数
function createMessage<T extends ChatMessage>(
  data: Omit<T, 'id' | 'timestamp'>
): T {
  return {
    ...data,
    id: crypto.randomUUID(),
    timestamp: new Date(),
  } as T;
}
\`\`\`

## Rust 代码

\`\`\`rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
}

impl ChatMessage {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: chrono::Utc::now(),
        }
    }
}
\`\`\`

## Python 代码

\`\`\`python
from dataclasses import dataclass
from datetime import datetime
from typing import Literal, Optional
import uuid

@dataclass
class ChatMessage:
    id: str
    role: Literal['user', 'assistant']
    content: str
    timestamp: datetime
    is_streaming: Optional[bool] = False
    
    @classmethod
    def create(cls, role: Literal['user', 'assistant'], content: str) -> 'ChatMessage':
        return cls(
            id=str(uuid.uuid4()),
            role=role,
            content=content,
            timestamp=datetime.now()
        )

def render_message(message: ChatMessage) -> str:
    """渲染消息内容"""
    return f"[{message.timestamp}] {message.role}: {message.content}"
\`\`\``,

  math: `# 数学公式演示

## 行内公式

爱因斯坦的质能方程：$E = mc^2$

圆的面积公式：$A = \\pi r^2$

二次方程的解：$x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$

## 块级公式

**积分公式：**

$$
\\int_{-\\infty}^{\\infty} e^{-x^2} dx = \\sqrt{\\pi}
$$

**矩阵表示：**

$$
\\begin{pmatrix}
a & b \\\\
c & d
\\end{pmatrix}
\\begin{pmatrix}
x \\\\
y
\\end{pmatrix}
=
\\begin{pmatrix}
ax + by \\\\
cx + dy
\\end{pmatrix}
$$

**求和公式：**

$$
\\sum_{n=1}^{\\infty} \\frac{1}{n^2} = \\frac{\\pi^2}{6}
$$

**微分方程：**

$$
\\frac{d^2y}{dx^2} + \\omega^2 y = 0
$$

解为：$y(x) = A\\cos(\\omega x) + B\\sin(\\omega x)$`,

  table: `# 表格演示

## 功能对比表

| 功能 | Vercel AI Chatbot | Wikify | 状态 |
|------|-------------------|--------|------|
| Markdown渲染 | ✅ Streamdown | ✅ Streamdown | 🟢 已实现 |
| 代码高亮 | ✅ Shiki | ✅ Shiki | 🟢 已实现 |
| 数学公式 | ✅ KaTeX | ✅ KaTeX | 🟢 已实现 |
| Mermaid图表 | ✅ | ✅ | 🟢 已实现 |
| 流式渲染 | ✅ | ✅ | 🟢 已实现 |
| 滚动优化 | ✅ use-stick-to-bottom | ✅ use-stick-to-bottom | 🟢 已实现 |

## 技术栈对比

| 组件 | 我们的选择 | 优势 |
|------|------------|------|
| **前端框架** | React + TypeScript | 类型安全，开发效率高 |
| **UI库** | Radix UI + Tailwind | 无障碍访问，高度可定制 |
| **状态管理** | Zustand | 轻量级，易于使用 |
| **数据获取** | React Query | 强大的缓存和同步 |
| **WebSocket** | 原生 + 重连机制 | 稳定的实时通信 |
| **后端** | Rust + Tokio | 高性能，内存安全 |

## 性能指标

| 指标 | 目标值 | 当前值 | 状态 |
|------|--------|--------|------|
| 首屏加载时间 | < 2s | ~1.5s | ✅ |
| 消息渲染延迟 | < 100ms | ~50ms | ✅ |
| 内存使用 | < 100MB | ~80MB | ✅ |
| 包大小 | < 500KB | ~450KB | ✅ |`,

  mermaid: `# Mermaid 图表演示

## 流程图

\`\`\`mermaid
graph TD
    A[用户输入消息] --> B{消息类型检测}
    B -->|普通文本| C[直接发送]
    B -->|包含代码| D[语法高亮处理]
    B -->|包含公式| E[LaTeX渲染]
    C --> F[WebSocket传输]
    D --> F
    E --> F
    F --> G[后端处理]
    G --> H[AI模型推理]
    H --> I[流式响应]
    I --> J[前端实时渲染]
    J --> K[Markdown解析]
    K --> L[用户界面显示]
\`\`\`

## 时序图

\`\`\`mermaid
sequenceDiagram
    participant U as 用户
    participant F as 前端
    participant W as WebSocket
    participant B as 后端
    participant A as AI模型

    U->>F: 输入消息
    F->>W: 发送消息
    W->>B: 转发消息
    B->>A: 调用AI模型
    A-->>B: 开始流式响应
    B-->>W: 转发响应流
    W-->>F: 实时数据
    F-->>U: 渲染Markdown
    
    Note over F,U: 支持实时Markdown渲染
    Note over B,A: 支持多种AI模型
\`\`\`

## 系统架构图

\`\`\`mermaid
graph LR
    subgraph "前端 (React)"
        UI[用户界面]
        MD[Markdown渲染]
        WS[WebSocket客户端]
    end
    
    subgraph "后端 (Rust)"
        API[API服务器]
        RAG[RAG引擎]
        VDB[向量数据库]
    end
    
    subgraph "AI服务"
        LLM[大语言模型]
        EMB[嵌入模型]
    end
    
    UI --> MD
    UI --> WS
    WS <--> API
    API --> RAG
    RAG --> VDB
    RAG --> LLM
    RAG --> EMB
\`\`\`

## 状态图

\`\`\`mermaid
stateDiagram-v2
    [*] --> 连接中
    连接中 --> 已连接: WebSocket连接成功
    连接中 --> 连接失败: 连接超时
    连接失败 --> 重连中: 自动重连
    重连中 --> 已连接: 重连成功
    重连中 --> 连接失败: 重连失败
    已连接 --> 发送中: 用户发送消息
    发送中 --> 接收中: 消息发送成功
    接收中 --> 已连接: 接收完成
    已连接 --> 断开连接: 用户关闭
    断开连接 --> [*]
\`\`\``,
};

export function MarkdownDemo() {
  const [selectedExample, setSelectedExample] = useState<keyof typeof markdownExamples>('basic');

  return (
    <div className="max-w-6xl mx-auto p-6 space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            Markdown 渲染效果演示
            <Badge variant="secondary">Streamdown 驱动</Badge>
          </CardTitle>
          <CardDescription>
            展示 Wikify 聊天界面的 Markdown 渲染能力，与 Vercel AI Chatbot 相同的技术栈
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Tabs value={selectedExample} onValueChange={(value) => setSelectedExample(value as keyof typeof markdownExamples)}>
            <TabsList className="grid w-full grid-cols-5">
              <TabsTrigger value="basic">基础语法</TabsTrigger>
              <TabsTrigger value="code">代码高亮</TabsTrigger>
              <TabsTrigger value="math">数学公式</TabsTrigger>
              <TabsTrigger value="table">表格</TabsTrigger>
              <TabsTrigger value="mermaid">Mermaid图表</TabsTrigger>
            </TabsList>
            
            {Object.entries(markdownExamples).map(([key, content]) => (
              <TabsContent key={key} value={key} className="mt-6">
                <div className="border rounded-lg p-6 bg-background">
                  <StreamingContent 
                    content={content}
                    className="prose prose-sm dark:prose-invert max-w-none"
                  />
                </div>
              </TabsContent>
            ))}
          </Tabs>
          
          <div className="mt-6 p-4 bg-muted/20 rounded-lg">
            <h4 className="font-semibold mb-2">✨ 技术特性</h4>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                Streamdown 渲染
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                Shiki 代码高亮
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                KaTeX 数学公式
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                Mermaid 图表
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                流式渲染优化
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                主题自适应
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                安全性防护
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                移动端优化
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
