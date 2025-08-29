/**
 * Wiki生成配置对话框组件
 * 允许用户配置并手动触发Wiki生成
 */

import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Separator } from "@/components/ui/separator";
import { useToast } from "@/hooks/use-toast";
import { useGenerateWiki } from "@/hooks/use-api";
import { BookOpen, Settings, Loader2 } from "lucide-react";

// Types
import { WikiGenerationConfig } from "@/types/api";

interface WikiGenerationDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessionId: string;
  repositoryName?: string;
}

const WikiGenerationDialog = ({
  open,
  onOpenChange,
  sessionId,
  repositoryName,
}: WikiGenerationDialogProps) => {
  const { toast } = useToast();
  const generateWikiMutation = useGenerateWiki();

  // Configuration state
  const [config, setConfig] = useState<WikiGenerationConfig>({
    language: "en",
    max_pages: 50,
    include_diagrams: true,
    comprehensive_view: false,
  });

  // Handle configuration changes
  const handleConfigChange = (key: keyof WikiGenerationConfig, value: any) => {
    setConfig(prev => ({
      ...prev,
      [key]: value,
    }));
  };

  // Handle form submission
  const handleGenerate = async () => {
    try {
      await generateWikiMutation.mutateAsync({
        session_id: sessionId,
        config,
      });

      toast({
        title: "Wiki Generation Started",
        description: "Your wiki is being generated. This may take a few minutes.",
      });

      onOpenChange(false);
    } catch (error) {
      console.error("Failed to generate wiki:", error);
      toast({
        title: "Generation Failed",
        description: "Failed to start wiki generation. Please try again.",
        variant: "destructive",
      });
    }
  };

  // Handle dialog close
  const handleClose = () => {
    if (!generateWikiMutation.isPending) {
      onOpenChange(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <BookOpen className="h-5 w-5" />
            Generate Wiki
          </DialogTitle>
          <DialogDescription>
            Configure and generate comprehensive documentation for{" "}
            {repositoryName ? `"${repositoryName}"` : "this repository"}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Basic Configuration */}
          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <Settings className="h-4 w-4" />
              <Label className="text-sm font-medium">Configuration</Label>
            </div>

            {/* Language */}
            <div className="space-y-2">
              <Label htmlFor="language" className="text-sm">
                Language
              </Label>
              <Input
                id="language"
                value={config.language || ""}
                onChange={(e) => handleConfigChange("language", e.target.value)}
                placeholder="en"
                className="h-9"
              />
              <p className="text-xs text-muted-foreground">
                Language code for content generation (e.g., en, zh, ja)
              </p>
            </div>

            {/* Max Pages */}
            <div className="space-y-2">
              <Label htmlFor="max-pages" className="text-sm">
                Maximum Pages
              </Label>
              <Input
                id="max-pages"
                type="number"
                min="1"
                max="200"
                value={config.max_pages || ""}
                onChange={(e) => 
                  handleConfigChange("max_pages", parseInt(e.target.value) || undefined)
                }
                placeholder="50"
                className="h-9"
              />
              <p className="text-xs text-muted-foreground">
                Maximum number of pages to generate (1-200)
              </p>
            </div>
          </div>

          <Separator />

          {/* Advanced Options */}
          <div className="space-y-4">
            <Label className="text-sm font-medium">Advanced Options</Label>

            {/* Include Diagrams */}
            <div className="flex items-center justify-between space-x-2">
              <div className="space-y-0.5">
                <Label htmlFor="include-diagrams" className="text-sm">
                  Include Diagrams
                </Label>
                <p className="text-xs text-muted-foreground">
                  Generate Mermaid diagrams and visualizations
                </p>
              </div>
              <Switch
                id="include-diagrams"
                checked={config.include_diagrams || false}
                onCheckedChange={(checked) =>
                  handleConfigChange("include_diagrams", checked)
                }
              />
            </div>

            {/* Comprehensive View */}
            <div className="flex items-center justify-between space-x-2">
              <div className="space-y-0.5">
                <Label htmlFor="comprehensive-view" className="text-sm">
                  Comprehensive View
                </Label>
                <p className="text-xs text-muted-foreground">
                  Generate detailed analysis for all code files
                </p>
              </div>
              <Switch
                id="comprehensive-view"
                checked={config.comprehensive_view || false}
                onCheckedChange={(checked) =>
                  handleConfigChange("comprehensive_view", checked)
                }
              />
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={handleClose}
            disabled={generateWikiMutation.isPending}
          >
            Cancel
          </Button>
          <Button
            onClick={handleGenerate}
            disabled={generateWikiMutation.isPending}
          >
            {generateWikiMutation.isPending ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Generating...
              </>
            ) : (
              <>
                <BookOpen className="mr-2 h-4 w-4" />
                Generate Wiki
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default WikiGenerationDialog;
