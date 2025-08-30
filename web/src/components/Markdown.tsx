/**
 * Markdown 渲染组件
 * 用于渲染研究结果和其他 Markdown 内容
 * 简化版本，避免复杂依赖
 */

import React from 'react';
import { Copy, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';

// ============================================================================
// 简单的 Markdown 解析和渲染
// ============================================================================

interface MarkdownProps {
  content: string;
  className?: string;
}

const parseMarkdown = (content: string): React.ReactNode => {
  const lines = content.split('\n');
  const elements: React.ReactNode[] = [];
  let currentIndex = 0;

  while (currentIndex < lines.length) {
    const line = lines[currentIndex];

    // 标题
    if (line.startsWith('# ')) {
      elements.push(
        <h1 key={currentIndex} className="text-3xl font-bold mt-8 mb-4 first:mt-0">
          {line.substring(2)}
        </h1>
      );
    } else if (line.startsWith('## ')) {
      elements.push(
        <h2 key={currentIndex} className="text-2xl font-semibold mt-6 mb-3 first:mt-0">
          {line.substring(3)}
        </h2>
      );
    } else if (line.startsWith('### ')) {
      elements.push(
        <h3 key={currentIndex} className="text-xl font-semibold mt-5 mb-2 first:mt-0">
          {line.substring(4)}
        </h3>
      );
    } else if (line.startsWith('#### ')) {
      elements.push(
        <h4 key={currentIndex} className="text-lg font-semibold mt-4 mb-2 first:mt-0">
          {line.substring(5)}
        </h4>
      );
    }
    // 代码块
    else if (line.startsWith('```')) {
      const language = line.substring(3);
      const codeLines: string[] = [];
      currentIndex++;

      while (currentIndex < lines.length && !lines[currentIndex].startsWith('```')) {
        codeLines.push(lines[currentIndex]);
        currentIndex++;
      }

      elements.push(
        <CodeBlock key={currentIndex} language={language}>
          {codeLines.join('\n')}
        </CodeBlock>
      );
    }
    // 列表项
    else if (line.startsWith('- ') || line.startsWith('* ')) {
      const listItems: string[] = [line.substring(2)];
      let nextIndex = currentIndex + 1;

      while (nextIndex < lines.length && (lines[nextIndex].startsWith('- ') || lines[nextIndex].startsWith('* '))) {
        listItems.push(lines[nextIndex].substring(2));
        nextIndex++;
      }

      elements.push(
        <ul key={currentIndex} className="list-disc list-inside my-4 space-y-1">
          {listItems.map((item, index) => (
            <li key={index} className="ml-4">{item}</li>
          ))}
        </ul>
      );

      currentIndex = nextIndex - 1;
    }
    // 引用块
    else if (line.startsWith('> ')) {
      elements.push(
        <blockquote key={currentIndex} className="border-l-4 border-primary pl-4 py-2 my-4 bg-muted/50 italic">
          {line.substring(2)}
        </blockquote>
      );
    }
    // 分隔线
    else if (line.trim() === '---' || line.trim() === '***') {
      elements.push(<hr key={currentIndex} className="my-6 border-border" />);
    }
    // 普通段落
    else if (line.trim()) {
      elements.push(
        <p key={currentIndex} className="my-3 leading-relaxed">
          {formatInlineElements(line)}
        </p>
      );
    }
    // 空行
    else {
      elements.push(<br key={currentIndex} />);
    }

    currentIndex++;
  }

  return elements;
};

const formatInlineElements = (text: string): React.ReactNode => {
  // 简单的内联格式化：粗体、斜体、代码
  let result: React.ReactNode[] = [];
  let currentText = text;
  let key = 0;

  // 处理代码
  currentText = currentText.replace(/`([^`]+)`/g, (match, code) => {
    result.push(
      <code key={key++} className="bg-muted px-1.5 py-0.5 rounded text-sm font-mono">
        {code}
      </code>
    );
    return `__CODE_${result.length - 1}__`;
  });

  // 处理粗体
  currentText = currentText.replace(/\*\*([^*]+)\*\*/g, (match, bold) => {
    result.push(<strong key={key++}>{bold}</strong>);
    return `__BOLD_${result.length - 1}__`;
  });

  // 处理斜体
  currentText = currentText.replace(/\*([^*]+)\*/g, (match, italic) => {
    result.push(<em key={key++}>{italic}</em>);
    return `__ITALIC_${result.length - 1}__`;
  });

  // 重新组装文本
  const parts = currentText.split(/(__(?:CODE|BOLD|ITALIC)_\d+__)/);
  return parts.map((part, index) => {
    const match = part.match(/__(?:CODE|BOLD|ITALIC)_(\d+)__/);
    if (match) {
      return result[parseInt(match[1])];
    }
    return part;
  });
};

// ============================================================================
// 代码块组件
// ============================================================================

interface CodeBlockProps {
  children: string;
  language?: string;
}

const CodeBlock: React.FC<CodeBlockProps> = ({ children, language }) => {
  const [copied, setCopied] = React.useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(children);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error('Failed to copy code:', error);
    }
  };

  return (
    <div className="relative group my-4">
      <div className="absolute right-2 top-2 z-10">
        <Button
          variant="ghost"
          size="sm"
          onClick={handleCopy}
          className="h-8 w-8 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
        >
          {copied ? (
            <Check className="h-3 w-3 text-green-500" />
          ) : (
            <Copy className="h-3 w-3" />
          )}
        </Button>
      </div>
      <pre className="bg-muted p-4 rounded-md overflow-x-auto">
        <code className="text-sm font-mono">
          {children}
        </code>
      </pre>
      {language && (
        <div className="absolute top-2 left-2 text-xs text-muted-foreground bg-background px-2 py-1 rounded">
          {language}
        </div>
      )}
    </div>
  );
};

// ============================================================================
// 主组件
// ============================================================================

export const Markdown: React.FC<MarkdownProps> = ({ content, className = '' }) => {
  return (
    <div className={`prose dark:prose-invert max-w-none ${className}`}>
      {parseMarkdown(content)}
    </div>
  );
};

export default Markdown;
