/**
 * 进度集成 Hook
 * 将 WebSocket 消息集成到统一的进度管理系统中
 */

import { useCallback, useRef } from "react";
import { useProgressStore } from "@/store/progress-store";
import {
  IndexProgressMessage,
  IndexErrorMessage,
  WikiProgressMessage,
  WikiCompleteMessage,
  WikiErrorMessage,
} from "@/types/websocket";
import {
  IndexingProgressState,
  WikiGenerationProgressState,
} from "@/types/progress";

// ============================================================================
// 进度集成 Hook
// ============================================================================

export function useProgressIntegration() {
  const { startProgress, updateProgress, completeProgress, errorProgress } =
    useProgressStore();

  // 存储活跃的进度 ID - 使用 useRef 确保在组件重新渲染时保持状态
  const progressMapRef = useRef(new Map<string, string>()); // key: type_repositoryId, value: progressId
  const progressMap = progressMapRef.current;

  // ============================================================================
  // 索引进度处理
  // ============================================================================

  const handleIndexProgress = useCallback(
    (message: IndexProgressMessage) => {
      const key = `indexing_${message.repository_id}`;
      let progressId = progressMap.get(key);

      if (!progressId) {
        // 创建新的进度状态
        const initialState: Omit<IndexingProgressState, "id" | "startTime"> = {
          type: "indexing",
          status: "running",
          progress: message.progress,
          repositoryId: message.repository_id,
          currentFile: message.current_file,
          filesProcessed: message.files_processed,
          totalFiles: message.total_files,
          processingRate: message.processing_rate,
        };

        progressId = startProgress(initialState);
        progressMap.set(key, progressId);
      } else {
        // 更新现有进度
        const updates: Partial<IndexingProgressState> = {
          progress: message.progress,
          currentFile: message.current_file,
          filesProcessed: message.files_processed,
          totalFiles: message.total_files,
          processingRate: message.processing_rate,
        };

        updateProgress(progressId, updates);
      }

      // 检查是否完成
      if (message.progress >= 1.0) {
        completeProgress(progressId);
        progressMap.delete(key);
      }
    },
    [startProgress, updateProgress, completeProgress]
  );

  const handleIndexError = useCallback(
    (message: IndexErrorMessage) => {
      const key = `indexing_${message.repository_id}`;
      const progressId = progressMap.get(key);

      if (progressId) {
        errorProgress(progressId, message.error);
        progressMap.delete(key);
      }
    },
    [errorProgress]
  );

  // ============================================================================
  // Wiki 生成进度处理
  // ============================================================================

  const handleWikiProgress = useCallback(
    (message: WikiProgressMessage) => {
      const key = `wiki_generation_${message.repository_id}`;
      let progressId = progressMap.get(key);

      if (!progressId) {
        // 创建新的进度状态
        const initialState: Omit<
          WikiGenerationProgressState,
          "id" | "startTime"
        > = {
          type: "wiki_generation",
          status: "running",
          progress: message.progress,
          repositoryId: message.repository_id,
          currentStep: message.current_step,
          totalSteps: message.total_steps,
          completedSteps: message.completed_steps,
          stepDetails: message.step_details,
        };

        progressId = startProgress(initialState);
        progressMap.set(key, progressId);
      } else {
        // 更新现有进度
        const updates: Partial<WikiGenerationProgressState> = {
          progress: message.progress,
          currentStep: message.current_step,
          totalSteps: message.total_steps,
          completedSteps: message.completed_steps,
          stepDetails: message.step_details,
        };

        updateProgress(progressId, updates);
      }
    },
    [startProgress, updateProgress]
  );

  const handleWikiComplete = useCallback(
    (message: WikiCompleteMessage) => {
      const key = `wiki_generation_${message.repository_id}`;
      const progressId = progressMap.get(key);

      if (progressId) {
        const updates: Partial<WikiGenerationProgressState> = {
          progress: 1.0,
          wikiId: message.wiki_id,
          pagesCount: message.pages_count,
          sectionsCount: message.sections_count,
        };

        updateProgress(progressId, updates);
        completeProgress(progressId);
        progressMap.delete(key);
      }
    },
    [updateProgress, completeProgress]
  );

  const handleWikiError = useCallback(
    (message: WikiErrorMessage) => {
      const key = `wiki_generation_${message.repository_id}`;
      const progressId = progressMap.get(key);

      if (progressId) {
        errorProgress(progressId, message.error);
        progressMap.delete(key);
      }
    },
    [errorProgress]
  );

  // ============================================================================
  // 手动进度管理
  // ============================================================================

  const startManualProgress = useCallback(
    (
      type: "rag_query" | "research",
      repositoryId: string,
      config: any = {}
    ) => {
      const key = `${type}_${repositoryId}`;

      // 如果已经有进度，返回现有的
      const existingProgressId = progressMap.get(key);
      if (existingProgressId) {
        return existingProgressId;
      }

      let initialState: any;

      if (type === "rag_query") {
        initialState = {
          type: "rag_query",
          status: "running",
          progress: 0,
          repositoryId,
          queryId: config.queryId || `query_${Date.now()}`,
          currentPhase: "embedding",
          phaseDetails: "Embedding query...",
          isStreaming: false,
          tokensGenerated: 0,
        };
      } else if (type === "research") {
        initialState = {
          type: "research",
          status: "running",
          progress: 0,
          repositoryId,
          researchId: config.researchId || `research_${Date.now()}`,
          currentStage: "Initializing research...",
          totalStages: config.totalStages || 5,
          completedStages: 0,
          documentsProcessed: 0,
          totalDocuments: config.totalDocuments || 100,
        };
      }

      const progressId = startProgress(initialState);
      progressMap.set(key, progressId);

      return progressId;
    },
    [startProgress]
  );

  const updateManualProgress = useCallback(
    (type: "rag_query" | "research", repositoryId: string, updates: any) => {
      const key = `${type}_${repositoryId}`;
      const progressId = progressMap.get(key);

      if (progressId) {
        updateProgress(progressId, updates);

        // 检查是否完成
        if (updates.progress >= 1.0) {
          completeProgress(progressId);
          progressMap.delete(key);
        }
      }
    },
    [updateProgress, completeProgress]
  );

  const completeManualProgress = useCallback(
    (type: "rag_query" | "research", repositoryId: string) => {
      const key = `${type}_${repositoryId}`;
      const progressId = progressMap.get(key);

      if (progressId) {
        completeProgress(progressId);
        progressMap.delete(key);
      }
    },
    [completeProgress]
  );

  const errorManualProgress = useCallback(
    (type: "rag_query" | "research", repositoryId: string, error: string) => {
      const key = `${type}_${repositoryId}`;
      const progressId = progressMap.get(key);

      if (progressId) {
        errorProgress(progressId, error);
        progressMap.delete(key);
      }
    },
    [errorProgress]
  );

  // ============================================================================
  // 清理函数
  // ============================================================================

  const clearAllProgress = useCallback(() => {
    progressMap.clear();
  }, []);

  return {
    // WebSocket 消息处理
    handleIndexProgress,
    handleIndexError,
    handleWikiProgress,
    handleWikiComplete,
    handleWikiError,

    // 手动进度管理
    startManualProgress,
    updateManualProgress,
    completeManualProgress,
    errorManualProgress,

    // 工具函数
    clearAllProgress,
  };
}

// ============================================================================
// 便捷 Hooks
// ============================================================================

export function useRagQueryProgress(repositoryId: string) {
  const integration = useProgressIntegration();

  const startQuery = useCallback(
    (queryId?: string) => {
      return integration.startManualProgress("rag_query", repositoryId, {
        queryId,
      });
    },
    [integration, repositoryId]
  );

  const updateQuery = useCallback(
    (updates: any) => {
      integration.updateManualProgress("rag_query", repositoryId, updates);
    },
    [integration, repositoryId]
  );

  const completeQuery = useCallback(() => {
    integration.completeManualProgress("rag_query", repositoryId);
  }, [integration, repositoryId]);

  const errorQuery = useCallback(
    (error: string) => {
      integration.errorManualProgress("rag_query", repositoryId, error);
    },
    [integration, repositoryId]
  );

  return {
    startQuery,
    updateQuery,
    completeQuery,
    errorQuery,
  };
}

export function useResearchProgress(repositoryId: string) {
  const integration = useProgressIntegration();

  const startResearch = useCallback(
    (config?: any) => {
      return integration.startManualProgress("research", repositoryId, config);
    },
    [integration, repositoryId]
  );

  const updateResearch = useCallback(
    (updates: any) => {
      integration.updateManualProgress("research", repositoryId, updates);
    },
    [integration, repositoryId]
  );

  const completeResearch = useCallback(() => {
    integration.completeManualProgress("research", repositoryId);
  }, [integration, repositoryId]);

  const errorResearch = useCallback(
    (error: string) => {
      integration.errorManualProgress("research", repositoryId, error);
    },
    [integration, repositoryId]
  );

  return {
    startResearch,
    updateResearch,
    completeResearch,
    errorResearch,
  };
}
