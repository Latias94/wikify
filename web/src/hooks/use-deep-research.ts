/**
 * 深度研究相关的 React Hooks
 * 提供智能研究功能的状态管理和操作
 */

import { useState, useEffect, useCallback, useRef } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useToast } from "@/hooks/use-toast";
import { apiClient } from "@/lib/api-client";
import { useResearchWebSocket } from "@/hooks/use-websocket";
import {
  DeepResearchRequest,
  DeepResearchResponse,
  ResearchStage,
  ResearchProgressUpdate,
  ResearchStageType,
} from "@/types/api";

// ============================================================================
// 研究状态管理
// ============================================================================

interface ResearchState {
  researchId: string | null;
  sessionId: string | null;
  status: "idle" | "planning" | "researching" | "completed" | "failed";
  currentIteration: number;
  maxIterations: number;
  stages: ResearchStage[];
  currentStageIndex: number;
  progress: {
    current_stage: string;
    completion_percentage: number;
    estimated_time_remaining?: number;
  };
  finalConclusion?: string;
  confidenceScore?: number;
  isAutoProgressing: boolean;
}

const initialState: ResearchState = {
  researchId: null,
  sessionId: null,
  status: "idle",
  currentIteration: 0,
  maxIterations: 5,
  stages: [],
  currentStageIndex: 0,
  progress: {
    current_stage: "",
    completion_percentage: 0,
  },
  isAutoProgressing: false,
};

// ============================================================================
// 深度研究 Hook
// ============================================================================

export const useDeepResearch = () => {
  const [state, setState] = useState<ResearchState>(initialState);
  const { toast } = useToast();
  const queryClient = useQueryClient();

  // 统一的 WebSocket 连接
  const { isConnected: wsConnected, disconnect: wsDisconnect } =
    useResearchWebSocket(
      state.researchId,
      // onProgress
      (update) => {
        handleProgressUpdate(update);
      },
      // onComplete
      (result) => {
        setState((prev) => ({
          ...prev,
          status: "completed",
          finalConclusion: result.conclusion || prev.finalConclusion,
        }));
      },
      // onError
      (error) => {
        setState((prev) => ({
          ...prev,
          status: "failed",
          error,
        }));
        toast({
          title: "Research Error",
          description: error,
          variant: "destructive",
        });
      }
    );

  // ============================================================================
  // 工具函数
  // ============================================================================

  /**
   * 检查研究是否完成
   */
  const checkIfResearchComplete = useCallback((content: string): boolean => {
    // 检查明确的结论标记
    if (
      content.includes("## Final Conclusion") ||
      content.includes("## 最终结论")
    ) {
      return true;
    }

    // 检查结论部分
    if (
      (content.includes("## Conclusion") ||
        content.includes("## Summary") ||
        content.includes("## 总结")) &&
      !content.includes("I will now proceed to") &&
      !content.includes("Next Steps") &&
      !content.includes("next iteration")
    ) {
      return true;
    }

    // 检查完成短语
    const completionPhrases = [
      "This concludes our research",
      "This completes our investigation",
      "This concludes the deep research process",
      "Key Findings and Implementation Details",
      "In conclusion,",
      "研究结论",
      "综合分析完成",
    ];

    return completionPhrases.some((phrase) => content.includes(phrase));
  }, []);

  /**
   * 提取研究阶段
   */
  const extractResearchStage = useCallback(
    (content: string, iteration: number): ResearchStage | null => {
      const timestamp = new Date().toISOString();
      const stageId = `stage-${iteration}-${Date.now()}`;

      // 研究计划（第一次迭代）
      if (iteration === 1 && content.includes("## Research Plan")) {
        return {
          id: stageId,
          title: "Research Plan",
          content,
          iteration: 1,
          type: "plan",
          timestamp,
        };
      }

      // 研究更新（迭代 1-4）
      if (iteration >= 1 && iteration <= 4) {
        const updateMatch = content.match(
          new RegExp(`## Research Update ${iteration}`)
        );
        if (updateMatch) {
          return {
            id: stageId,
            title: `Research Update ${iteration}`,
            content,
            iteration,
            type: "update",
            timestamp,
          };
        }
      }

      // 最终结论
      if (content.includes("## Final Conclusion")) {
        return {
          id: stageId,
          title: "Final Conclusion",
          content,
          iteration,
          type: "conclusion",
          timestamp,
        };
      }

      return null;
    },
    []
  );

  /**
   * 处理进度更新
   */
  const handleProgressUpdate = useCallback(
    (update: ResearchProgressUpdate) => {
      setState((prev) => {
        const newStages = [...prev.stages];

        // 查找现有阶段或添加新阶段
        const existingIndex = newStages.findIndex(
          (s) => s.id === update.stage.id
        );
        if (existingIndex >= 0) {
          newStages[existingIndex] = update.stage;
        } else {
          newStages.push(update.stage);
          newStages.sort((a, b) => a.iteration - b.iteration);
        }

        return {
          ...prev,
          stages: newStages,
          currentStageIndex: newStages.length - 1,
          progress: update.progress,
          status: update.is_complete ? "completed" : "researching",
          finalConclusion: update.is_complete
            ? update.stage.content
            : prev.finalConclusion,
        };
      });

      if (update.is_complete) {
        toast({
          title: "Research Complete",
          description: "Deep research has finished successfully",
        });
      }
    },
    [toast]
  );

  // ============================================================================
  // 研究操作
  // ============================================================================

  /**
   * 开始深度研究
   */
  const startResearchMutation = useMutation({
    mutationFn: (request: DeepResearchRequest) =>
      apiClient.startDeepResearch(request),
    onSuccess: (response: DeepResearchResponse) => {
      setState((prev) => ({
        ...prev,
        researchId: response.research_id,
        sessionId: response.research_id,
        status: response.status,
        currentIteration: response.current_iteration,
        maxIterations: response.max_iterations,
        stages: response.stages,
        progress: response.progress,
        isAutoProgressing: true,
      }));

      // WebSocket 连接由 useResearchWebSocket hook 自动管理

      toast({
        title: "Research Started",
        description: "Deep research process has begun",
      });
    },
    onError: (error: any) => {
      setState((prev) => ({ ...prev, status: "failed" }));
      toast({
        title: "Research Failed",
        description: error.message || "Failed to start research",
        variant: "destructive",
      });
    },
  });

  /**
   * 停止研究
   */
  const stopResearch = useCallback(() => {
    wsDisconnect();

    setState((prev) => ({
      ...prev,
      status: "completed",
      isAutoProgressing: false,
    }));

    toast({
      title: "Research Stopped",
      description: "Research process has been stopped",
    });
  }, [wsDisconnect, toast]);

  /**
   * 重置研究状态
   */
  const resetResearch = useCallback(() => {
    wsDisconnect();

    setState(initialState);
  }, [wsDisconnect]);

  // ============================================================================
  // 导航操作
  // ============================================================================

  /**
   * 导航到特定阶段
   */
  const navigateToStage = useCallback(
    (index: number) => {
      if (index >= 0 && index < state.stages.length) {
        setState((prev) => ({
          ...prev,
          currentStageIndex: index,
        }));
      }
    },
    [state.stages.length]
  );

  /**
   * 导航到下一阶段
   */
  const navigateToNextStage = useCallback(() => {
    navigateToStage(state.currentStageIndex + 1);
  }, [navigateToStage, state.currentStageIndex]);

  /**
   * 导航到上一阶段
   */
  const navigateToPreviousStage = useCallback(() => {
    navigateToStage(state.currentStageIndex - 1);
  }, [navigateToStage, state.currentStageIndex]);

  // ============================================================================
  // 清理
  // ============================================================================

  useEffect(() => {
    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, []);

  // ============================================================================
  // 返回值
  // ============================================================================

  return {
    // 状态
    ...state,
    currentStage: state.stages[state.currentStageIndex] || null,

    // 操作
    startResearch: startResearchMutation.mutate,
    stopResearch,
    resetResearch,

    // 导航
    navigateToStage,
    navigateToNextStage,
    navigateToPreviousStage,

    // 状态检查
    canNavigateNext: state.currentStageIndex < state.stages.length - 1,
    canNavigatePrevious: state.currentStageIndex > 0,
    isResearching:
      state.status === "researching" || state.status === "planning",
    isComplete: state.status === "completed",

    // 加载状态
    isStarting: startResearchMutation.isPending,

    // WebSocket 状态
    isConnected: wsConnected(),
  };
};
