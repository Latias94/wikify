/**
 * Wiki搜索组件
 * 提供全文搜索功能和结果展示
 */

import { useState, useCallback, useEffect, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import {
  Search,
  X,
  FileText,
  Hash,
  BookOpen,
  Clock,
  Star,
  Filter,
  ArrowRight
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { wikiSearchEngine } from '@/lib/search-engine';
import {
  SearchResults,
  SearchResultItem,
  SearchQuery,
  SearchResultType
} from '@/types/search';
import { WikiStructure } from '@/types/api';

interface WikiSearchProps {
  wiki: WikiStructure;
  onResultSelect?: (result: SearchResultItem) => void;
  onClose?: () => void;
  className?: string;
}

const WikiSearch = ({
  wiki,
  onResultSelect,
  onClose,
  className
}: WikiSearchProps) => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResults | null>(null);
  const [isSearching, setIsSearching] = useState(false);
  const [selectedFilters, setSelectedFilters] = useState<SearchResultType[]>([]);
  const [showFilters, setShowFilters] = useState(false);

  // 初始化搜索索引
  useEffect(() => {
    if (wiki) {
      wikiSearchEngine.buildIndex(wiki);
    }
  }, [wiki]);

  // 执行搜索
  const performSearch = useCallback(async (searchQuery: string) => {
    if (!searchQuery.trim()) {
      setResults(null);
      return;
    }

    setIsSearching(true);

    // 模拟异步搜索（实际上是同步的，但为了用户体验）
    await new Promise(resolve => setTimeout(resolve, 100));

    const searchParams: SearchQuery = {
      query: searchQuery,
      type: selectedFilters.length > 0 ? selectedFilters : undefined
    };

    const searchResults = wikiSearchEngine.search(searchParams);
    setResults(searchResults);
    setIsSearching(false);
  }, [selectedFilters]);

  // 防抖搜索
  useEffect(() => {
    const timeoutId = setTimeout(() => {
      performSearch(query);
    }, 300);

    return () => clearTimeout(timeoutId);
  }, [query, performSearch]);

  // 处理结果选择
  const handleResultSelect = useCallback((result: SearchResultItem) => {
    onResultSelect?.(result);
    onClose?.();
  }, [onResultSelect, onClose]);

  // 处理过滤器切换
  const handleFilterToggle = useCallback((type: SearchResultType) => {
    setSelectedFilters(prev => 
      prev.includes(type)
        ? prev.filter(t => t !== type)
        : [...prev, type]
    );
  }, []);

  // 清空搜索
  const handleClear = useCallback(() => {
    setQuery('');
    setResults(null);
  }, []);

  // 获取类型图标
  const getTypeIcon = (type: SearchResultType) => {
    switch (type) {
      case 'page':
        return <FileText className="h-4 w-4" />;
      case 'section':
        return <Hash className="h-4 w-4" />;
      default:
        return <BookOpen className="h-4 w-4" />;
    }
  };

  // 获取重要性颜色
  const getImportanceColor = (importance?: string) => {
    switch (importance) {
      case 'High':
        return 'destructive';
      case 'Medium':
        return 'default';
      case 'Low':
        return 'secondary';
      default:
        return 'outline';
    }
  };

  // 高亮搜索结果
  const highlightText = (text: string, highlights: any[]) => {
    if (!highlights.length) return text;

    // 简单的高亮实现
    let highlightedText = text;
    highlights.forEach(highlight => {
      if (highlight.field === 'title' || highlight.field === 'content') {
        highlight.indices.forEach(([start, end]: [number, number]) => {
          const before = text.slice(0, start);
          const match = text.slice(start, end + 1);
          const after = text.slice(end + 1);
          highlightedText = before + `<mark class="bg-yellow-200 dark:bg-yellow-800">${match}</mark>` + after;
        });
      }
    });

    return highlightedText;
  };

  const filterOptions: { type: SearchResultType; label: string; icon: React.ReactNode }[] = [
    { type: 'page', label: 'Pages', icon: <FileText className="h-4 w-4" /> },
    { type: 'section', label: 'Sections', icon: <Hash className="h-4 w-4" /> },
    { type: 'content', label: 'Content', icon: <BookOpen className="h-4 w-4" /> }
  ];

  return (
    <div className={cn("w-full max-w-2xl mx-auto", className)}>
      {/* 搜索输入框 */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search wiki content..."
          className="pl-10 pr-20"
          autoFocus
        />
        <div className="absolute right-2 top-1/2 transform -translate-y-1/2 flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setShowFilters(!showFilters)}
            className={cn(
              "h-6 w-6 p-0",
              selectedFilters.length > 0 && "text-primary"
            )}
          >
            <Filter className="h-3 w-3" />
          </Button>
          {query && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClear}
              className="h-6 w-6 p-0"
            >
              <X className="h-3 w-3" />
            </Button>
          )}
        </div>
      </div>

      {/* 过滤器 */}
      <AnimatePresence>
        {showFilters && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            className="mt-2 p-3 border rounded-lg bg-muted/50"
          >
            <div className="flex items-center gap-2 flex-wrap">
              <span className="text-sm font-medium text-muted-foreground">Filter by:</span>
              {filterOptions.map(option => (
                <Button
                  key={option.type}
                  variant={selectedFilters.includes(option.type) ? "default" : "outline"}
                  size="sm"
                  onClick={() => handleFilterToggle(option.type)}
                  className="h-7 text-xs"
                >
                  {option.icon}
                  <span className="ml-1">{option.label}</span>
                </Button>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* 搜索结果 */}
      <AnimatePresence>
        {(results || isSearching) && (
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 10 }}
            className="mt-4"
          >
            <Card>
              <CardContent className="p-0">
                {/* 结果头部 */}
                <div className="p-4 border-b">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      {isSearching ? (
                        <>
                          <div className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
                          <span className="text-sm text-muted-foreground">Searching...</span>
                        </>
                      ) : (
                        <>
                          <Search className="h-4 w-4 text-muted-foreground" />
                          <span className="text-sm text-muted-foreground">
                            {results?.total || 0} results for "{query}"
                          </span>
                        </>
                      )}
                    </div>
                    {results && (
                      <span className="text-xs text-muted-foreground">
                        {results.executionTime.toFixed(1)}ms
                      </span>
                    )}
                  </div>
                </div>

                {/* 结果列表 */}
                <ScrollArea className="max-h-96">
                  {results && results.items.length > 0 ? (
                    <div className="divide-y">
                      {results.items.map((result, index) => (
                        <motion.div
                          key={result.id}
                          initial={{ opacity: 0, x: -10 }}
                          animate={{ opacity: 1, x: 0 }}
                          transition={{ delay: index * 0.05 }}
                          className="p-4 hover:bg-accent cursor-pointer transition-colors"
                          onClick={() => handleResultSelect(result)}
                        >
                          <div className="space-y-2">
                            {/* 结果头部 */}
                            <div className="flex items-start justify-between gap-2">
                              <div className="flex items-center gap-2 min-w-0 flex-1">
                                {getTypeIcon(result.type)}
                                <h4 
                                  className="font-medium text-sm truncate"
                                  dangerouslySetInnerHTML={{
                                    __html: highlightText(result.title, result.highlights)
                                  }}
                                />
                              </div>
                              <div className="flex items-center gap-1 shrink-0">
                                {result.metadata.importance && (
                                  <Badge 
                                    variant={getImportanceColor(result.metadata.importance)}
                                    className="text-xs h-5"
                                  >
                                    <Star className="h-2 w-2 mr-1" />
                                    {result.metadata.importance}
                                  </Badge>
                                )}
                                <Badge variant="outline" className="text-xs h-5">
                                  {Math.round(result.score * 100)}%
                                </Badge>
                              </div>
                            </div>

                            {/* 面包屑 */}
                            <div className="flex items-center gap-1 text-xs text-muted-foreground">
                              {result.metadata.breadcrumb.map((crumb, i) => (
                                <span key={i} className="flex items-center gap-1">
                                  {i > 0 && <ArrowRight className="h-3 w-3" />}
                                  <span className="truncate max-w-20">{crumb}</span>
                                </span>
                              ))}
                            </div>

                            {/* 摘录 */}
                            <p 
                              className="text-sm text-muted-foreground line-clamp-2"
                              dangerouslySetInnerHTML={{
                                __html: highlightText(result.excerpt, result.highlights)
                              }}
                            />
                          </div>
                        </motion.div>
                      ))}
                    </div>
                  ) : results && !isSearching ? (
                    <div className="p-8 text-center text-muted-foreground">
                      <Search className="h-8 w-8 mx-auto mb-2 opacity-50" />
                      <p>No results found for "{query}"</p>
                      {results.suggestions && results.suggestions.length > 0 && (
                        <div className="mt-4">
                          <p className="text-sm mb-2">Did you mean:</p>
                          <div className="flex flex-wrap gap-1 justify-center">
                            {results.suggestions.map(suggestion => (
                              <Button
                                key={suggestion}
                                variant="outline"
                                size="sm"
                                onClick={() => setQuery(suggestion)}
                                className="h-6 text-xs"
                              >
                                {suggestion}
                              </Button>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                  ) : null}
                </ScrollArea>
              </CardContent>
            </Card>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export { WikiSearch };
