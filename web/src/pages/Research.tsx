/**
 * 智能研究页面
 * 提供深度研究功能的完整界面
 */

import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { DeepResearchInterface, StreamingResearchInterface } from '@/components/research';
import { AuthRequired, FeatureConditional } from '@/components/AuthProvider';
import { useRepositoryById } from '@/store/app-store';
import { useRepositories } from '@/hooks/use-api';
import { ArrowLeft, Brain, AlertCircle, Zap, Clock, Sparkles } from 'lucide-react';

// ============================================================================
// 主组件
// ============================================================================

const ResearchPage: React.FC = () => {
  const { repositoryId } = useParams<{ repositoryId: string }>();
  const navigate = useNavigate();
  const repository = useRepositoryById(repositoryId || '');
  const [researchMode, setResearchMode] = useState<'streaming' | 'traditional'>('streaming');

  // 确保数据刷新
  const { refetch } = useRepositories();

  useEffect(() => {
    // 强制重新获取repositories数据以确保repository信息是最新的
    refetch();
  }, [refetch]);

  // 处理研究完成
  const handleResearchComplete = (result: any) => {
    console.log('Research completed:', result);
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
          <main className="container py-6 space-y-6">
            {/* 研究模式选择 */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Brain className="h-5 w-5" />
                  研究模式选择
                </CardTitle>
                <CardDescription>
                  选择适合您需求的研究模式
                </CardDescription>
              </CardHeader>
              <CardContent>
                <Tabs value={researchMode} onValueChange={(value) => setResearchMode(value as 'streaming' | 'traditional')}>
                  <TabsList className="grid w-full grid-cols-2">
                    <TabsTrigger value="streaming" className="flex items-center gap-2">
                      <Zap className="h-4 w-4" />
                      流式研究
                      <Badge variant="secondary" className="ml-1">推荐</Badge>
                    </TabsTrigger>
                    <TabsTrigger value="traditional" className="flex items-center gap-2">
                      <Clock className="h-4 w-4" />
                      传统研究
                    </TabsTrigger>
                  </TabsList>

                  <div className="mt-4 space-y-4">
                    <TabsContent value="streaming" className="space-y-3">
                      <div className="flex items-start gap-3 p-4 bg-blue-50 dark:bg-blue-950/20 rounded-lg border border-blue-200 dark:border-blue-800">
                        <Sparkles className="h-5 w-5 text-blue-600 mt-0.5" />
                        <div>
                          <h4 className="font-medium text-blue-900 dark:text-blue-100">实时流式研究</h4>
                          <p className="text-sm text-blue-700 dark:text-blue-300 mt-1">
                            获得即时反馈和实时进度更新，支持中断和恢复，提供最佳的用户体验。
                          </p>
                          <ul className="text-xs text-blue-600 dark:text-blue-400 mt-2 space-y-1">
                            <li>• 毫秒级实时更新</li>
                            <li>• 可随时中断研究</li>
                            <li>• 流畅的用户体验</li>
                            <li>• 自动错误恢复</li>
                          </ul>
                        </div>
                      </div>
                    </TabsContent>

                    <TabsContent value="traditional" className="space-y-3">
                      <div className="flex items-start gap-3 p-4 bg-gray-50 dark:bg-gray-950/20 rounded-lg border border-gray-200 dark:border-gray-800">
                        <Clock className="h-5 w-5 text-gray-600 mt-0.5" />
                        <div>
                          <h4 className="font-medium text-gray-900 dark:text-gray-100">传统轮询研究</h4>
                          <p className="text-sm text-gray-700 dark:text-gray-300 mt-1">
                            使用传统的轮询方式获取研究进度，适合网络环境不稳定的情况。
                          </p>
                          <ul className="text-xs text-gray-600 dark:text-gray-400 mt-2 space-y-1">
                            <li>• 定期更新进度</li>
                            <li>• 网络兼容性好</li>
                            <li>• 稳定可靠</li>
                            <li>• 支持断点续传</li>
                          </ul>
                        </div>
                      </div>
                    </TabsContent>
                  </div>
                </Tabs>
              </CardContent>
            </Card>

            {/* 研究界面 */}
            {researchMode === 'streaming' ? (
              <StreamingResearchInterface
                repositoryId={repositoryId}
                onResearchComplete={handleResearchComplete}
              />
            ) : (
              <DeepResearchInterface
                repositoryId={repositoryId}
                onResearchComplete={handleResearchComplete}
              />
            )}
          </main>
        </div>
      </FeatureConditional>
    </AuthRequired>
  );
};

export default ResearchPage;
