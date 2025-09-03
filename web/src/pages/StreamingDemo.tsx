/**
 * 流式研究功能演示页面
 * 展示新的流式深度研究功能
 */

import React, { useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { StreamingResearchInterface, DeepResearchInterface } from '@/components/research';
import { 
  Zap, 
  Clock, 
  Sparkles, 
  Brain, 
  ArrowLeft,
  CheckCircle,
  XCircle,
  AlertCircle,
  Info
} from 'lucide-react';
import { useNavigate } from 'react-router-dom';

const StreamingDemoPage: React.FC = () => {
  const navigate = useNavigate();
  const [demoMode, setDemoMode] = useState<'streaming' | 'traditional'>('streaming');
  
  // 模拟的仓库 ID（在实际应用中这应该来自路由参数）
  const demoRepositoryId = 'demo-repo-123';

  const handleResearchComplete = (result: any) => {
    console.log('Demo research completed:', result);
  };

  const handleGoBack = () => {
    navigate('/');
  };

  return (
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
            返回首页
          </Button>
          <div className="flex items-center space-x-2">
            <Brain className="h-6 w-6" />
            <h1 className="text-lg font-semibold">流式研究功能演示</h1>
            <Badge variant="secondary">Beta</Badge>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="container py-6 space-y-6">
        {/* 功能介绍 */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Sparkles className="h-5 w-5 text-blue-600" />
              全新流式深度研究功能
            </CardTitle>
            <CardDescription>
              体验下一代实时研究技术，获得即时反馈和流畅的用户体验
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="flex items-start gap-3 p-4 bg-blue-50 dark:bg-blue-950/20 rounded-lg border border-blue-200 dark:border-blue-800">
                <Zap className="h-5 w-5 text-blue-600 mt-0.5" />
                <div>
                  <h4 className="font-medium text-blue-900 dark:text-blue-100">实时流式响应</h4>
                  <p className="text-sm text-blue-700 dark:text-blue-300 mt-1">
                    毫秒级实时更新，无需等待完整结果
                  </p>
                </div>
              </div>
              
              <div className="flex items-start gap-3 p-4 bg-green-50 dark:bg-green-950/20 rounded-lg border border-green-200 dark:border-green-800">
                <CheckCircle className="h-5 w-5 text-green-600 mt-0.5" />
                <div>
                  <h4 className="font-medium text-green-900 dark:text-green-100">可中断控制</h4>
                  <p className="text-sm text-green-700 dark:text-green-300 mt-1">
                    随时停止或重新开始研究过程
                  </p>
                </div>
              </div>
              
              <div className="flex items-start gap-3 p-4 bg-purple-50 dark:bg-purple-950/20 rounded-lg border border-purple-200 dark:border-purple-800">
                <Brain className="h-5 w-5 text-purple-600 mt-0.5" />
                <div>
                  <h4 className="font-medium text-purple-900 dark:text-purple-100">智能分析</h4>
                  <p className="text-sm text-purple-700 dark:text-purple-300 mt-1">
                    深度理解代码结构和业务逻辑
                  </p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* 技术对比 */}
        <Card>
          <CardHeader>
            <CardTitle>技术对比</CardTitle>
            <CardDescription>
              了解流式研究相比传统方式的优势
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="overflow-x-auto">
              <table className="w-full border-collapse">
                <thead>
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">特性</th>
                    <th className="text-left p-3 font-medium text-blue-600">流式研究</th>
                    <th className="text-left p-3 font-medium text-gray-600">传统研究</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">响应时间</td>
                    <td className="p-3">
                      <div className="flex items-center gap-2">
                        <CheckCircle className="h-4 w-4 text-green-500" />
                        <span>毫秒级实时</span>
                      </div>
                    </td>
                    <td className="p-3">
                      <div className="flex items-center gap-2">
                        <Clock className="h-4 w-4 text-yellow-500" />
                        <span>1-5秒延迟</span>
                      </div>
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">用户体验</td>
                    <td className="p-3">
                      <div className="flex items-center gap-2">
                        <CheckCircle className="h-4 w-4 text-green-500" />
                        <span>流畅无卡顿</span>
                      </div>
                    </td>
                    <td className="p-3">
                      <div className="flex items-center gap-2">
                        <AlertCircle className="h-4 w-4 text-yellow-500" />
                        <span>间歇性更新</span>
                      </div>
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">网络效率</td>
                    <td className="p-3">
                      <div className="flex items-center gap-2">
                        <CheckCircle className="h-4 w-4 text-green-500" />
                        <span>单一长连接</span>
                      </div>
                    </td>
                    <td className="p-3">
                      <div className="flex items-center gap-2">
                        <XCircle className="h-4 w-4 text-red-500" />
                        <span>频繁HTTP请求</span>
                      </div>
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">错误恢复</td>
                    <td className="p-3">
                      <div className="flex items-center gap-2">
                        <CheckCircle className="h-4 w-4 text-green-500" />
                        <span>自动重连</span>
                      </div>
                    </td>
                    <td className="p-3">
                      <div className="flex items-center gap-2">
                        <AlertCircle className="h-4 w-4 text-yellow-500" />
                        <span>手动重试</span>
                      </div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>

        {/* 演示区域 */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Brain className="h-5 w-5" />
              功能演示
            </CardTitle>
            <CardDescription>
              选择研究模式并体验不同的功能特性
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Tabs value={demoMode} onValueChange={(value) => setDemoMode(value as 'streaming' | 'traditional')}>
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
              
              <div className="mt-6">
                <TabsContent value="streaming" className="space-y-4">
                  <div className="flex items-start gap-3 p-4 bg-blue-50 dark:bg-blue-950/20 rounded-lg border border-blue-200 dark:border-blue-800">
                    <Info className="h-5 w-5 text-blue-600 mt-0.5" />
                    <div>
                      <h4 className="font-medium text-blue-900 dark:text-blue-100">流式研究模式</h4>
                      <p className="text-sm text-blue-700 dark:text-blue-300 mt-1">
                        使用 Server-Sent Events (SSE) 技术实现实时数据流传输，提供最佳的用户体验。
                      </p>
                    </div>
                  </div>
                  
                  <StreamingResearchInterface
                    repositoryId={demoRepositoryId}
                    onResearchComplete={handleResearchComplete}
                  />
                </TabsContent>
                
                <TabsContent value="traditional" className="space-y-4">
                  <div className="flex items-start gap-3 p-4 bg-gray-50 dark:bg-gray-950/20 rounded-lg border border-gray-200 dark:border-gray-800">
                    <Info className="h-5 w-5 text-gray-600 mt-0.5" />
                    <div>
                      <h4 className="font-medium text-gray-900 dark:text-gray-100">传统研究模式</h4>
                      <p className="text-sm text-gray-700 dark:text-gray-300 mt-1">
                        使用传统的轮询方式获取研究进度，适合网络环境不稳定的情况。
                      </p>
                    </div>
                  </div>
                  
                  <DeepResearchInterface
                    repositoryId={demoRepositoryId}
                    onResearchComplete={handleResearchComplete}
                  />
                </TabsContent>
              </div>
            </Tabs>
          </CardContent>
        </Card>

        {/* 使用说明 */}
        <Card>
          <CardHeader>
            <CardTitle>使用说明</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex items-start gap-3">
                <div className="flex-shrink-0 w-6 h-6 bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-400 rounded-full flex items-center justify-center text-sm font-medium">
                  1
                </div>
                <div>
                  <h4 className="font-medium">选择研究模式</h4>
                  <p className="text-sm text-muted-foreground mt-1">
                    推荐使用流式研究模式以获得最佳体验
                  </p>
                </div>
              </div>
              
              <div className="flex items-start gap-3">
                <div className="flex-shrink-0 w-6 h-6 bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-400 rounded-full flex items-center justify-center text-sm font-medium">
                  2
                </div>
                <div>
                  <h4 className="font-medium">输入研究问题</h4>
                  <p className="text-sm text-muted-foreground mt-1">
                    描述您想要深入了解的代码库相关问题
                  </p>
                </div>
              </div>
              
              <div className="flex items-start gap-3">
                <div className="flex-shrink-0 w-6 h-6 bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-400 rounded-full flex items-center justify-center text-sm font-medium">
                  3
                </div>
                <div>
                  <h4 className="font-medium">配置研究参数</h4>
                  <p className="text-sm text-muted-foreground mt-1">
                    调整最大迭代次数和源数量以控制研究深度
                  </p>
                </div>
              </div>
              
              <div className="flex items-start gap-3">
                <div className="flex-shrink-0 w-6 h-6 bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-400 rounded-full flex items-center justify-center text-sm font-medium">
                  4
                </div>
                <div>
                  <h4 className="font-medium">观察实时进度</h4>
                  <p className="text-sm text-muted-foreground mt-1">
                    流式模式下可以看到实时的研究进度和中间结果
                  </p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </main>
    </div>
  );
};

export default StreamingDemoPage;
