/**
 * 服务器状态栏组件
 * 在页面底部显示简洁的服务器连接状态
 */

import React, { useState, useEffect } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { 
  Server, 
  Wifi, 
  WifiOff, 
  AlertCircle, 
  CheckCircle2,
  Settings,
  RefreshCw
} from 'lucide-react';
import { BackendConnectionSettings } from '@/components/BackendConnectionSettings';
import { useHealthCheck } from '@/hooks/use-api';
import { backendConnection, BackendEndpoint } from '@/lib/backend-connection';
import { cn } from '@/lib/utils';

interface ServerStatusBarProps {
  onConnectionChange?: (endpoint: BackendEndpoint | null) => void;
  className?: string;
}

export function ServerStatusBar({ onConnectionChange, className }: ServerStatusBarProps) {
  const [currentEndpoint, setCurrentEndpoint] = useState<BackendEndpoint | null>(null);
  const [isManualChecking, setIsManualChecking] = useState(false);
  
  // 使用健康检查 Hook
  const { 
    data: healthData, 
    isLoading: isHealthLoading, 
    error: healthError, 
    refetch: refetchHealth 
  } = useHealthCheck();

  // 获取当前端点信息
  useEffect(() => {
    const getCurrentEndpoint = async () => {
      const endpoint = backendConnection.getCurrentEndpoint();
      setCurrentEndpoint(endpoint);
    };
    getCurrentEndpoint();
  }, []);

  const handleManualCheck = async () => {
    setIsManualChecking(true);
    try {
      await refetchHealth();
    } finally {
      setIsManualChecking(false);
    }
  };

  const getConnectionStatus = () => {
    if (isHealthLoading || isManualChecking) {
      return {
        status: 'checking' as const,
        message: 'Checking connection...',
        color: 'secondary' as const,
        icon: RefreshCw,
        iconClass: 'text-blue-500 animate-spin'
      };
    }

    if (healthData) {
      return {
        status: 'connected' as const,
        message: `Connected to ${currentEndpoint?.name || 'Backend'}`,
        color: 'default' as const,
        icon: CheckCircle2,
        iconClass: 'text-green-500'
      };
    }

    if (healthError) {
      return {
        status: 'error' as const,
        message: 'Connection failed',
        color: 'destructive' as const,
        icon: AlertCircle,
        iconClass: 'text-red-500'
      };
    }

    return {
      status: 'unknown' as const,
      message: 'Connection status unknown',
      color: 'secondary' as const,
      icon: WifiOff,
      iconClass: 'text-muted-foreground'
    };
  };

  const connectionStatus = getConnectionStatus();
  const StatusIcon = connectionStatus.icon;

  return (
    <div className={cn(
      "fixed bottom-0 left-0 right-0 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-t z-50",
      className
    )}>
      <div className="container mx-auto px-4 py-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <StatusIcon className={cn("h-4 w-4", connectionStatus.iconClass)} />
              <span className="text-sm text-muted-foreground">
                {currentEndpoint?.name || 'Backend'}
              </span>
              <Badge variant={connectionStatus.color} className="text-xs">
                {connectionStatus.status === 'connected' ? 'Online' : 
                 connectionStatus.status === 'error' ? 'Offline' : 
                 connectionStatus.status === 'checking' ? 'Checking' : 'Unknown'}
              </Badge>
            </div>
            
            {connectionStatus.status === 'error' && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleManualCheck}
                disabled={isManualChecking}
                className="h-6 px-2 text-xs"
              >
                <RefreshCw className={cn("h-3 w-3 mr-1", isManualChecking && "animate-spin")} />
                Retry
              </Button>
            )}
          </div>

          <div className="flex items-center gap-2">
            {/* 服务器设置下拉菜单 */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" className="h-6 px-2">
                  <Server size={12} className="mr-1" />
                  <Settings size={10} />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" side="top" className="w-96 p-0 mb-2">
                <BackendConnectionSettings onConnectionChange={onConnectionChange} compact />
              </DropdownMenuContent>
            </DropdownMenu>

            {/* 当前服务器信息 */}
            {currentEndpoint && (
              <span className="text-xs text-muted-foreground">
                {currentEndpoint.apiUrl.replace('http://', '').replace('/api', '')}
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
