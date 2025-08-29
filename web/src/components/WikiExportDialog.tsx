/**
 * Wiki导出对话框组件
 * 提供导出配置和进度显示
 */

import { useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import {
  Download,
  FileText,
  Folder,
  Settings,
  CheckCircle,
  Loader2,
  Archive
} from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { WikiStructure } from '@/types/api';
import { wikiExporter, ExportConfig, ExportStats } from '@/lib/wiki-exporter';
import { cn } from '@/lib/utils';

interface WikiExportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  wiki: WikiStructure;
}

type ExportState = 'idle' | 'configuring' | 'exporting' | 'completed' | 'error';

const WikiExportDialog = ({ open, onOpenChange, wiki }: WikiExportDialogProps) => {
  const { toast } = useToast();
  
  // State
  const [exportState, setExportState] = useState<ExportState>('idle');
  const [exportConfig, setExportConfig] = useState<ExportConfig>({
    includeMetadata: true,
    includeTableOfContents: true,
    fileNameFormat: 'title',
    folderStructure: 'hierarchical'
  });
  const [exportProgress, setExportProgress] = useState(0);
  const [exportStats, setExportStats] = useState<ExportStats | null>(null);

  // 开始导出
  const handleStartExport = useCallback(async () => {
    setExportState('exporting');
    setExportProgress(0);

    try {
      // 模拟进度更新
      const progressInterval = setInterval(() => {
        setExportProgress(prev => Math.min(prev + 10, 90));
      }, 200);

      // 执行导出
      const exporter = new (await import('@/lib/wiki-exporter')).WikiExporter(exportConfig);
      const { blob, stats } = await exporter.exportToZip(wiki);

      clearInterval(progressInterval);
      setExportProgress(100);

      // 触发下载
      const filename = `${wiki.title.replace(/[^a-zA-Z0-9]/g, '_')}_wiki_export.zip`;
      exporter.downloadZip(blob, filename);

      setExportStats(stats);
      setExportState('completed');

      toast({
        title: "Export Successful",
        description: `Wiki exported successfully! ${stats.totalFiles} files created.`,
      });

    } catch (error) {
      console.error('Export failed:', error);
      setExportState('error');
      toast({
        title: "Export Failed",
        description: error instanceof Error ? error.message : "Unknown error occurred",
        variant: "destructive"
      });
    }
  }, [wiki, exportConfig, toast]);

  // 重置状态
  const handleReset = useCallback(() => {
    setExportState('idle');
    setExportProgress(0);
    setExportStats(null);
  }, []);

  // 关闭对话框
  const handleClose = useCallback(() => {
    if (exportState !== 'exporting') {
      onOpenChange(false);
      // 延迟重置状态，避免动画问题
      setTimeout(handleReset, 300);
    }
  }, [exportState, onOpenChange, handleReset]);

  // 渲染配置界面
  const renderConfiguration = () => (
    <div className="space-y-6">
      {/* 基本设置 */}
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <Settings className="h-4 w-4" />
          <h3 className="font-medium">Export Settings</h3>
        </div>

        <div className="space-y-3">
          {/* 包含元数据 */}
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label htmlFor="metadata">Include Metadata</Label>
              <p className="text-xs text-muted-foreground">
                Add YAML frontmatter with page information
              </p>
            </div>
            <Switch
              id="metadata"
              checked={exportConfig.includeMetadata}
              onCheckedChange={(checked) =>
                setExportConfig(prev => ({ ...prev, includeMetadata: checked }))
              }
            />
          </div>

          {/* 包含目录 */}
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label htmlFor="toc">Table of Contents</Label>
              <p className="text-xs text-muted-foreground">
                Generate TABLE_OF_CONTENTS.md file
              </p>
            </div>
            <Switch
              id="toc"
              checked={exportConfig.includeTableOfContents}
              onCheckedChange={(checked) =>
                setExportConfig(prev => ({ ...prev, includeTableOfContents: checked }))
              }
            />
          </div>
        </div>
      </div>

      <Separator />

      {/* 文件名格式 */}
      <div className="space-y-3">
        <Label>File Naming</Label>
        <RadioGroup
          value={exportConfig.fileNameFormat}
          onValueChange={(value: 'title' | 'id') =>
            setExportConfig(prev => ({ ...prev, fileNameFormat: value }))
          }
        >
          <div className="flex items-center space-x-2">
            <RadioGroupItem value="title" id="title" />
            <Label htmlFor="title" className="text-sm">
              Use page titles (e.g., "getting_started.md")
            </Label>
          </div>
          <div className="flex items-center space-x-2">
            <RadioGroupItem value="id" id="id" />
            <Label htmlFor="id" className="text-sm">
              Use page IDs (e.g., "page_123.md")
            </Label>
          </div>
        </RadioGroup>
      </div>

      <Separator />

      {/* 文件夹结构 */}
      <div className="space-y-3">
        <Label>Folder Structure</Label>
        <RadioGroup
          value={exportConfig.folderStructure}
          onValueChange={(value: 'flat' | 'hierarchical') =>
            setExportConfig(prev => ({ ...prev, folderStructure: value }))
          }
        >
          <div className="flex items-center space-x-2">
            <RadioGroupItem value="hierarchical" id="hierarchical" />
            <Label htmlFor="hierarchical" className="text-sm">
              Hierarchical (organized in folders)
            </Label>
          </div>
          <div className="flex items-center space-x-2">
            <RadioGroupItem value="flat" id="flat" />
            <Label htmlFor="flat" className="text-sm">
              Flat (all files in root)
            </Label>
          </div>
        </RadioGroup>
      </div>

      {/* 预览信息 */}
      <div className="p-3 bg-muted/50 rounded-lg">
        <div className="flex items-center gap-2 mb-2">
          <FileText className="h-4 w-4" />
          <span className="text-sm font-medium">Export Preview</span>
        </div>
        <div className="text-xs text-muted-foreground space-y-1">
          <div>📄 {wiki.pages.length} pages</div>
          <div>📑 {wiki.sections.length} root sections</div>
          <div>📁 Structure: {exportConfig.folderStructure}</div>
          <div>🏷️ Naming: {exportConfig.fileNameFormat}</div>
        </div>
      </div>
    </div>
  );

  // 渲染导出进度
  const renderProgress = () => (
    <div className="space-y-6">
      <div className="text-center">
        <motion.div
          animate={{ rotate: 360 }}
          transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
          className="inline-block"
        >
          <Archive className="h-8 w-8 text-primary" />
        </motion.div>
        <h3 className="font-medium mt-2">Exporting Wiki...</h3>
        <p className="text-sm text-muted-foreground">
          Creating markdown files and packaging them
        </p>
      </div>

      <div className="space-y-2">
        <div className="flex justify-between text-sm">
          <span>Progress</span>
          <span>{exportProgress}%</span>
        </div>
        <Progress value={exportProgress} className="h-2" />
      </div>

      <div className="text-xs text-muted-foreground text-center">
        Please wait while we prepare your download...
      </div>
    </div>
  );

  // 渲染完成状态
  const renderCompleted = () => (
    <div className="space-y-6">
      <div className="text-center">
        <CheckCircle className="h-8 w-8 text-green-500 mx-auto" />
        <h3 className="font-medium mt-2">Export Completed!</h3>
        <p className="text-sm text-muted-foreground">
          Your wiki has been successfully exported and downloaded
        </p>
      </div>

      {exportStats && (
        <div className="grid grid-cols-2 gap-4">
          <div className="text-center p-3 bg-muted/50 rounded-lg">
            <div className="text-lg font-semibold">{exportStats.totalFiles}</div>
            <div className="text-xs text-muted-foreground">Files Created</div>
          </div>
          <div className="text-center p-3 bg-muted/50 rounded-lg">
            <div className="text-lg font-semibold">
              {(exportStats.zipSize / 1024).toFixed(1)}KB
            </div>
            <div className="text-xs text-muted-foreground">Archive Size</div>
          </div>
          <div className="text-center p-3 bg-muted/50 rounded-lg">
            <div className="text-lg font-semibold">{exportStats.totalPages}</div>
            <div className="text-xs text-muted-foreground">Pages</div>
          </div>
          <div className="text-center p-3 bg-muted/50 rounded-lg">
            <div className="text-lg font-semibold">
              {exportStats.exportTime.toFixed(0)}ms
            </div>
            <div className="text-xs text-muted-foreground">Export Time</div>
          </div>
        </div>
      )}
    </div>
  );

  // 渲染错误状态
  const renderError = () => (
    <div className="space-y-6">
      <div className="text-center">
        <div className="h-8 w-8 bg-destructive/10 text-destructive rounded-full flex items-center justify-center mx-auto">
          <span className="text-sm">!</span>
        </div>
        <h3 className="font-medium mt-2">Export Failed</h3>
        <p className="text-sm text-muted-foreground">
          An error occurred while exporting the wiki
        </p>
      </div>
    </div>
  );

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Download className="h-5 w-5" />
            Export Wiki
          </DialogTitle>
          <DialogDescription>
            Export "{wiki.title}" as a markdown archive
          </DialogDescription>
        </DialogHeader>

        <div className="py-4">
          {exportState === 'idle' && renderConfiguration()}
          {exportState === 'exporting' && renderProgress()}
          {exportState === 'completed' && renderCompleted()}
          {exportState === 'error' && renderError()}
        </div>

        <div className="flex justify-end gap-2">
          {exportState === 'idle' && (
            <>
              <Button variant="outline" onClick={handleClose}>
                Cancel
              </Button>
              <Button onClick={handleStartExport}>
                <Download className="h-4 w-4 mr-2" />
                Export
              </Button>
            </>
          )}
          
          {exportState === 'exporting' && (
            <Button disabled>
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              Exporting...
            </Button>
          )}
          
          {(exportState === 'completed' || exportState === 'error') && (
            <>
              {exportState === 'error' && (
                <Button variant="outline" onClick={handleReset}>
                  Try Again
                </Button>
              )}
              <Button onClick={handleClose}>
                Close
              </Button>
            </>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
};

export { WikiExportDialog };
