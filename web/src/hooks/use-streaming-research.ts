/**
 * 流式深度研究 Hook
 * 使用 Server-Sent Events (SSE) 实现实时研究进度更新
 */

import { useState, useEffect, useCallback, useRef } from "react";
import { useToast } from "@/hooks/use-toast";
import { apiClient } from "@/lib/api-client";

interface StreamingResearchState {
  researchId: string | null;
  status:
    | "idle"
    | "starting"
    | "researching"
    | "completed"
    | "failed"
    | "cancelled";
  originalQuery: string;
  currentIteration: number;
  maxIterations: number;
  progress: number;
  currentResponse: string | null;
  lastUpdated: string | null;
  finalResult: any | null;
  error: string | null;
}

interface ResearchProgressEvent {
  type: "progress" | "complete" | "error";
  research_id: string;
  original_query: string;
  status: string;
  current_iteration: number;
  max_iterations: number;
  progress: number;
  current_response?: string;
  last_updated: string;
  timestamp: string;
  final_result?: any;
  error?: string;
  message?: string;
}

interface StreamingResearchRequest {
  repository_id: string;
  research_question: string;
  config?: {
    max_iterations?: number;
    max_sources_per_iteration?: number;
  };
}

const initialState: StreamingResearchState = {
  researchId: null,
  status: "idle",
  originalQuery: "",
  currentIteration: 0,
  maxIterations: 5,
  progress: 0,
  currentResponse: null,
  lastUpdated: null,
  finalResult: null,
  error: null,
};

export const useStreamingResearch = () => {
  const [state, setState] = useState<StreamingResearchState>(initialState);
  const { toast } = useToast();
  const readerRef = useRef<ReadableStreamDefaultReader<Uint8Array> | null>(
    null
  );
  const abortControllerRef = useRef<AbortController | null>(null);

  const cleanup = useCallback(() => {
    if (readerRef.current) {
      readerRef.current.cancel();
      readerRef.current = null;
    }
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
  }, []);

  const handleSSEEvent = useCallback(
    (data: ResearchProgressEvent) => {
      switch (data.type) {
        case "progress":
          setState((prev) => ({
            ...prev,
            researchId: data.research_id,
            status: "researching",
            currentIteration: data.current_iteration,
            maxIterations: data.max_iterations,
            progress: data.progress,
            currentResponse: data.current_response || prev.currentResponse,
            lastUpdated: data.last_updated,
          }));
          break;

        case "complete":
          setState((prev) => ({
            ...prev,
            status: "completed",
            finalResult: data.final_result,
            progress: 1,
          }));

          toast({
            title: "研究完成",
            description: data.message || "深度研究已成功完成",
          });

          cleanup();
          break;

        case "error":
          setState((prev) => ({
            ...prev,
            status: "failed",
            error: data.error || "Unknown error occurred",
          }));

          toast({
            title: "研究失败",
            description: data.error || "研究过程中发生错误",
            variant: "destructive",
          });

          cleanup();
          break;

        default:
          console.warn("Unknown SSE event type:", data.type);
      }
    },
    [toast, cleanup]
  );

  const startStreamingResearch = useCallback(
    async (
      repositoryId: string,
      researchQuestion: string,
      config?: { max_iterations?: number; max_sources_per_iteration?: number }
    ) => {
      try {
        cleanup();

        setState((prev) => ({
          ...prev,
          status: "starting",
          originalQuery: researchQuestion,
          error: null,
        }));

        abortControllerRef.current = new AbortController();

        const requestData: StreamingResearchRequest = {
          repository_id: repositoryId,
          research_question: researchQuestion,
          config,
        };

        const response = await fetch(
          `${apiClient.defaults.baseURL}/research/deep-stream`,
          {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              Accept: "text/event-stream",
              "Cache-Control": "no-cache",
              ...apiClient.defaults.headers,
            },
            body: JSON.stringify(requestData),
            signal: abortControllerRef.current.signal,
          }
        );

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        if (!response.body) {
          throw new Error("Response body is null");
        }

        const reader = response.body.getReader();
        readerRef.current = reader;

        const decoder = new TextDecoder();
        let buffer = "";

        const processStream = async () => {
          try {
            while (true) {
              const { done, value } = await reader.read();
              if (done) break;

              buffer += decoder.decode(value, { stream: true });
              const lines = buffer.split("\n");
              buffer = lines.pop() || "";

              for (const line of lines) {
                if (line.startsWith("data: ")) {
                  try {
                    const data = JSON.parse(line.slice(6));
                    handleSSEEvent(data);
                  } catch (error) {
                    console.error("Failed to parse SSE data:", error);
                  }
                }
              }
            }
          } catch (error) {
            if (error instanceof Error && error.name !== "AbortError") {
              console.error("Stream processing error:", error);
              setState((prev) => ({
                ...prev,
                status: "failed",
                error: error.message,
              }));
            }
          }
        };

        processStream();
      } catch (error) {
        console.error("Failed to start streaming research:", error);
        setState((prev) => ({
          ...prev,
          status: "failed",
          error: error instanceof Error ? error.message : "启动研究失败",
        }));

        toast({
          title: "启动失败",
          description: error instanceof Error ? error.message : "无法启动研究",
          variant: "destructive",
        });

        cleanup();
      }
    },
    [cleanup, toast, handleSSEEvent]
  );

  const stopResearch = useCallback(() => {
    setState((prev) => ({ ...prev, status: "cancelled" }));
    cleanup();
    toast({ title: "研究已停止", description: "研究已被用户取消" });
  }, [cleanup, toast]);

  const resetResearch = useCallback(() => {
    cleanup();
    setState(initialState);
  }, [cleanup]);

  useEffect(() => cleanup, [cleanup]);

  return {
    ...state,
    isConnected: readerRef.current !== null,
    startStreamingResearch,
    stopResearch,
    resetResearch,
    progressPercentage: Math.round(state.progress * 100),
    isActive: state.status === "researching" || state.status === "starting",
    isComplete: state.status === "completed",
    isFailed: state.status === "failed",
    isCancelled: state.status === "cancelled",
  };
};
