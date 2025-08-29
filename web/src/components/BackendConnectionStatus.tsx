/**
 * 后端连接状态指示器
 * 显示后端服务器的连接状态和健康检查信息
 */

import React, { useState, useEffect } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { 
  AlertCircle, 
  CheckCircle2, 
  Loader2, 
  RefreshCw, 
  Server,
  Wifi,
  WifiOff,
  Settings
} from 'lucide-react';
import { useHealthCheck } from '@/hooks/use-api';
import { backendConnection, BackendEndpoint } from '@/lib/backend-connection';
import { cn } from '@/lib/utils';

interface BackendConnectionStatusProps {
  onOpenSettings?: () => void;
  className?: string;
  compact?: boolean;
}

export function BackendConnectionStatus({
  onOpenSettings,
  className,
  compact = false,
}: BackendConnectionStatusProps) {
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

  // 手动检查连接
  const handleManualCheck = async () => {
    setIsManualChecking(true);
    try {
      await refetchHealth();
      if (currentEndpoint) {
        await backendConnection.checkEndpoint(currentEndpoint);
      }
    } catch (error) {
      console.error('Manual health check failed:', error);
    } finally {
      setIsManualChecking(false);
    }
  };

  // 获取连接状态
  const getConnectionStatus = () => {
    if (isHealthLoading || isManualChecking) {
      return {
        status: 'checking' as const,
        message: 'Checking connection...',
        color: 'default' as const,
        icon: Loader2,
        iconClass: 'animate-spin'
      };
    }

    if (healthError) {
      return {
        status: 'error' as const,
        message: 'Backend disconnected',
        color: 'destructive' as const,
        icon: WifiOff,
        iconClass: 'text-destructive'
      };
    }

    if (healthData) {
      return {
        status: 'connected' as const,
        message: 'Backend connected',
        color: 'default' as const,
        icon: Wifi,
        iconClass: 'text-green-500'
      };
    }

    return {
      status: 'unknown' as const,
      message: 'Connection status unknown',
      color: 'secondary' as const,
      icon: AlertCircle,
      iconClass: 'text-muted-foreground'
    };
  };

  const connectionStatus = getConnectionStatus();
  const StatusIcon = connectionStatus.icon;

  // 紧凑模式 - 只显示图标和状态
  if (compact) {
    return (
      <div className={cn("flex items-center gap-2", className)}>
        <StatusIcon className={cn("h-4 w-4", connectionStatus.iconClass)} />
        <Badge variant={connectionStatus.color} className="text-xs">
          {connectionStatus.status === 'connected' ? 'Online' : 
           connectionStatus.status === 'error' ? 'Offline' : 
           connectionStatus.status === 'checking' ? 'Checking' : 'Unknown'}
        </Badge>
        {connectionStatus.status === 'error' && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleManualCheck}
            disabled={isManualChecking}
            className="h-6 w-6 p-0"
          >
            <RefreshCw className={cn("h-3 w-3", isManualChecking && "animate-spin")} />
          </Button>
        )}
      </div>
    );
  }

  // 完整模式 - 显示详细信息
  return (
    <Card className={cn("border-l-4", {
      "border-l-green-500": connectionStatus.status === 'connected',
      "border-l-red-500": connectionStatus.status === 'error',
      "border-l-blue-500": connectionStatus.status === 'checking',
      "border-l-gray-400": connectionStatus.status === 'unknown',
    }, className)}>
      <CardContent className="p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <StatusIcon className={cn("h-5 w-5", connectionStatus.iconClass)} />
            <div>
              <div className="flex items-center gap-2">
                <span className="font-medium">{connectionStatus.message}</span>
                <Badge variant={connectionStatus.color} className="text-xs">
                  {connectionStatus.status.toUpperCase()}
                </Badge>
              </div>
              {currentEndpoint && (
                <div className="text-sm text-muted-foreground mt-1">
                  <Server className="h-3 w-3 inline mr-1" />
                  {currentEndpoint.name} ({currentEndpoint.apiUrl})
                </div>
              )}
              {healthData && (
                <div className="text-xs text-muted-foreground mt-1">
                  Last checked: {new Date(healthData.timestamp).toLocaleTimeString()}
                </div>
              )}
              {healthError && (
                <div className="text-xs text-red-600 mt-1">
                  Error: {healthError.message}
                </div>
              )}
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleManualCheck}
              disabled={isManualChecking}
              className="h-8"
            >
              <RefreshCw className={cn("h-3 w-3 mr-1", isManualChecking && "animate-spin")} />
              Check
            </Button>
            {onOpenSettings && (
              <Button
                variant="outline"
                size="sm"
                onClick={onOpenSettings}
                className="h-8"
              >
                <Settings className="h-3 w-3 mr-1" />
                Settings
              </Button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export default BackendConnectionStatus;
