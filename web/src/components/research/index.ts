/**
 * 智能研究组件导出
 */

export { DeepResearchInterface } from "./DeepResearchInterface";
export { StreamingResearchInterface } from "./StreamingResearchInterface";
export {
  ResearchStageViewer,
  CompactResearchStageViewer,
} from "./ResearchStageViewer";
export { ResearchProgressIndicator } from "./ResearchProgressIndicator";
export {
  ResearchNavigation,
  useResearchKeyboardNavigation,
} from "./ResearchNavigation";

// 重新导出hooks
export { useDeepResearch } from "@/hooks/use-deep-research";
export { useStreamingResearch } from "@/hooks/use-streaming-research";
