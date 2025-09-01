/**
 * Wiki查看器组件
 * 基于后端返回的WikiStructure数据完全渲染
 */

import { useState, useCallback, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useToast } from "@/hooks/use-toast";
import { 
  ArrowLeft, 
  BookOpen,
  FileText,
  Download,
  Search,
  ChevronRight,
  ChevronDown,
  ExternalLink,
  Hash,
  Star,
  Clock
} from "lucide-react";
import { useParams, useNavigate } from "react-router-dom";

// Components
import { StreamingContent } from "@/components/chat/StreamingContent";
import { WikiSearch } from "@/components/WikiSearch";
import { WikiExportDialog } from "@/components/WikiExportDialog";

// Hooks and API
import { useWiki } from "@/hooks/use-api";

// Types
import { WikiStructure, WikiPage, WikiSection } from "@/types/api";
import { SearchResultItem } from "@/types/search";
import { cn } from "@/lib/utils";

interface WikiViewerProps {
  className?: string;
}

const WikiViewer = ({ className }: WikiViewerProps) => {
  const { repositoryId } = useParams<{ repositoryId: string }>();
  const navigate = useNavigate();
  const { toast } = useToast();

  // State
  const [selectedPageId, setSelectedPageId] = useState<string | null>(null);
  const [selectedSectionId, setSelectedSectionId] = useState<string | null>(null);
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set());
  const [showSearch, setShowSearch] = useState(false);
  const [showExportDialog, setShowExportDialog] = useState(false);

  // API
  const { data: wiki, isLoading, error } = useWiki(repositoryId || '');

  // Effects
  useEffect(() => {
    if (wiki && !selectedPageId && wiki.pages.length > 0) {
      // 默认选择第一个页面
      setSelectedPageId(wiki.pages[0].id);
    }
  }, [wiki, selectedPageId]);

  useEffect(() => {
    if (!repositoryId) {
      toast({
        title: "Invalid Repository",
        description: "No repository ID provided",
        variant: "destructive"
      });
      navigate('/');
    }
  }, [repositoryId, navigate, toast]);

  // Handlers
  const handleGoBack = useCallback(() => {
    navigate('/');
  }, [navigate]);

  const handlePageSelect = useCallback((pageId: string) => {
    setSelectedPageId(pageId);
    setSelectedSectionId(null);
  }, []);

  const handleSectionSelect = useCallback((sectionId: string) => {
    setSelectedSectionId(sectionId);
  }, []);

  const toggleSectionExpanded = useCallback((sectionId: string) => {
    setExpandedSections(prev => {
      const newSet = new Set(prev);
      if (newSet.has(sectionId)) {
        newSet.delete(sectionId);
      } else {
        newSet.add(sectionId);
      }
      return newSet;
    });
  }, []);

  const handleExport = useCallback(() => {
    setShowExportDialog(true);
  }, []);

  const handleSearchResultSelect = useCallback((result: SearchResultItem) => {
    // 根据搜索结果类型进行导航
    if (result.type === 'page' && result.metadata.pageId) {
      setSelectedPageId(result.metadata.pageId);
      setSelectedSectionId(null);
    } else if (result.type === 'section' && result.metadata.sectionId) {
      setSelectedSectionId(result.metadata.sectionId);
      if (result.metadata.pageId) {
        setSelectedPageId(result.metadata.pageId);
      }
    }

    // 关闭搜索
    setShowSearch(false);

    toast({
      title: "Navigated to result",
      description: `Jumped to: ${result.title}`,
    });
  }, [toast]);

  // Get current content
  const getCurrentContent = useCallback(() => {
    if (!wiki) return null;

    if (selectedSectionId) {
      // 查找选中的section
      const findSection = (sections: WikiSection[]): WikiSection | null => {
        for (const section of sections) {
          if (section.id === selectedSectionId) return section;
          const found = findSection(section.subsections);
          if (found) return found;
        }
        return null;
      };

      // 在根级sections中查找
      const section = findSection(wiki.sections);
      if (section) return section;
    }

    if (selectedPageId) {
      return wiki.pages.find(page => page.id === selectedPageId);
    }

    // 默认返回wiki概览
    return {
      id: 'overview',
      title: wiki.title,
      content: wiki.description,
    };
  }, [wiki, selectedPageId, selectedSectionId]);

  // Loading state
  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <BookOpen className="h-8 w-8 animate-spin mx-auto mb-4" />
          <h2 className="text-lg font-semibold mb-2">Loading Wiki...</h2>
          <p className="text-muted-foreground">Please wait while we load the documentation</p>
        </div>
      </div>
    );
  }

  // Error state
  if (error || !wiki) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <FileText className="h-8 w-8 text-destructive mx-auto mb-4" />
          <h2 className="text-lg font-semibold mb-2">Failed to Load Wiki</h2>
          <p className="text-muted-foreground mb-4">
            {error?.message || "Unable to load wiki documentation"}
          </p>
          <Button onClick={handleGoBack}>
            Return to Home
          </Button>
        </div>
      </div>
    );
  }

  const currentContent = getCurrentContent();

  return (
    <motion.div 
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className={cn("min-h-screen flex flex-col bg-background", className)}
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
            Back
          </Button>
          
          <Separator orientation="vertical" className="h-6" />
          
          <div className="flex items-center gap-2">
            <BookOpen className="h-5 w-5 text-primary" />
            <h1 className="font-semibold">{wiki.title}</h1>
          </div>
          
          <div className="flex-1" />

          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowSearch(!showSearch)}
              className="flex items-center gap-2"
            >
              <Search className="h-4 w-4" />
              Search
            </Button>

            <Button
              variant="outline"
              size="sm"
              onClick={handleExport}
              className="flex items-center gap-2"
            >
              <Download className="h-4 w-4" />
              Export
            </Button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar - Table of Contents */}
        <aside className="w-80 border-r bg-muted/30">
          <ScrollArea className="h-full">
            <div className="p-4 space-y-4">
              {/* Wiki Overview */}
              <Card 
                className={cn(
                  "cursor-pointer transition-colors hover:bg-accent",
                  !selectedPageId && !selectedSectionId && "ring-2 ring-primary"
                )}
                onClick={() => {
                  setSelectedPageId(null);
                  setSelectedSectionId(null);
                }}
              >
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm flex items-center gap-2">
                    <BookOpen className="h-4 w-4" />
                    Overview
                  </CardTitle>
                </CardHeader>
                <CardContent className="pt-0">
                  <CardDescription className="text-xs line-clamp-2">
                    {wiki.description}
                  </CardDescription>
                </CardContent>
              </Card>

              {/* Pages */}
              {wiki.pages.length > 0 && (
                <div className="space-y-2">
                  <h3 className="text-sm font-medium text-muted-foreground px-2">Pages</h3>
                  <div className="space-y-1">
                    {wiki.pages.map((page) => (
                      <PageItem
                        key={page.id}
                        page={page}
                        isSelected={selectedPageId === page.id}
                        selectedSectionId={selectedSectionId}
                        expandedSections={expandedSections}
                        onPageSelect={handlePageSelect}
                        onSectionSelect={handleSectionSelect}
                        onToggleExpanded={toggleSectionExpanded}
                      />
                    ))}
                  </div>
                </div>
              )}

              {/* Root Sections */}
              {wiki.sections.length > 0 && (
                <div className="space-y-2">
                  <h3 className="text-sm font-medium text-muted-foreground px-2">Sections</h3>
                  <div className="space-y-1">
                    {wiki.sections.map((section) => (
                      <SectionItem
                        key={section.id}
                        section={section}
                        wiki={wiki}
                        isSelected={selectedSectionId === section.id}
                        expandedSections={expandedSections}
                        onSectionSelect={handleSectionSelect}
                        onToggleExpanded={toggleSectionExpanded}
                        level={0}
                      />
                    ))}
                  </div>
                </div>
              )}
            </div>
          </ScrollArea>
        </aside>

        {/* Main Content Area */}
        <main className="flex-1 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="container py-6 max-w-4xl">
              {/* Search Component */}
              <AnimatePresence>
                {showSearch && wiki && (
                  <motion.div
                    initial={{ opacity: 0, y: -20 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -20 }}
                    className="mb-6"
                  >
                    <WikiSearch
                      wiki={wiki}
                      onResultSelect={handleSearchResultSelect}
                      onClose={() => setShowSearch(false)}
                    />
                  </motion.div>
                )}
              </AnimatePresence>
              {currentContent && (
                <motion.div
                  key={currentContent.id}
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.3 }}
                  className="space-y-6"
                >
                  {/* Content Header */}
                  <div className="space-y-2">
                    <h1 className="text-3xl font-bold tracking-tight">
                      {currentContent.title}
                    </h1>
                    
                    {/* Metadata for pages */}
                    {'importance' in currentContent && (
                      <div className="flex items-center gap-2">
                        <Badge 
                          variant={
                            currentContent.importance === 'High' ? 'destructive' :
                            currentContent.importance === 'Medium' ? 'default' : 'secondary'
                          }
                          className="text-xs"
                        >
                          <Star className="h-3 w-3 mr-1" />
                          {currentContent.importance}
                        </Badge>
                        
                        {currentContent.file_paths && currentContent.file_paths.length > 0 && (
                          <Badge variant="outline" className="text-xs">
                            <FileText className="h-3 w-3 mr-1" />
                            {currentContent.file_paths.length} files
                          </Badge>
                        )}
                      </div>
                    )}
                  </div>

                  {/* Content Body */}
                  <div className="prose prose-gray dark:prose-invert max-w-none">
                    <StreamingContent
                      content={currentContent.content}
                      className="max-w-none"
                    />
                  </div>

                  {/* Related Pages */}
                  {'related_pages' in currentContent && currentContent.related_pages && currentContent.related_pages.length > 0 && (
                    <div className="mt-8 p-4 bg-muted/50 rounded-lg">
                      <h3 className="text-sm font-medium mb-2 flex items-center gap-2">
                        <ExternalLink className="h-4 w-4" />
                        Related Pages
                      </h3>
                      <div className="flex flex-wrap gap-2">
                        {currentContent.related_pages.map((pageId) => {
                          const relatedPage = wiki.pages.find(p => p.id === pageId);
                          return relatedPage ? (
                            <Button
                              key={pageId}
                              variant="outline"
                              size="sm"
                              onClick={() => handlePageSelect(pageId)}
                              className="text-xs"
                            >
                              {relatedPage.title}
                            </Button>
                          ) : null;
                        })}
                      </div>
                    </div>
                  )}
                </motion.div>
              )}
            </div>
          </ScrollArea>
        </main>
      </div>

      {/* Export Dialog */}
      {wiki && (
        <WikiExportDialog
          open={showExportDialog}
          onOpenChange={setShowExportDialog}
          wiki={wiki}
        />
      )}
    </motion.div>
  );
};

// Helper Components
interface PageItemProps {
  page: WikiPage;
  isSelected: boolean;
  selectedSectionId: string | null;
  expandedSections: Set<string>;
  onPageSelect: (pageId: string) => void;
  onSectionSelect: (sectionId: string) => void;
  onToggleExpanded: (sectionId: string) => void;
}

const PageItem = ({
  page,
  isSelected,
  selectedSectionId,
  expandedSections,
  onPageSelect,
  onSectionSelect,
  onToggleExpanded
}: PageItemProps) => {
  // Pages don't have subsections in the new structure
  const hasSubsections = false;
  const isExpanded = expandedSections.has(page.id);

  return (
    <div className="space-y-1">
      <div
        className={cn(
          "flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer transition-colors text-sm",
          "hover:bg-accent hover:text-accent-foreground",
          isSelected && "bg-primary text-primary-foreground"
        )}
        onClick={() => onPageSelect(page.id)}
      >
        {hasSubsections && (
          <Button
            variant="ghost"
            size="sm"
            className="h-4 w-4 p-0"
            onClick={(e) => {
              e.stopPropagation();
              onToggleExpanded(page.id);
            }}
          >
            {isExpanded ? (
              <ChevronDown className="h-3 w-3" />
            ) : (
              <ChevronRight className="h-3 w-3" />
            )}
          </Button>
        )}

        <FileText className="h-4 w-4 shrink-0" />

        <span className="truncate flex-1">{page.title}</span>

        <Badge
          variant={
            page.importance === 'High' ? 'destructive' :
            page.importance === 'Medium' ? 'default' : 'secondary'
          }
          className="text-xs h-4"
        >
          {page.importance.charAt(0)}
        </Badge>
      </div>

      {/* Page Sections */}
      {/* Pages don't have subsections in the new structure */}
    </div>
  );
};

interface SectionItemProps {
  section: WikiSection;
  wiki: WikiStructure;
  isSelected: boolean;
  expandedSections: Set<string>;
  onSectionSelect: (sectionId: string) => void;
  onToggleExpanded: (sectionId: string) => void;
  level: number;
}

const SectionItem = ({
  section,
  wiki,
  isSelected,
  expandedSections,
  onSectionSelect,
  onToggleExpanded,
  level
}: SectionItemProps) => {
  // Find actual subsection objects from IDs
  const subsections = section.subsections
    .map(id => wiki.sections.find(s => s.id === id))
    .filter(Boolean) as WikiSection[];

  const hasSubsections = subsections.length > 0;
  const isExpanded = expandedSections.has(section.id);
  const indent = level * 16; // 16px per level

  return (
    <div className="space-y-1">
      <div
        className={cn(
          "flex items-center gap-2 px-2 py-1 rounded-md cursor-pointer transition-colors text-sm",
          "hover:bg-accent hover:text-accent-foreground",
          isSelected && "bg-primary text-primary-foreground"
        )}
        style={{ marginLeft: `${indent}px` }}
        onClick={() => onSectionSelect(section.id)}
      >
        {hasSubsections && (
          <Button
            variant="ghost"
            size="sm"
            className="h-4 w-4 p-0"
            onClick={(e) => {
              e.stopPropagation();
              onToggleExpanded(section.id);
            }}
          >
            {isExpanded ? (
              <ChevronDown className="h-3 w-3" />
            ) : (
              <ChevronRight className="h-3 w-3" />
            )}
          </Button>
        )}

        <Hash className="h-3 w-3 shrink-0" />
        <span className="truncate flex-1">{section.title}</span>
      </div>

      {/* Subsections */}
      <AnimatePresence>
        {hasSubsections && isExpanded && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.2 }}
            className="space-y-1"
          >
            {subsections.map((subsection) => (
              <SectionItem
                key={subsection.id}
                section={subsection}
                wiki={wiki}
                isSelected={selectedSectionId === subsection.id}
                expandedSections={expandedSections}
                onSectionSelect={onSectionSelect}
                onToggleExpanded={onToggleExpanded}
                level={level + 1}
              />
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export { WikiViewer };
