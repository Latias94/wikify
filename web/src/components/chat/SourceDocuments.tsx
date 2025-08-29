/**
 * 源文档组件
 * 显示 AI 回答的相关源文档
 */

import { memo, useState, useCallback, useEffect } from 'react';
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
  Hash,
  MapPin,
  Code2,
  Eye,
  EyeOff,
  Search,
  FolderOpen
} from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { SourceDocument } from '@/types/api';
import { cn } from '@/lib/utils';
import {
  formatFilePath,
  detectLanguage,
  estimateLineRange,
  formatLineRange,
  generateIDELink,
  canOpenInBrowser,
  formatCopyablePath,
  generateFileInfo,
  generateGitBrowserLink,
  isRemoteRepository
} from '@/utils/formatters';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark, oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { useTheme } from 'next-themes';
import { useSelectedRepository } from '@/store/app-store';

interface SourceDocumentsProps {
  sources: SourceDocument[];
  className?: string;
  maxHeight?: number; // 可选，不提供时会自动计算
}

interface SourceDocumentItemProps {
  source: SourceDocument;
  index: number;
}

const SourceDocumentItem = memo(({ source, index }: SourceDocumentItemProps) => {
  const { theme } = useTheme();
  const { toast } = useToast();
  const [isExpanded, setIsExpanded] = useState(false);
  const selectedRepository = useSelectedRepository();

  const handleToggleExpand = useCallback(() => {
    setIsExpanded(prev => !prev);
  }, []);

  // 计算行号范围
  const lineRange = estimateLineRange(source.content, source.chunk_index);
  const actualStartLine = source.start_line || lineRange.start;
  const actualEndLine = source.end_line || lineRange.end;

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

  const handleCopyPath = useCallback(async () => {
    try {
      // 提供相对路径，开发者可以在自己的项目中搜索
      const relativePath = source.file_path.replace(/^.*\/(?=\w)/, '');
      await navigator.clipboard.writeText(relativePath);
      toast({
        title: "已复制相对路径",
        description: `${relativePath} - 可在项目中搜索此文件`,
      });
    } catch (error) {
      toast({
        title: "复制失败",
        description: "无法复制文件路径",
        variant: "destructive"
      });
    }
  }, [source.file_path, toast]);

  const handleCopyFileInfo = useCallback(async () => {
    try {
      const fileName = source.file_path.split('/').pop() || source.file_path;
      const lineInfo = formatLineRange(actualStartLine, actualEndLine);
      const searchInfo = `文件: ${fileName}${lineInfo ? ` (${lineInfo})` : ''}`;

      await navigator.clipboard.writeText(searchInfo);
      toast({
        title: "已复制搜索信息",
        description: `${searchInfo} - 可在 IDE 中搜索文件名`,
      });
    } catch (error) {
      toast({
        title: "复制失败",
        description: "无法复制文件信息",
        variant: "destructive"
      });
    }
  }, [source.file_path, actualStartLine, actualEndLine, toast]);

  const handleCopySearchCommand = useCallback(async () => {
    try {
      // 生成可以在项目中搜索的命令
      const fileName = source.file_path.split('/').pop() || source.file_path;
      const searchCommand = `# 在项目中搜索此文件:\nfind . -name "${fileName}" -type f`;

      await navigator.clipboard.writeText(searchCommand);
      toast({
        title: "已复制搜索命令",
        description: "可在项目根目录执行此命令查找文件",
      });
    } catch (error) {
      toast({
        title: "复制失败",
        description: "无法复制搜索命令",
        variant: "destructive"
      });
    }
  }, [source.file_path, toast]);

  const handleOpenFile = useCallback(() => {
    const ideLinks = generateIDELink(source.file_path, actualStartLine);

    // 尝试打开 VS Code（最常用）
    const link = document.createElement('a');
    link.href = ideLinks.vscode;
    link.click();

    toast({
      title: "尝试打开文件",
      description: `正在尝试在 VS Code 中打开 ${source.file_path}`,
    });
  }, [source.file_path, actualStartLine, toast]);

  const handleOpenInBrowser = useCallback(() => {
    // 检查是否为远程 Git 仓库
    const repositoryUrl = selectedRepository?.repo_path;

    if (isRemoteRepository(repositoryUrl)) {
      const gitLink = generateGitBrowserLink(source.file_path, actualStartLine, repositoryUrl);

      if (gitLink) {
        window.open(gitLink.url, '_blank');
        toast({
          title: "已在浏览器中打开",
          description: `在 ${gitLink.platform} 中查看文件`,
        });
      } else {
        toast({
          title: "无法打开",
          description: "不支持的 Git 平台",
          variant: "destructive"
        });
      }
    } else {
      toast({
        title: "无法打开",
        description: "仅支持远程 Git 仓库（GitHub、GitLab 等）",
        variant: "destructive"
      });
    }
  }, [source.file_path, actualStartLine, selectedRepository?.repo_path, toast]);

  const language = detectLanguage(source.file_path);
  const similarity = Math.round(source.similarity_score * 100);

  return (
    <Card className="border-l-4 border-l-primary/30 w-full overflow-hidden">
      <CardHeader className="pb-2">
        {/* 文件路径和操作按钮 */}
        <div className="flex items-center justify-between min-w-0 w-full">
          <div className="flex items-center gap-2 min-w-0 flex-1 overflow-hidden">
            <FileText size={14} className="text-muted-foreground shrink-0" />
            <div className="flex flex-col min-w-0 flex-1">
              <CardTitle
                className="text-sm font-medium truncate min-w-0 cursor-pointer hover:text-primary transition-colors"
                onClick={handleCopyPath}
                title={`点击复制路径: ${source.file_path}`}
              >
                {formatFilePath(source.file_path, 30)}
              </CardTitle>
              {/* 行号信息 */}
              {(actualStartLine || actualEndLine) && (
                <div className="flex items-center gap-2 text-xs text-muted-foreground mt-1">
                  <MapPin size={10} />
                  <span>{formatLineRange(actualStartLine, actualEndLine)}</span>
                  {source.chunk_index !== undefined && (
                    <Badge variant="outline" className="text-xs px-1 py-0 h-4">
                      块 {source.chunk_index + 1}
                    </Badge>
                  )}
                </div>
              )}
            </div>
            <Badge variant="secondary" className="text-xs shrink-0 ml-2">
              {similarity}%
            </Badge>
          </div>
          
          <div className="flex items-center gap-1 shrink-0">
            {/* 复制代码内容 */}
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopyContent}
              className="h-6 w-6 p-0"
              title="复制代码内容"
            >
              <Copy size={12} />
            </Button>

            {/* 复制搜索信息 */}
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopyFileInfo}
              className="h-6 w-6 p-0"
              title="复制文件搜索信息"
            >
              <Search size={12} />
            </Button>

            {/* 复制搜索命令 */}
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopySearchCommand}
              className="h-6 w-6 p-0"
              title="复制文件查找命令"
            >
              <FolderOpen size={12} />
            </Button>

            {/* 暂时隐藏 IDE 打开功能 - 需要解决路径映射问题 */}
            {/* <Button
              variant="ghost"
              size="sm"
              onClick={handleOpenFile}
              className="h-6 w-6 p-0"
              title="在 VS Code 中打开"
            >
              <ExternalLink size={12} />
            </Button> */}

            {/* Git 平台浏览器查看（仅远程仓库） */}
            {isRemoteRepository(selectedRepository?.repo_path) && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleOpenInBrowser}
                className="h-6 w-6 p-0"
                title="在 Git 平台中查看文件"
              >
                <ExternalLink size={12} />
              </Button>
            )}

            {/* 展开/折叠 */}
            <Button
              variant="ghost"
              size="sm"
              onClick={handleToggleExpand}
              className="h-6 w-6 p-0"
              title={isExpanded ? "折叠代码" : "展开代码"}
            >
              {isExpanded ? (
                <EyeOff size={12} />
              ) : (
                <Eye size={12} />
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
            className="overflow-hidden w-full max-w-full"
          >
            <CardContent className="pt-0 px-4">
              {/* 代码信息栏 */}
              <div className="flex items-center justify-between text-xs text-muted-foreground mb-2 px-2">
                <div className="flex items-center gap-2">
                  <Badge variant="outline" className="text-xs">
                    {language}
                  </Badge>
                  <span>{source.content.split('\n').length} 行</span>
                  {actualStartLine && (
                    <span>从第 {actualStartLine} 行开始</span>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleCopyContent}
                    className="h-5 px-2 text-xs"
                    title="复制代码内容"
                  >
                    <Copy size={10} className="mr-1" />
                    复制代码
                  </Button>
                </div>
              </div>

              {/* 代码展示区域 */}
              <div className="max-h-80 w-full overflow-auto border rounded-md bg-muted/30 relative">
                <SyntaxHighlighter
                  style={theme === 'dark' ? oneDark : oneLight}
                  language={language}
                  PreTag="div"
                  className="!mt-0 !mb-0 text-xs"
                  showLineNumbers={true}
                  startingLineNumber={actualStartLine || 1}
                  wrapLines={false}
                  wrapLongLines={false}
                  customStyle={{
                    margin: 0,
                    padding: '0.75rem',
                    background: 'transparent',
                    fontSize: '0.75rem',
                    lineHeight: '1.4rem'
                  }}
                >
                  {source.content}
                </SyntaxHighlighter>

                {/* 滚动提示 */}
                <div className="absolute top-2 right-2 opacity-50 text-xs text-muted-foreground pointer-events-none">
                  {source.content.split('\n').length > 20 && "↕ 滚动查看"}
                </div>
              </div>
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
  maxHeight
}: SourceDocumentsProps) => {
  const [showAll, setShowAll] = useState(false);
  const [canScroll, setCanScroll] = useState(false);
  const selectedRepository = useSelectedRepository();

  // 智能计算最大高度
  const calculateMaxHeight = () => {
    if (maxHeight) return maxHeight;

    // 基于视口高度动态计算
    const viewportHeight = window.innerHeight;
    const isMobile = window.innerWidth < 768;

    if (isMobile) {
      // 移动设备：最多占用 40% 视口高度
      return Math.min(300, viewportHeight * 0.4);
    } else {
      // 桌面设备：最多占用 50% 视口高度，但不超过 500px
      return Math.min(500, viewportHeight * 0.5);
    }
  };

  const dynamicMaxHeight = calculateMaxHeight();
  const displayedSources = showAll ? sources : sources.slice(0, 3);
  const hasMore = sources.length > 3;

  const handleToggleShowAll = useCallback(() => {
    setShowAll(prev => !prev);
  }, []);

  // 检查是否可以滚动
  const handleScrollCheck = useCallback((element: HTMLDivElement | null) => {
    if (element) {
      const hasScroll = element.scrollHeight > element.clientHeight;
      setCanScroll(hasScroll);
    }
  }, []);

  // 当显示的源文档数量变化时重新检查滚动
  useEffect(() => {
    // 延迟检查，确保 DOM 已更新
    const timer = setTimeout(() => {
      const scrollContainer = document.querySelector('[data-scroll-container="sources"]') as HTMLDivElement;
      handleScrollCheck(scrollContainer);
    }, 100);

    return () => clearTimeout(timer);
  }, [displayedSources.length, handleScrollCheck]);

  if (sources.length === 0) {
    return null;
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -10 }}
      transition={{ duration: 0.2 }}
      className={cn("space-y-2 w-full max-w-full overflow-hidden", className)}
    >
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <div className="flex items-center gap-2">
          <Hash size={12} />
          <span>相关源文档 ({sources.length})</span>
          {showAll && sources.length > 3 && (
            <Badge variant="outline" className="text-xs px-1 py-0 h-4">
              显示全部
            </Badge>
          )}
        </div>

        <div className="flex items-center gap-2">
          {/* 文件类型统计 */}
          {sources.length > 1 && (
            <div className="flex items-center gap-1">
              {Array.from(new Set(sources.map(s => detectLanguage(s.file_path)))).slice(0, 3).map(lang => (
                <Badge key={lang} variant="secondary" className="text-xs px-1 py-0 h-4">
                  {lang}
                </Badge>
              ))}
              {Array.from(new Set(sources.map(s => detectLanguage(s.file_path)))).length > 3 && (
                <span className="text-xs">+{Array.from(new Set(sources.map(s => detectLanguage(s.file_path)))).length - 3}</span>
              )}
            </div>
          )}

          {/* 平均相似度 */}
          <span className="text-xs">
            平均匹配度: {Math.round(sources.reduce((acc, s) => acc + s.similarity_score, 0) / sources.length * 100)}%
          </span>
        </div>
      </div>

      <div className="relative">
        <div
          ref={handleScrollCheck}
          data-scroll-container="sources"
          className="space-y-2 overflow-y-auto overflow-x-hidden scrollbar-thin scrollbar-thumb-muted scrollbar-track-transparent"
          style={{ maxHeight: `${dynamicMaxHeight}px` }}
        >
          <div className="space-y-2 w-full pr-1">
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
                  className="w-full overflow-hidden"
                >
                  <SourceDocumentItem
                    source={source}
                    index={index}
                  />
                </motion.div>
              ))}
            </AnimatePresence>
          </div>
        </div>

        {/* 滚动提示 */}
        {canScroll && showAll && (
          <div className="absolute bottom-0 left-0 right-0 h-6 bg-gradient-to-t from-background/80 to-transparent pointer-events-none flex items-end justify-center pb-1">
            <div className="text-xs text-muted-foreground">↕ 可滚动查看更多</div>
          </div>
        )}
      </div>

      {hasMore && (
        <div className="space-y-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleToggleShowAll}
            className="w-full h-8 text-xs text-muted-foreground hover:text-foreground border border-dashed border-muted-foreground/30 hover:border-muted-foreground/60"
          >
            {showAll ? (
              <>
                <ChevronDown size={12} className="mr-1" />
                显示更少 (隐藏 {sources.length - 3} 个)
              </>
            ) : (
              <>
                <ChevronRight size={12} className="mr-1" />
                显示全部 {sources.length} 个文档
                <Badge variant="secondary" className="ml-2 text-xs px-1 py-0 h-4">
                  +{sources.length - 3}
                </Badge>
              </>
            )}
          </Button>

          {/* 快速操作提示 */}
          {!showAll && (
            <div className="text-xs text-muted-foreground text-center">
              💡 提示：点击文件路径复制相对路径，<Search size={10} className="inline mx-1" /> 复制搜索信息，<FolderOpen size={10} className="inline mx-1" /> 复制查找命令
              {isRemoteRepository(selectedRepository?.repo_path) && (
                <span>，<ExternalLink size={10} className="inline mx-1" /> 在 Git 平台查看</span>
              )}
            </div>
          )}
        </div>
      )}
    </motion.div>
  );
});

SourceDocuments.displayName = 'SourceDocuments';

export { SourceDocuments };
