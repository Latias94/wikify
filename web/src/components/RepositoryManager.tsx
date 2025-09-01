import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { useToast } from "@/hooks/use-toast";
import { useGlobalWebSocket } from "@/hooks/use-websocket-manager";
import {
  GitBranch,
  Plus,
  MessageCircle,
  BookOpen,
  RefreshCw,
  Trash2,
  CheckCircle,
  Loader2,
  Globe,
  Folder,
  AlertCircle,
  Brain
} from "lucide-react";

// API hooks
import {
  useRepositories,
  useInitializeRepository,
  useReindexRepository,
  useDeleteRepository
} from "@/hooks/use-api";

// Store hooks
import {
  useRepositories as useRepositoriesStore,
  useErrors
} from "@/store/app-store";

// Types
import { Repository, InitializeRepositoryRequest } from "@/types/api";
import { InitializeRepositoryFormData } from "@/types/ui";

// Components
import WikiGenerationDialog from "@/components/WikiGenerationDialog";
import UniversalProgress, { ProgressPanel } from "@/components/UniversalProgress";

const RepositoryManager = () => {
  const navigate = useNavigate();
  const { toast } = useToast();



  // Form state
  const [formData, setFormData] = useState<InitializeRepositoryFormData>({
    repository: '',
    repo_type: 'remote',
    auto_generate_wiki: true, // 默认启用自动生成wiki
  });

  // Local loading state for immediate feedback
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [reindexingRepos, setReindexingRepos] = useState<Set<string>>(new Set());

  // Wiki generation dialog state
  const [showWikiDialog, setShowWikiDialog] = useState(false);
  const [selectedRepoForWiki, setSelectedRepoForWiki] = useState<Repository | null>(null);

  // API hooks
  const { isLoading: isLoadingRepos, refetch } = useRepositories();
  const initializeRepositoryMutation = useInitializeRepository();
  const reindexRepositoryMutation = useReindexRepository();
  const deleteRepositoryMutation = useDeleteRepository();

  // Store state
  const repositories = useRepositoriesStore();
  const errors = useErrors();

  // 全局 WebSocket 连接，自动处理所有进度消息
  const { isConnected: wsConnected, lastError: wsError } = useGlobalWebSocket();

  // Derived state
  const isAdding = isSubmitting; // Use local state instead of mutation pending
  const hasError = !!errors.repositories;





  const handleAddRepository = async () => {
    // 防止重复提交
    if (isSubmitting) {
      return;
    }

    if (!formData.repository.trim()) {
      toast({
        title: "Path Required",
        description: "Please enter a repository URL or path",
        variant: "destructive"
      });
      return;
    }

    const requestData: InitializeRepositoryRequest = {
      repository: formData.repository,
      repo_type: formData.repo_type === 'remote' ? 'github' : 'local',
      auto_generate_wiki: formData.auto_generate_wiki,
    };

    // Set local loading state for immediate feedback
    setIsSubmitting(true);

    try {
      // Fire and forget - don't wait for the full indexing process
      initializeRepositoryMutation.mutate(requestData, {
        onSuccess: () => {
          // Reset form immediately after successful request
          setFormData({
            repository: '',
            repo_type: 'remote',
            auto_generate_wiki: true,
          });

          // React Query会自动invalidate repositories查询，不需要手动refetch
          // refetch(); // 移除手动refetch，避免重复请求
        },
        onSettled: () => {
          // Always reset loading state after request completes (success or error)
          setIsSubmitting(false);
        }
      });

      // Reset loading state after a short delay to show immediate feedback
      setTimeout(() => {
        setIsSubmitting(false);
      }, 1000); // 1 second loading feedback

    } catch (error) {
      // Error is handled by the mutation's onError callback
      console.error('Failed to add repository:', error);
      setIsSubmitting(false);
    }
  };

  const handleRemoveRepository = async (repository: Repository) => {
    try {
      await deleteRepositoryMutation.mutateAsync(repository.id);
      refetch();
    } catch (error) {
      console.error('Failed to delete repository:', error);
    }
  };

  const handleStartChat = async (repository: Repository) => {
    if (repository.status !== 'indexed') {
      toast({
        title: "Repository Not Ready",
        description: "Please wait for the repository to finish indexing before starting a chat.",
        variant: "destructive"
      });
      return;
    }

    try {
      // In Wikify, the repository.id is the session_id
      // No need to create a separate session
      navigate(`/chat/${repository.id}`);
    } catch (error) {
      console.error('Failed to navigate to chat:', error);
    }
  };

  const handleStartWiki = async (repository: Repository) => {
    if (repository.status !== 'indexed') {
      toast({
        title: "Repository Not Ready",
        description: "Please wait for the repository to finish indexing before viewing the wiki.",
        variant: "destructive"
      });
      return;
    }

    // Only prevent navigation if wiki is currently being generated
    if (repository.wiki_status === 'generating') {
      toast({
        title: "Wiki Being Generated",
        description: "Please wait for the wiki to finish generating.",
        variant: "destructive"
      });
      return;
    }

    try {
      // Always navigate to wiki page - let WikiViewer handle error states
      navigate(`/wiki/${repository.id}`);
    } catch (error) {
      console.error('Failed to navigate to wiki:', error);
      toast({
        title: "Navigation Error",
        description: "Failed to open wiki page. Please try again.",
        variant: "destructive"
      });
    }
  };

  const handleGenerateWiki = (repository: Repository) => {
    if (repository.status !== 'indexed') {
      toast({
        title: "Repository Not Ready",
        description: "Please wait for the repository to finish indexing before generating wiki.",
        variant: "destructive"
      });
      return;
    }

    setSelectedRepoForWiki(repository);
    setShowWikiDialog(true);
  };

  const handleStartResearch = async (repository: Repository) => {
    if (repository.status !== 'indexed') {
      toast({
        title: "Repository Not Ready",
        description: "Please wait for the repository to finish indexing before starting research.",
        variant: "destructive"
      });
      return;
    }

    try {
      navigate(`/research/${repository.id}`);
    } catch (error) {
      console.error('Failed to navigate to research:', error);
      toast({
        title: "Navigation Error",
        description: "Failed to open research page. Please try again.",
        variant: "destructive"
      });
    }
  };

  const handleRefreshRepository = async (repository: Repository) => {
    // 防止重复提交
    if (reindexingRepos.has(repository.id)) {
      return;
    }

    // 如果正在索引，不允许重新索引
    if (repository.status === 'indexing') {
      toast({
        title: "Repository is indexing",
        description: "Please wait for the current indexing to complete.",
        variant: "destructive"
      });
      return;
    }

    // 如果已经索引完成，需要用户确认
    if (repository.status === 'indexed') {
      const confirmed = window.confirm(
        `Are you sure you want to reindex "${repository.name}"?\n\n` +
        "This will reset the current index and start the indexing process again. " +
        "The repository will be unavailable for chat during reindexing."
      );

      if (!confirmed) {
        return;
      }
    }

    // Set local loading state
    setReindexingRepos(prev => new Set(prev).add(repository.id));

    try {
      // Fire and forget - don't wait for the full reindexing process
      reindexRepositoryMutation.mutate(repository.id, {
        onSuccess: () => {
          // Refresh repositories list
          refetch();
        },
        onSettled: () => {
          // Always reset loading state after request completes
          setReindexingRepos(prev => {
            const newSet = new Set(prev);
            newSet.delete(repository.id);
            return newSet;
          });
        }
      });

      // Reset loading state after a short delay
      setTimeout(() => {
        setReindexingRepos(prev => {
          const newSet = new Set(prev);
          newSet.delete(repository.id);
          return newSet;
        });
      }, 1000); // 1 second loading feedback

    } catch (error) {
      console.error('Failed to reindex repository:', error);
      setReindexingRepos(prev => {
        const newSet = new Set(prev);
        newSet.delete(repository.id);
        return newSet;
      });
    }
  };

  const getStatusIcon = (status: Repository['status']) => {
    switch (status) {
      case 'indexed':
        return <CheckCircle className="h-4 w-4 text-success" />;
      case 'indexing':
        return <Loader2 className="h-4 w-4 text-warning animate-spin" />;
      case 'failed':
        return <AlertCircle className="h-4 w-4 text-destructive" />;
      case 'created':
        return <Loader2 className="h-4 w-4 text-muted-foreground" />;
      case 'archived':
        return <Folder className="h-4 w-4 text-muted-foreground" />;
      default:
        return null;
    }
  };

  const getStatusText = (repo: Repository) => {
    switch (repo.status) {
      case 'indexed':
        return 'Ready';
      case 'indexing':
        return 'Indexing...';
      case 'failed':
        return 'Failed';
      case 'created':
        return 'Created';
      case 'archived':
        return 'Archived';
      default:
        return 'Unknown';
    }
  };

  const getRepoTypeIcon = (repoType: Repository['repo_type']) => {
    switch (repoType) {
      case 'github':
        return <GitBranch className="h-5 w-5 text-primary" />;
      case 'git':
        return <Globe className="h-5 w-5 text-primary" />;
      case 'local':
        return <Folder className="h-5 w-5 text-accent-foreground" />;
      default:
        return <Folder className="h-5 w-5 text-muted-foreground" />;
    }
  };

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-8">
      {/* Header */}
      <div className="text-center space-y-4">
        <div className="flex items-center justify-center gap-3">
          <Folder className="h-8 w-8 text-primary" />
          <h1 className="text-3xl font-bold text-foreground">Wikify</h1>
        </div>
        <p className="text-muted-foreground max-w-2xl mx-auto">
          AI-powered code repository documentation and Q&A system.
          Add your repositories and start asking questions about your codebase.
        </p>

        {/* WebSocket 连接状态 */}
        <div className="flex items-center justify-center gap-2 text-sm">
          <div className={`w-2 h-2 rounded-full ${wsConnected ? 'bg-green-500' : 'bg-red-500'}`} />
          <span className="text-muted-foreground">
            {wsConnected ? 'Real-time updates connected' : 'Real-time updates disconnected'}
          </span>
          {wsError && (
            <span className="text-red-500 text-xs">({wsError})</span>
          )}
        </div>
      </div>

      {/* Add Repository Section */}
      <Card className="shadow-medium">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Plus className="h-5 w-5" />
            Add New Repository
          </CardTitle>
          <CardDescription>
            Connect a remote repository or index a local codebase
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-4 mb-4">
            <Button
              variant={formData.repo_type === 'remote' ? 'default' : 'outline'}
              onClick={() => setFormData(prev => ({ ...prev, repo_type: 'remote' }))}
              className="flex items-center gap-2"
            >
              <GitBranch className="h-4 w-4" />
              Remote Repository
            </Button>
            <Button
              variant={formData.repo_type === 'local' ? 'default' : 'outline'}
              onClick={() => setFormData(prev => ({ ...prev, repo_type: 'local' }))}
              className="flex items-center gap-2"
            >
              <Folder className="h-4 w-4" />
              Local Path
            </Button>
          </div>

          <div className="space-y-3">
            {/* Path input - different UI based on repository type */}
            {formData.repo_type === 'remote' ? (
              <Input
                placeholder="https://github.com/user/repository"
                value={formData.repository}
                onChange={(e) => setFormData(prev => ({ ...prev, repository: e.target.value }))}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    handleAddRepository();
                  }
                }}
              />
            ) : (
              <div className="space-y-2">
                <Input
                  placeholder="/path/to/your/project"
                  value={formData.repository}
                  onChange={(e) => setFormData(prev => ({ ...prev, repository: e.target.value }))}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.shiftKey) {
                      e.preventDefault();
                      handleAddRepository();
                    }
                  }}
                />
                <p className="text-xs text-muted-foreground">
                  💡 Enter the full path to your local project folder
                </p>
              </div>
            )}

            {/* Auto Generate Wiki Option */}
            <div className="flex items-center justify-between space-x-2 p-3 border rounded-lg bg-muted/50">
              <div className="space-y-0.5">
                <Label htmlFor="auto-wiki" className="text-sm font-medium">
                  Auto-generate Wiki
                </Label>
                <p className="text-xs text-muted-foreground">
                  Automatically generate wiki documentation after indexing completes
                </p>
              </div>
              <Switch
                id="auto-wiki"
                checked={formData.auto_generate_wiki || false}
                onCheckedChange={(checked) =>
                  setFormData(prev => ({ ...prev, auto_generate_wiki: checked }))
                }
              />
            </div>

            <Button
              onClick={handleAddRepository}
              disabled={isAdding || !formData.repository.trim()}
              className="w-full"
            >
              {isAdding ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin mr-2" />
                  Starting Indexing...
                </>
              ) : (
                <>
                  <Plus className="h-4 w-4 mr-2" />
                  Add Repository
                </>
              )}
            </Button>

            <p className="text-xs text-muted-foreground text-center mt-2">
              💡 You can add multiple repositories. Indexing runs in the background.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Progress Panel */}
      <ProgressPanel className="mb-6" />

      {/* Repositories List */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-semibold">Your Repositories</h2>
          <div className="flex items-center gap-2">
            {hasError && (
              <Badge variant="destructive" className="text-xs">
                <AlertCircle className="h-3 w-3 mr-1" />
                Error
              </Badge>
            )}
            <Badge variant="secondary">{repositories?.length || 0} repositories</Badge>
          </div>
        </div>

        {hasError && (
          <Card className="border-destructive">
            <CardContent className="pt-6">
              <div className="flex items-center gap-2 text-destructive">
                <AlertCircle className="h-4 w-4" />
                <span className="text-sm font-medium">Failed to load repositories</span>
              </div>
              <p className="text-sm text-muted-foreground mt-1">{errors.repositories}</p>
              <Button
                size="sm"
                variant="outline"
                onClick={() => refetch()}
                className="mt-3"
              >
                <RefreshCw className="h-4 w-4 mr-2" />
                Retry
              </Button>
            </CardContent>
          </Card>
        )}

        {isLoadingRepos && (
          <Card>
            <CardContent className="flex flex-col items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mb-4" />
              <p className="text-muted-foreground">Loading repositories...</p>
            </CardContent>
          </Card>
        )}

        {!repositories || repositories.length === 0 ? (
          <Card className="shadow-soft">
            <CardContent className="flex flex-col items-center justify-center py-12 text-center">
              <Folder className="h-12 w-12 text-muted-foreground mb-4" />
              <h3 className="text-lg font-medium mb-2">No repositories yet</h3>
              <p className="text-muted-foreground mb-4">
                Add your first repository to start exploring your codebase with AI
              </p>
            </CardContent>
          </Card>
        ) : (
          <div className="grid gap-4">
            {repositories?.map((repo) => (
              <Card key={repo.id} className="shadow-soft hover:shadow-medium transition-smooth">
                <CardContent className="p-6">
                  {/* 主要信息行 */}
                  <div className="flex items-start justify-between mb-4">
                    <div className="flex items-start gap-4 flex-1">
                      <div className="flex-shrink-0 mt-1">
                        {getRepoTypeIcon(repo.repo_type)}
                      </div>

                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-2">
                          <h3 className="font-semibold text-lg truncate">{repo.name}</h3>
                          <Badge variant={repo.repo_type === 'github' ? 'default' : 'secondary'}>
                            {repo.repo_type === 'github' ? 'GitHub' : repo.repo_type === 'git' ? 'Git' : 'Local'}
                          </Badge>
                        </div>

                        <p className="text-sm text-muted-foreground truncate mb-2">
                          {repo.repo_path}
                        </p>

                        {repo.description && (
                          <p className="text-xs text-muted-foreground mb-3">
                            {repo.description}
                          </p>
                        )}

                        <div className="flex items-center gap-4 text-sm">
                          <div className="flex items-center gap-1">
                            {getStatusIcon(repo.status)}
                            <span className="font-medium">Status:</span>
                            <span>{getStatusText(repo)}</span>
                          </div>

                          <Separator orientation="vertical" className="h-4" />
                          <span className="text-muted-foreground">
                            Added: {new Date(repo.created_at).toLocaleDateString()}
                          </span>

                          {repo.last_indexed_at && (
                            <>
                              <Separator orientation="vertical" className="h-4" />
                              <span className="text-muted-foreground">
                                Indexed: {new Date(repo.last_indexed_at).toLocaleDateString()}
                              </span>
                            </>
                          )}
                        </div>
                      </div>
                    </div>

                    {/* 右侧按钮区域 */}
                    <div className="flex items-center gap-2 ml-4">
                      <Button
                        size="sm"
                        className="flex items-center gap-1"
                        onClick={() => handleStartChat(repo)}
                        disabled={repo.status !== 'indexed'}
                      >
                        <MessageCircle className="h-4 w-4" />
                        Chat
                      </Button>

                      <Button
                        size="sm"
                        variant="outline"
                        className="flex items-center gap-1"
                        onClick={() => handleStartResearch(repo)}
                        disabled={repo.status !== 'indexed'}
                      >
                        <Brain className="h-4 w-4" />
                        Research
                      </Button>

                      <Button
                        size="sm"
                        variant="outline"
                        className="flex items-center gap-1"
                        onClick={() => handleStartWiki(repo)}
                        disabled={
                          repo.status !== 'indexed' ||
                          repo.wiki_status === 'generating'
                        }
                      >
                        <BookOpen className="h-4 w-4" />
                        {repo.wiki_status === 'generating' ? 'Generating...' :
                         repo.wiki_status === 'generated' ? 'View Wiki' :
                         repo.wiki_status === 'failed' ? 'View Wiki' :
                         'View Wiki'}
                      </Button>

                      <Button
                        size="sm"
                        variant="outline"
                        className="flex items-center gap-1"
                        onClick={() => handleGenerateWiki(repo)}
                        disabled={repo.status !== 'indexed'}
                      >
                        <Plus className="h-4 w-4" />
                        Generate
                      </Button>

                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleRefreshRepository(repo)}
                        disabled={repo.status === 'indexing' || reindexingRepos.has(repo.id)}
                        title={
                          repo.status === 'indexing'
                            ? "Repository is currently being indexed"
                            : reindexingRepos.has(repo.id)
                            ? "Starting reindex..."
                            : repo.status === 'indexed'
                            ? "Reindex repository (will require confirmation)"
                            : "Start indexing repository"
                        }
                      >
                        <RefreshCw className={`h-4 w-4 ${reindexingRepos.has(repo.id) ? 'animate-spin' : ''}`} />
                      </Button>

                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleRemoveRepository(repo)}
                        disabled={deleteRepositoryMutation.isPending}
                      >
                        {deleteRepositoryMutation.isPending ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <Trash2 className="h-4 w-4" />
                        )}
                      </Button>
                    </div>
                  </div>

                  {/* 索引进度显示 - 独立区域 */}
                  {repo.status === 'indexing' && (
                    <div className="mt-4 pt-4 border-t">
                      <UniversalProgress
                        repositoryId={repo.id}
                        type="indexing"
                        config={{
                          variant: "inline",
                          showDetails: true,
                          showTimeEstimate: true,
                          showCancelButton: true,
                        }}
                        callbacks={{
                          onComplete: () => {
                            // 刷新仓库列表
                            refetch();
                          },
                          onError: () => {
                            console.error('Indexing error for repository:', repo.id);
                            // 刷新仓库列表以更新状态
                            refetch();
                          },
                        }}
                      />
                    </div>
                  )}

                  {/* Wiki 生成进度显示 - 独立区域 */}
                  {repo.wiki_status === 'generating' && (
                    <div className="mt-4 pt-4 border-t">
                      <UniversalProgress
                        repositoryId={repo.id}
                        type="wiki_generation"
                        config={{
                          variant: "inline",
                          showDetails: true,
                          showTimeEstimate: true,
                          showCancelButton: true,
                        }}
                        callbacks={{
                          onComplete: () => {
                            // 刷新仓库列表
                            refetch();
                          },
                          onError: () => {
                            console.error('Wiki generation error for repository:', repo.id);
                            // 刷新仓库列表以更新状态
                            refetch();
                          },
                        }}
                      />
                    </div>
                  )}
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>

      {/* Wiki Generation Dialog */}
      {selectedRepoForWiki && (
        <WikiGenerationDialog
          open={showWikiDialog}
          onOpenChange={setShowWikiDialog}
          sessionId={selectedRepoForWiki.id}
          repositoryName={selectedRepoForWiki.name}
        />
      )}
    </div>
  );
};

export default RepositoryManager;
