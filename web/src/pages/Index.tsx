import { useState, useEffect } from "react";
import RepositoryManager from "@/components/RepositoryManager";
import ThemeToggle from "@/components/ThemeToggle";
import { BackendConnectionSettings } from "@/components/BackendConnectionSettings";
import BackendConnectionStatus from "@/components/BackendConnectionStatus";
import { ServerStatusBar } from "@/components/ServerStatusBar";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Settings, Server } from "lucide-react";
import { backendConnection, BackendEndpoint } from "@/lib/backend-connection";
import { apiClient } from "@/lib/api-client";

const Index = () => {
  const [currentBackend, setCurrentBackend] = useState<BackendEndpoint | null>(null);
  const [statusBarMode, setStatusBarMode] = useState<'header' | 'bottom' | 'hidden'>('header'); // 状态栏模式

  useEffect(() => {
    // Initialize backend connection on app start
    const initializeBackend = async () => {
      const endpoint = await backendConnection.initialize();
      setCurrentBackend(endpoint);

      // Update API client with the current endpoint
      if (endpoint) {
        apiClient.updateBaseURL(endpoint.apiUrl);
      }
    };

    initializeBackend();
  }, []);

  const handleConnectionChange = (endpoint: BackendEndpoint | null) => {
    setCurrentBackend(endpoint);
    if (endpoint) {
      apiClient.updateBaseURL(endpoint.apiUrl);
    }
  };

  return (
    <div className="min-h-screen">
      {/* Header with controls */}
      <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-14 items-center justify-between">
          <div className="flex items-center gap-2">
            <h1 className="text-lg font-semibold">Wikify</h1>
            {currentBackend && (
              <span className="text-xs text-muted-foreground">
                → {currentBackend.name}
              </span>
            )}
          </div>
          <div className="flex items-center gap-4">
            {/* 后端连接状态 */}
            <BackendConnectionStatus compact />

            <div className="flex items-center gap-2">
              {/* Server Settings Dropdown */}
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="sm">
                    <Server size={16} className="mr-1" />
                    <Settings size={14} />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-96 p-0">
                  <BackendConnectionSettings onConnectionChange={handleConnectionChange} compact />
                </DropdownMenuContent>
              </DropdownMenu>

              <ThemeToggle />
            </div>
          </div>
        </div>
      </header>

      {/* Main content */}
      <main className="py-8">
        <div className="container space-y-6">
          <RepositoryManager />
        </div>
      </main>
    </div>
  );
};

export default Index;
