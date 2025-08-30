/**
 * 智能研究页面
 * 提供深度研究功能的完整界面
 */

import React from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { DeepResearchInterface } from '@/components/research';
import { AuthRequired, FeatureConditional } from '@/components/AuthProvider';
import { useRepositoryById } from '@/store/app-store';
import { ArrowLeft, Brain, AlertCircle } from 'lucide-react';

// ============================================================================
// 主组件
// ============================================================================

const ResearchPage: React.FC = () => {
  const { repositoryId } = useParams<{ repositoryId: string }>();
  const navigate = useNavigate();
  const repository = useRepositoryById(repositoryId || '');

  // 处理研究完成
  const handleResearchComplete = (conclusion: string) => {
    console.log('Research completed:', conclusion);
    // 可以在这里添加额外的处理逻辑，比如保存结果、显示通知等
  };

  // 返回首页
  const handleGoBack = () => {
    navigate('/');
  };

  // 如果没有找到仓库
  if (!repositoryId) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-destructive">
              <AlertCircle className="h-5 w-5" />
              Invalid Repository
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground mb-4">
              No repository ID provided. Please select a repository from the main page.
            </p>
            <Button onClick={handleGoBack} className="w-full">
              <ArrowLeft className="h-4 w-4 mr-2" />
              Back to Repositories
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  // 如果仓库不存在
  if (!repository) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-destructive">
              <AlertCircle className="h-5 w-5" />
              Repository Not Found
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground mb-4">
              The requested repository could not be found. It may have been deleted or you may not have access to it.
            </p>
            <Button onClick={handleGoBack} className="w-full">
              <ArrowLeft className="h-4 w-4 mr-2" />
              Back to Repositories
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  // 如果仓库未索引
  if (repository.status !== 'indexed') {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-yellow-600">
              <AlertCircle className="h-5 w-5" />
              Repository Not Ready
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground mb-4">
              This repository is still being indexed. Please wait for the indexing process to complete before starting research.
            </p>
            <div className="space-y-2">
              <p className="text-sm">
                <strong>Repository:</strong> {repository.name}
              </p>
              <p className="text-sm">
                <strong>Status:</strong> {repository.status}
              </p>
            </div>
            <Button onClick={handleGoBack} className="w-full mt-4">
              <ArrowLeft className="h-4 w-4 mr-2" />
              Back to Repositories
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <AuthRequired
      fallback={
        <div className="min-h-screen flex items-center justify-center">
          <Card className="w-full max-w-md">
            <CardHeader>
              <CardTitle>Authentication Required</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-muted-foreground mb-4">
                Please sign in to access the research features.
              </p>
              <Button onClick={() => navigate('/login')} className="w-full">
                Sign In
              </Button>
            </CardContent>
          </Card>
        </div>
      }
    >
      <FeatureConditional
        feature="research_engine"
        fallback={
          <div className="min-h-screen flex items-center justify-center">
            <Card className="w-full max-w-md">
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Brain className="h-5 w-5" />
                  Research Feature Unavailable
                </CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-muted-foreground mb-4">
                  The deep research feature is not available in this deployment configuration.
                </p>
                <Button onClick={handleGoBack} className="w-full">
                  <ArrowLeft className="h-4 w-4 mr-2" />
                  Back to Repositories
                </Button>
              </CardContent>
            </Card>
          </div>
        }
      >
        <div className="min-h-screen bg-background">
          {/* Header */}
          <header className="border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
            <div className="container flex h-14 items-center">
              <Button
                variant="ghost"
                size="sm"
                onClick={handleGoBack}
                className="mr-4"
              >
                <ArrowLeft className="h-4 w-4 mr-2" />
                Back
              </Button>
              
              <div className="flex items-center gap-3">
                <Brain className="h-6 w-6 text-primary" />
                <div>
                  <h1 className="text-lg font-semibold">Deep Research</h1>
                  <p className="text-sm text-muted-foreground">
                    {repository.name}
                  </p>
                </div>
              </div>
            </div>
          </header>

          {/* Main Content */}
          <main className="container py-6">
            <DeepResearchInterface
              repositoryId={repositoryId}
              onResearchComplete={handleResearchComplete}
            />
          </main>
        </div>
      </FeatureConditional>
    </AuthRequired>
  );
};

export default ResearchPage;
