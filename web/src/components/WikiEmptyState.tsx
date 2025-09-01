/**
 * Wiki空状态组件
 * 当wiki不存在或生成失败时显示
 */

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { BookOpen, Plus, RefreshCw, AlertCircle } from "lucide-react";
import { useNavigate } from "react-router-dom";

interface WikiEmptyStateProps {
  repositoryId: string;
  type: 'not_found' | 'generation_failed' | 'no_content';
  onGenerateWiki?: () => void;
  onRetry?: () => void;
}

export const WikiEmptyState = ({ 
  repositoryId, 
  type, 
  onGenerateWiki, 
  onRetry 
}: WikiEmptyStateProps) => {
  const navigate = useNavigate();

  const getStateConfig = () => {
    switch (type) {
      case 'not_found':
        return {
          icon: BookOpen,
          title: "No Wiki Available",
          description: "This repository doesn't have a wiki yet. Generate one to create comprehensive documentation.",
          primaryAction: {
            label: "Generate Wiki",
            icon: Plus,
            onClick: onGenerateWiki || (() => navigate('/'))
          },
          secondaryAction: {
            label: "Back to Home",
            onClick: () => navigate('/')
          }
        };
      
      case 'generation_failed':
        return {
          icon: AlertCircle,
          title: "Wiki Generation Failed",
          description: "The wiki generation process encountered an error. This might be due to insufficient repository content or processing issues.",
          primaryAction: {
            label: "Try Again",
            icon: RefreshCw,
            onClick: onRetry || onGenerateWiki || (() => navigate('/'))
          },
          secondaryAction: {
            label: "Back to Home",
            onClick: () => navigate('/')
          }
        };
      
      case 'no_content':
        return {
          icon: BookOpen,
          title: "Empty Wiki",
          description: "The wiki was generated but contains no content. The repository might not have sufficient documentation to generate meaningful wiki pages.",
          primaryAction: {
            label: "Regenerate Wiki",
            icon: RefreshCw,
            onClick: onRetry || onGenerateWiki || (() => navigate('/'))
          },
          secondaryAction: {
            label: "Back to Home",
            onClick: () => navigate('/')
          }
        };
      
      default:
        return {
          icon: BookOpen,
          title: "Wiki Unavailable",
          description: "Unable to load wiki content.",
          primaryAction: {
            label: "Back to Home",
            onClick: () => navigate('/')
          }
        };
    }
  };

  const config = getStateConfig();
  const Icon = config.icon;
  const PrimaryIcon = config.primaryAction.icon;

  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <Card className="max-w-md w-full">
        <CardHeader className="text-center">
          <div className="mx-auto mb-4 p-3 bg-muted rounded-full w-fit">
            <Icon className="h-8 w-8 text-muted-foreground" />
          </div>
          <CardTitle className="text-xl">{config.title}</CardTitle>
          <CardDescription className="text-center leading-relaxed">
            {config.description}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <Button 
            onClick={config.primaryAction.onClick}
            className="w-full"
            size="lg"
          >
            {PrimaryIcon && <PrimaryIcon className="h-4 w-4 mr-2" />}
            {config.primaryAction.label}
          </Button>
          
          {config.secondaryAction && (
            <Button 
              onClick={config.secondaryAction.onClick}
              variant="outline"
              className="w-full"
            >
              {config.secondaryAction.label}
            </Button>
          )}
        </CardContent>
      </Card>
    </div>
  );
};
