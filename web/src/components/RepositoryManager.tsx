import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { useToast } from "@/hooks/use-toast";
import {
  FolderOpen,
  Github,
  Plus,
  MessageCircle,
  RefreshCw,
  Trash2,
  CheckCircle,
  Loader2,
  Globe,
  Folder,
  AlertCircle
} from "lucide-react";

// API hooks
import {
  useRepositories,
  useAddRepository,
  useDeleteRepository,
  useCreateSession
} from "@/hooks/use-api";

// Store hooks
import {
  useRepositories as useRepositoriesStore,
  useLoadingState,
  useErrors
} from "@/store/app-store";

// Types
import { Repository, AddRepositoryRequest } from "@/types/api";
import { AddRepositoryFormData } from "@/types/ui";

const RepositoryManager = () => {
  const navigate = useNavigate();
  const { toast } = useToast();

  // Form state
  const [formData, setFormData] = useState<AddRepositoryFormData>({
    repo_path: '',
    repo_type: 'remote',
    name: '',
    description: '',
  });

  // API hooks
  const { data: repositoriesData, isLoading: isLoadingRepos, refetch } = useRepositories();
  const addRepositoryMutation = useAddRepository();
  const deleteRepositoryMutation = useDeleteRepository();
  const createSessionMutation = useCreateSession();

  // Store state
  const repositories = useRepositoriesStore();
  const loadingState = useLoadingState();
  const errors = useErrors();

  // Derived state
  const isAdding = addRepositoryMutation.isPending;
  const hasError = !!errors.repositories;

  const handleAddRepository = async () => {
    if (!formData.repo_path.trim()) {
      toast({
        title: "Path Required",
        description: "Please enter a repository URL or path",
        variant: "destructive"
      });
      return;
    }

    const requestData: AddRepositoryRequest = {
      repo_path: formData.repo_path,
      repo_type: formData.repo_type === 'remote' ? 'github' : 'local',
      name: formData.name || undefined,
      description: formData.description || undefined,
    };

    try {
      await addRepositoryMutation.mutateAsync(requestData);

      // Reset form
      setFormData({
        repo_path: '',
        repo_type: 'remote',
        name: '',
        description: '',
      });

      // Refresh repositories list
      refetch();
    } catch (error) {
      // Error is handled by the mutation's onError callback
      console.error('Failed to add repository:', error);
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
      const session = await createSessionMutation.mutateAsync({
        repositoryId: repository.id,
        name: `Chat with ${repository.name}`,
      });

      // Navigate to chat interface
      navigate(`/chat/${session.id}`);
    } catch (error) {
      console.error('Failed to create session:', error);
    }
  };

  const handleRefreshRepository = async (repository: Repository) => {
    // TODO: Implement repository refresh/re-indexing
    toast({
      title: "Refresh Repository",
      description: "Repository refresh functionality will be implemented soon.",
    });
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
        return <Github className="h-5 w-5 text-primary" />;
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
          <FolderOpen className="h-8 w-8 text-primary" />
          <h1 className="text-3xl font-bold text-foreground">Wikify</h1>
        </div>
        <p className="text-muted-foreground max-w-2xl mx-auto">
          AI-powered code repository documentation and Q&A system. 
          Add your repositories and start asking questions about your codebase.
        </p>
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
              <Github className="h-4 w-4" />
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
            <Input
              placeholder={
                formData.repo_type === 'remote'
                  ? "https://github.com/user/repository"
                  : "/path/to/your/project"
              }
              value={formData.repo_path}
              onChange={(e) => setFormData(prev => ({ ...prev, repo_path: e.target.value }))}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  handleAddRepository();
                }
              }}
            />

            <Input
              placeholder="Repository name (optional)"
              value={formData.name}
              onChange={(e) => setFormData(prev => ({ ...prev, name: e.target.value }))}
            />

            <Input
              placeholder="Description (optional)"
              value={formData.description}
              onChange={(e) => setFormData(prev => ({ ...prev, description: e.target.value }))}
            />

            <Button
              onClick={handleAddRepository}
              disabled={isAdding || !formData.repo_path.trim()}
              className="w-full"
            >
              {isAdding ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin mr-2" />
                  Adding Repository...
                </>
              ) : (
                <>
                  <Plus className="h-4 w-4 mr-2" />
                  Add Repository
                </>
              )}
            </Button>
          </div>
        </CardContent>
      </Card>

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
            <Badge variant="secondary">{repositories.length} repositories</Badge>
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

        {repositories.length === 0 ? (
          <Card className="shadow-soft">
            <CardContent className="flex flex-col items-center justify-center py-12 text-center">
              <FolderOpen className="h-12 w-12 text-muted-foreground mb-4" />
              <h3 className="text-lg font-medium mb-2">No repositories yet</h3>
              <p className="text-muted-foreground mb-4">
                Add your first repository to start exploring your codebase with AI
              </p>
            </CardContent>
          </Card>
        ) : (
          <div className="grid gap-4">
            {repositories.map((repo) => (
              <Card key={repo.id} className="shadow-soft hover:shadow-medium transition-smooth">
                <CardContent className="p-6">
                  <div className="flex items-start justify-between">
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

                        <p className="text-sm text-muted-foreground truncate mb-3">
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
                    
                    <div className="flex items-center gap-2 ml-4">
                      <Button
                        size="sm"
                        className="flex items-center gap-1"
                        onClick={() => handleStartChat(repo)}
                        disabled={repo.status !== 'indexed' || createSessionMutation.isPending}
                      >
                        {createSessionMutation.isPending ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <MessageCircle className="h-4 w-4" />
                        )}
                        Chat
                      </Button>

                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleRefreshRepository(repo)}
                        disabled={repo.status === 'indexing'}
                      >
                        <RefreshCw className="h-4 w-4" />
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
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default RepositoryManager;
