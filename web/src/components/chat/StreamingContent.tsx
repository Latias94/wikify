/**
 * 流式内容渲染组件
 * 使用 Streamdown 进行优化的 Markdown 渲染
 */

import { memo } from 'react';
import { Streamdown } from 'streamdown';
import { cn } from '@/lib/utils';

interface StreamingContentProps {
  content: string;
  className?: string;
}

const StreamingContent = memo(({
  content,
  className
}: StreamingContentProps) => {
  return (
    <Streamdown
      className={cn("max-w-none", className)}
      parseIncompleteMarkdown={true}
      shikiTheme={['github-light', 'github-dark']}
    >
      {content}
    </Streamdown>
  );
});

StreamingContent.displayName = 'StreamingContent';

export { StreamingContent };

