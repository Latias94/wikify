/**
 * Backend Connection Manager
 * Handles automatic backend discovery and fallback connections
 */

export interface BackendEndpoint {
  id: string;
  name: string;
  apiUrl: string;
  wsUrl: string;
  isDefault?: boolean;
}

export interface ConnectionStatus {
  endpoint: BackendEndpoint;
  isConnected: boolean;
  lastChecked: Date;
  responseTime?: number;
  error?: string;
}

class BackendConnectionManager {
  private static instance: BackendConnectionManager;
  private endpoints: BackendEndpoint[] = [];
  private currentEndpoint: BackendEndpoint | null = null;
  private connectionStatus: Map<string, ConnectionStatus> = new Map();
  private checkInterval: NodeJS.Timeout | null = null;

  private constructor() {
    this.initializeDefaultEndpoints();
    this.loadUserEndpoints();
  }

  static getInstance(): BackendConnectionManager {
    if (!BackendConnectionManager.instance) {
      BackendConnectionManager.instance = new BackendConnectionManager();
    }
    return BackendConnectionManager.instance;
  }

  private initializeDefaultEndpoints() {
    // In development, use Vite proxy to avoid CORS issues
    const isDevelopment = import.meta.env.DEV;

    if (isDevelopment) {
      this.endpoints = [
        {
          id: "vite-proxy",
          name: "Development (Vite Proxy)",
          apiUrl: "/api",
          wsUrl: "/ws",
          isDefault: true,
        },
        {
          id: "localhost-direct",
          name: "Localhost (Direct)",
          apiUrl: "http://localhost:8080/api",
          wsUrl: "ws://localhost:8080/ws",
        },
        {
          id: "127.0.0.1",
          name: "127.0.0.1 (Fallback)",
          apiUrl: "http://127.0.0.1:8080/api",
          wsUrl: "ws://127.0.0.1:8080/ws",
        },
      ];
    } else {
      // In production, use direct connections
      this.endpoints = [
        {
          id: "localhost",
          name: "Localhost (Default)",
          apiUrl: "http://localhost:8080/api",
          wsUrl: "ws://localhost:8080/ws",
          isDefault: true,
        },
        {
          id: "127.0.0.1",
          name: "127.0.0.1 (Fallback)",
          apiUrl: "http://127.0.0.1:8080/api",
          wsUrl: "ws://127.0.0.1:8080/ws",
        },
        {
          id: "all-interfaces",
          name: "0.0.0.0 (All Interfaces)",
          apiUrl: "http://0.0.0.0:8080/api",
          wsUrl: "ws://0.0.0.0:8080/ws",
        },
      ];
    }
  }

  private loadUserEndpoints() {
    try {
      const saved = localStorage.getItem("wikify-backend-endpoints");
      if (saved) {
        const userEndpoints = JSON.parse(saved) as BackendEndpoint[];
        this.endpoints.push(...userEndpoints);
      }
    } catch (error) {
      console.warn("Failed to load user endpoints:", error);
    }
  }

  private saveUserEndpoints() {
    try {
      const userEndpoints = this.endpoints.filter((e) => !e.isDefault);
      localStorage.setItem(
        "wikify-backend-endpoints",
        JSON.stringify(userEndpoints)
      );
    } catch (error) {
      console.warn("Failed to save user endpoints:", error);
    }
  }

  async checkEndpoint(endpoint: BackendEndpoint): Promise<ConnectionStatus> {
    const startTime = Date.now();
    const status: ConnectionStatus = {
      endpoint,
      isConnected: false,
      lastChecked: new Date(),
    };

    try {
      // Try to connect to the health check endpoint
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);

      const response = await fetch(`${endpoint.apiUrl}/health`, {
        method: "GET",
        signal: controller.signal,
        headers: {
          "Content-Type": "application/json",
        },
      });

      clearTimeout(timeoutId);

      if (response.ok) {
        status.isConnected = true;
        status.responseTime = Date.now() - startTime;
      } else {
        status.error = `HTTP ${response.status}: ${response.statusText}`;
      }
    } catch (error) {
      if (error instanceof Error) {
        if (error.name === "AbortError") {
          status.error = "Connection timeout";
        } else {
          status.error = error.message;
        }
      } else {
        status.error = "Unknown connection error";
      }
    }

    this.connectionStatus.set(endpoint.id, status);
    return status;
  }

  async findAvailableEndpoint(): Promise<BackendEndpoint | null> {
    // Check current endpoint first if it exists
    if (this.currentEndpoint) {
      const status = await this.checkEndpoint(this.currentEndpoint);
      if (status.isConnected) {
        return this.currentEndpoint;
      }
    }

    // Try all endpoints in order
    for (const endpoint of this.endpoints) {
      const status = await this.checkEndpoint(endpoint);
      if (status.isConnected) {
        this.currentEndpoint = endpoint;
        this.saveCurrentEndpoint();
        return endpoint;
      }
    }

    return null;
  }

  async setCurrentEndpoint(endpoint: BackendEndpoint): Promise<boolean> {
    const status = await this.checkEndpoint(endpoint);
    if (status.isConnected) {
      this.currentEndpoint = endpoint;
      this.saveCurrentEndpoint();
      return true;
    }
    return false;
  }

  getCurrentEndpoint(): BackendEndpoint | null {
    return this.currentEndpoint;
  }

  getAllEndpoints(): BackendEndpoint[] {
    return [...this.endpoints];
  }

  getConnectionStatus(endpointId: string): ConnectionStatus | null {
    return this.connectionStatus.get(endpointId) || null;
  }

  getAllConnectionStatuses(): ConnectionStatus[] {
    return Array.from(this.connectionStatus.values());
  }

  addCustomEndpoint(endpoint: Omit<BackendEndpoint, "id">): BackendEndpoint {
    const newEndpoint: BackendEndpoint = {
      ...endpoint,
      id: `custom-${Date.now()}`,
    };

    this.endpoints.push(newEndpoint);
    this.saveUserEndpoints();
    return newEndpoint;
  }

  removeEndpoint(endpointId: string): boolean {
    const index = this.endpoints.findIndex((e) => e.id === endpointId);
    if (index !== -1 && !this.endpoints[index].isDefault) {
      this.endpoints.splice(index, 1);
      this.connectionStatus.delete(endpointId);
      this.saveUserEndpoints();

      // If this was the current endpoint, clear it
      if (this.currentEndpoint?.id === endpointId) {
        this.currentEndpoint = null;
        this.clearCurrentEndpoint();
      }

      return true;
    }
    return false;
  }

  startHealthCheck(intervalMs: number = 30000) {
    this.stopHealthCheck();
    this.checkInterval = setInterval(async () => {
      // Check current endpoint
      if (this.currentEndpoint) {
        await this.checkEndpoint(this.currentEndpoint);
      }
    }, intervalMs);
  }

  stopHealthCheck() {
    if (this.checkInterval) {
      clearInterval(this.checkInterval);
      this.checkInterval = null;
    }
  }

  private saveCurrentEndpoint() {
    if (this.currentEndpoint) {
      localStorage.setItem("wikify-current-endpoint", this.currentEndpoint.id);
    }
  }

  private clearCurrentEndpoint() {
    localStorage.removeItem("wikify-current-endpoint");
  }

  private loadCurrentEndpoint() {
    try {
      const savedId = localStorage.getItem("wikify-current-endpoint");
      if (savedId) {
        const endpoint = this.endpoints.find((e) => e.id === savedId);
        if (endpoint) {
          this.currentEndpoint = endpoint;
        }
      }
    } catch (error) {
      console.warn("Failed to load current endpoint:", error);
    }
  }

  // Initialize the connection manager
  async initialize(): Promise<BackendEndpoint | null> {
    this.loadCurrentEndpoint();
    const endpoint = await this.findAvailableEndpoint();
    if (endpoint) {
      this.startHealthCheck();
    }
    return endpoint;
  }
}

export const backendConnection = BackendConnectionManager.getInstance();
