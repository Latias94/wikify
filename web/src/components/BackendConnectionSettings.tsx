/**
 * Backend Connection Settings Component
 * Allows users to view, test, and manage backend connections
 */

import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { useToast } from '@/hooks/use-toast';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  Server,
  Wifi,
  WifiOff,
  Plus,
  Trash2,
  RefreshCw,
  CheckCircle,
  AlertCircle,
  Settings,
  Zap,
  Clock
} from 'lucide-react';
import { backendConnection, BackendEndpoint, ConnectionStatus } from '@/lib/backend-connection';

interface BackendConnectionSettingsProps {
  onConnectionChange?: (endpoint: BackendEndpoint | null) => void;
  compact?: boolean; // 新增：紧凑模式，适合下拉菜单
}

export function BackendConnectionSettings({ onConnectionChange, compact = false }: BackendConnectionSettingsProps) {
  const { toast } = useToast();
  const [endpoints, setEndpoints] = useState<BackendEndpoint[]>([]);
  const [connectionStatuses, setConnectionStatuses] = useState<ConnectionStatus[]>([]);
  const [currentEndpoint, setCurrentEndpoint] = useState<BackendEndpoint | null>(null);
  const [isChecking, setIsChecking] = useState(false);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  
  // New endpoint form
  const [newEndpoint, setNewEndpoint] = useState({
    name: '',
    host: 'localhost',
    port: '8080',
  });

  // Load initial data
  useEffect(() => {
    loadData();
  }, []);

  const loadData = useCallback(async () => {
    const allEndpoints = backendConnection.getAllEndpoints();
    const current = backendConnection.getCurrentEndpoint();
    const statuses = backendConnection.getAllConnectionStatuses();
    
    setEndpoints(allEndpoints);
    setCurrentEndpoint(current);
    setConnectionStatuses(statuses);
  }, []);

  const checkAllConnections = useCallback(async () => {
    setIsChecking(true);
    try {
      const promises = endpoints.map(endpoint => backendConnection.checkEndpoint(endpoint));
      await Promise.all(promises);
      
      const updatedStatuses = backendConnection.getAllConnectionStatuses();
      setConnectionStatuses(updatedStatuses);
      
      toast({
        title: "Connection Check Complete",
        description: `Checked ${endpoints.length} endpoints`,
      });
    } catch (error) {
      toast({
        title: "Connection Check Failed",
        description: "Failed to check some endpoints",
        variant: "destructive",
      });
    } finally {
      setIsChecking(false);
    }
  }, [endpoints, toast]);

  const switchToEndpoint = useCallback(async (endpoint: BackendEndpoint) => {
    const success = await backendConnection.setCurrentEndpoint(endpoint);
    if (success) {
      setCurrentEndpoint(endpoint);
      onConnectionChange?.(endpoint);
      toast({
        title: "Backend Switched",
        description: `Now using ${endpoint.name}`,
      });
    } else {
      toast({
        title: "Connection Failed",
        description: `Cannot connect to ${endpoint.name}`,
        variant: "destructive",
      });
    }
  }, [onConnectionChange, toast]);

  const addCustomEndpoint = useCallback(() => {
    if (!newEndpoint.name || !newEndpoint.host || !newEndpoint.port) {
      toast({
        title: "Invalid Input",
        description: "Please fill in all fields",
        variant: "destructive",
      });
      return;
    }

    const endpoint = backendConnection.addCustomEndpoint({
      name: newEndpoint.name,
      apiUrl: `http://${newEndpoint.host}:${newEndpoint.port}/api`,
      wsUrl: `ws://${newEndpoint.host}:${newEndpoint.port}/ws`,
    });

    setEndpoints(backendConnection.getAllEndpoints());
    setNewEndpoint({ name: '', host: 'localhost', port: '8080' });
    setIsDialogOpen(false);
    
    toast({
      title: "Endpoint Added",
      description: `Added ${endpoint.name}`,
    });
  }, [newEndpoint, toast]);

  const removeEndpoint = useCallback((endpointId: string) => {
    const success = backendConnection.removeEndpoint(endpointId);
    if (success) {
      setEndpoints(backendConnection.getAllEndpoints());
      toast({
        title: "Endpoint Removed",
        description: "Custom endpoint has been removed",
      });
    }
  }, [toast]);

  const getStatusBadge = (status: ConnectionStatus | undefined) => {
    if (!status) {
      return <Badge variant="secondary">Unknown</Badge>;
    }

    if (status.isConnected) {
      return (
        <Badge variant="default" className="bg-green-500">
          <CheckCircle size={12} className="mr-1" />
          Connected
          {status.responseTime && (
            <span className="ml-1 text-xs">({status.responseTime}ms)</span>
          )}
        </Badge>
      );
    } else {
      return (
        <Badge variant="destructive">
          <AlertCircle size={12} className="mr-1" />
          Failed
        </Badge>
      );
    }
  };

  // 紧凑模式：适合下拉菜单
  if (compact) {
    return (
      <div className="p-4 space-y-4 max-h-96 overflow-y-auto">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Server size={16} />
            <span className="font-medium text-sm">Backend Servers</span>
          </div>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={checkAllConnections}
              disabled={isChecking}
            >
              {isChecking ? (
                <RefreshCw size={12} className="animate-spin" />
              ) : (
                <RefreshCw size={12} />
              )}
            </Button>

            <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
              <DialogTrigger asChild>
                <Button variant="ghost" size="sm">
                  <Plus size={12} />
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Add Custom Backend</DialogTitle>
                  <DialogDescription>
                    Add a custom backend server endpoint
                  </DialogDescription>
                </DialogHeader>
                <div className="space-y-4">
                  <div>
                    <label className="text-sm font-medium">Name</label>
                    <Input
                      placeholder="My Custom Backend"
                      value={newEndpoint.name}
                      onChange={(e) => setNewEndpoint(prev => ({ ...prev, name: e.target.value }))}
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    <div>
                      <label className="text-sm font-medium">Host</label>
                      <Input
                        placeholder="localhost"
                        value={newEndpoint.host}
                        onChange={(e) => setNewEndpoint(prev => ({ ...prev, host: e.target.value }))}
                      />
                    </div>
                    <div>
                      <label className="text-sm font-medium">Port</label>
                      <Input
                        placeholder="8080"
                        value={newEndpoint.port}
                        onChange={(e) => setNewEndpoint(prev => ({ ...prev, port: e.target.value }))}
                      />
                    </div>
                  </div>
                  <Button onClick={addCustomEndpoint} className="w-full">
                    Add Endpoint
                  </Button>
                </div>
              </DialogContent>
            </Dialog>
          </div>
        </div>

        <div className="space-y-2">
          {endpoints.map((endpoint) => {
            const status = connectionStatuses.find(s => s.endpoint.id === endpoint.id);
            const isCurrent = currentEndpoint?.id === endpoint.id;

            return (
              <div
                key={endpoint.id}
                className={`p-2 border rounded text-xs transition-colors ${
                  isCurrent ? 'border-primary bg-primary/5' : 'border-border'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 min-w-0 flex-1">
                    {isCurrent ? (
                      <Zap size={12} className="text-primary flex-shrink-0" />
                    ) : status?.isConnected ? (
                      <Wifi size={12} className="text-green-500 flex-shrink-0" />
                    ) : (
                      <WifiOff size={12} className="text-muted-foreground flex-shrink-0" />
                    )}
                    <div className="min-w-0 flex-1">
                      <div className="font-medium truncate">{endpoint.name}</div>
                      <div className="text-muted-foreground truncate">
                        {endpoint.apiUrl.replace('http://', '').replace('/api', '')}
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center gap-1 flex-shrink-0">
                    {getStatusBadge(status)}

                    {!isCurrent && status?.isConnected && (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => switchToEndpoint(endpoint)}
                        className="h-6 px-2 text-xs"
                      >
                        Switch
                      </Button>
                    )}

                    {!endpoint.isDefault && (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => removeEndpoint(endpoint.id)}
                        className="h-6 w-6 p-0 text-destructive hover:text-destructive"
                      >
                        <Trash2 size={10} />
                      </Button>
                    )}
                  </div>
                </div>

                {status?.error && (
                  <div className="mt-1 text-xs text-destructive">
                    {status.error}
                  </div>
                )}
              </div>
            );
          })}
        </div>

        {currentEndpoint && (
          <div className="pt-2 border-t">
            <div className="text-xs font-medium mb-1">Current: {currentEndpoint.name}</div>
            <div className="text-xs text-muted-foreground">
              {currentEndpoint.apiUrl.replace('http://', '').replace('/api', '')}
            </div>
          </div>
        )}
      </div>
    );
  }

  // 原始的完整模式
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Server size={20} />
              Backend Connection
            </CardTitle>
            <CardDescription>
              Manage and monitor backend server connections
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={checkAllConnections}
              disabled={isChecking}
            >
              {isChecking ? (
                <RefreshCw size={14} className="animate-spin mr-1" />
              ) : (
                <RefreshCw size={14} className="mr-1" />
              )}
              Check All
            </Button>

            <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
              <DialogTrigger asChild>
                <Button variant="outline" size="sm">
                  <Plus size={14} className="mr-1" />
                  Add Custom
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Add Custom Backend</DialogTitle>
                  <DialogDescription>
                    Add a custom backend server endpoint
                  </DialogDescription>
                </DialogHeader>
                <div className="space-y-4">
                  <div>
                    <label className="text-sm font-medium">Name</label>
                    <Input
                      placeholder="My Custom Backend"
                      value={newEndpoint.name}
                      onChange={(e) => setNewEndpoint(prev => ({ ...prev, name: e.target.value }))}
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    <div>
                      <label className="text-sm font-medium">Host</label>
                      <Input
                        placeholder="localhost"
                        value={newEndpoint.host}
                        onChange={(e) => setNewEndpoint(prev => ({ ...prev, host: e.target.value }))}
                      />
                    </div>
                    <div>
                      <label className="text-sm font-medium">Port</label>
                      <Input
                        placeholder="8080"
                        value={newEndpoint.port}
                        onChange={(e) => setNewEndpoint(prev => ({ ...prev, port: e.target.value }))}
                      />
                    </div>
                  </div>
                  <Button onClick={addCustomEndpoint} className="w-full">
                    Add Endpoint
                  </Button>
                </div>
              </DialogContent>
            </Dialog>
          </div>
        </div>
      </CardHeader>
      
      <CardContent>
        <div className="space-y-3">
          <AnimatePresence>
            {endpoints.map((endpoint) => {
              const status = connectionStatuses.find(s => s.endpoint.id === endpoint.id);
              const isCurrent = currentEndpoint?.id === endpoint.id;
              
              return (
                <motion.div
                  key={endpoint.id}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  className={`p-3 border rounded-lg transition-colors ${
                    isCurrent ? 'border-primary bg-primary/5' : 'border-border'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className="flex items-center gap-2">
                        {isCurrent ? (
                          <Zap size={16} className="text-primary" />
                        ) : status?.isConnected ? (
                          <Wifi size={16} className="text-green-500" />
                        ) : (
                          <WifiOff size={16} className="text-muted-foreground" />
                        )}
                        <div>
                          <div className="font-medium text-sm">{endpoint.name}</div>
                          <div className="text-xs text-muted-foreground">
                            {endpoint.apiUrl}
                          </div>
                        </div>
                      </div>
                    </div>
                    
                    <div className="flex items-center gap-2">
                      {getStatusBadge(status)}
                      
                      {status?.lastChecked && (
                        <div className="text-xs text-muted-foreground flex items-center gap-1">
                          <Clock size={10} />
                          {status.lastChecked.toLocaleTimeString()}
                        </div>
                      )}
                      
                      {!isCurrent && status?.isConnected && (
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => switchToEndpoint(endpoint)}
                        >
                          Switch
                        </Button>
                      )}
                      
                      {!endpoint.isDefault && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => removeEndpoint(endpoint.id)}
                          className="text-destructive hover:text-destructive"
                        >
                          <Trash2 size={14} />
                        </Button>
                      )}
                    </div>
                  </div>
                  
                  {status?.error && (
                    <div className="mt-2 text-xs text-destructive">
                      Error: {status.error}
                    </div>
                  )}
                </motion.div>
              );
            })}
          </AnimatePresence>
        </div>
        
        {currentEndpoint && (
          <div className="mt-4 p-3 bg-muted/50 rounded-lg">
            <div className="text-sm font-medium mb-1">Current Backend</div>
            <div className="text-xs text-muted-foreground">
              API: {currentEndpoint.apiUrl}
            </div>
            <div className="text-xs text-muted-foreground">
              WebSocket: {currentEndpoint.wsUrl}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
