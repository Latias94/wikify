/**
 * 源文档组件
 * 显示 AI 回答的相关源文档
 */

import { memo, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { 
  FileText, 
  ExternalLink, 
  ChevronDown, 
  ChevronRight,
  Copy,
  Hash
} from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { SourceDocument } from '@/types/api';
import { cn } from '@/lib/utils';
import { formatFilePath, detectLanguage } from '@/utils/formatters';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark, oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { useTheme } from 'next-themes';

interface SourceDocumentsProps {
  sources: SourceDocument[];
  className?: string;
  maxHeight?: number;
}

interface SourceDocumentItemProps {
  source: SourceDocument;
  index: number;
}

const SourceDocumentItem = memo(({ source, index }: SourceDocumentItemProps) => {
  const { theme } = useTheme();
  const { toast } = useToast();
  const [isExpanded, setIsExpanded] = useState(false);

  const handleToggleExpand = useCallback(() => {
    setIsExpanded(prev => !prev);
  }, []);

  const handleCopyContent = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(source.content);
      toast({
        title: "已复制",
        description: "源文档内容已复制到剪贴板",
      });
    } catch (error) {
      toast({
        title: "复制失败",
        description: "无法复制内容到剪贴板",
        variant: "destructive"
      });
    }
  }, [source.content, toast]);

  const handleOpenFile = useCallback(() => {
    // TODO: 实现文件打开功能
    toast({
      title: "打开文件",
      description: "文件查看功能即将推出",
    });
  }, [toast]);

  const language = detectLanguage(source.file_path);
  const similarity = Math.round(source.similarity_score * 100);

  return (
    <Card className="border-l-4 border-l-primary/30">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 min-w-0 flex-1">
            <FileText size={14} className="text-muted-foreground shrink-0" />
            <CardTitle className="text-sm font-medium truncate">
              {formatFilePath(source.file_path, 40)}
            </CardTitle>
            <Badge variant="secondary" className="text-xs shrink-0">
              {similarity}% 匹配
            </Badge>
          </div>
          
          <div className="flex items-center gap-1 shrink-0">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopyContent}
              className="h-6 w-6 p-0"
            >
              <Copy size={12} />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleOpenFile}
              className="h-6 w-6 p-0"
            >
              <ExternalLink size={12} />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleToggleExpand}
              className="h-6 w-6 p-0"
            >
              {isExpanded ? (
                <ChevronDown size={12} />
              ) : (
                <ChevronRight size={12} />
              )}
            </Button>
          </div>
        </div>
      </CardHeader>

      <AnimatePresence>
        {isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="overflow-hidden"
          >
            <CardContent className="pt-0">
              <ScrollArea className="max-h-60">
                <div className="relative">
                  <SyntaxHighlighter
                    style={theme === 'dark' ? oneDark : oneLight}
                    language={language}
                    PreTag="div"
                    className="!mt-0 !mb-0 text-xs rounded-md"
                    showLineNumbers={false}
                    wrapLines={true}
                    wrapLongLines={true}
                  >
                    {source.content}
                  </SyntaxHighlighter>
                </div>
              </ScrollArea>
            </CardContent>
          </motion.div>
        )}
      </AnimatePresence>
    </Card>
  );
});

SourceDocumentItem.displayName = 'SourceDocumentItem';

const SourceDocuments = memo(({ 
  sources, 
  className,
  maxHeight = 400 
}: SourceDocumentsProps) => {
  const [showAll, setShowAll] = useState(false);
  
  const displayedSources = showAll ? sources : sources.slice(0, 3);
  const hasMore = sources.length > 3;

  const handleToggleShowAll = useCallback(() => {
    setShowAll(prev => !prev);
  }, []);

  if (sources.length === 0) {
    return null;
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -10 }}
      transition={{ duration: 0.2 }}
      className={cn("space-y-2", className)}
    >
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <Hash size={12} />
        <span>相关源文档 ({sources.length})</span>
      </div>

      <ScrollArea 
        className="space-y-2" 
        style={{ maxHeight: `${maxHeight}px` }}
      >
        <div className="space-y-2">
          <AnimatePresence>
            {displayedSources.map((source, index) => (
              <motion.div
                key={`${source.file_path}-${index}`}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: 20 }}
                transition={{ 
                  duration: 0.2, 
                  delay: index * 0.05 
                }}
              >
                <SourceDocumentItem 
                  source={source} 
                  index={index} 
                />
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      </ScrollArea>

      {hasMore && (
        <Button
          variant="ghost"
          size="sm"
          onClick={handleToggleShowAll}
          className="w-full h-8 text-xs text-muted-foreground hover:text-foreground"
        >
          {showAll ? (
            <>
              <ChevronDown size={12} className="mr-1" />
              显示更少
            </>
          ) : (
            <>
              <ChevronRight size={12} className="mr-1" />
              显示全部 {sources.length} 个文档
            </>
          )}
        </Button>
      )}
    </motion.div>
  );
});

SourceDocuments.displayName = 'SourceDocuments';

export { SourceDocuments };
