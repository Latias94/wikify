/**
 * 搜索相关类型定义
 */

import { WikiPage, WikiSection } from './api';

/**
 * 搜索结果项类型
 */
export type SearchResultType = 'page' | 'section' | 'content';

/**
 * 搜索结果项
 */
export interface SearchResultItem {
  id: string;
  type: SearchResultType;
  title: string;
  content: string;
  excerpt: string; // 搜索匹配的摘录
  score: number; // 搜索相关性分数 (0-1)
  highlights: SearchHighlight[]; // 高亮信息
  metadata: SearchResultMetadata;
}

/**
 * 搜索高亮信息
 */
export interface SearchHighlight {
  field: string; // 'title' | 'content'
  indices: [number, number][]; // 高亮位置 [start, end]
  value: string; // 高亮的文本
}

/**
 * 搜索结果元数据
 */
export interface SearchResultMetadata {
  pageId?: string; // 所属页面ID
  sectionId?: string; // 所属章节ID
  importance?: 'High' | 'Medium' | 'Low'; // 重要性
  filePaths?: string[]; // 相关文件路径
  breadcrumb: string[]; // 面包屑导航
}

/**
 * 搜索配置
 */
export interface SearchConfig {
  threshold: number; // 搜索阈值 (0-1, 越小越严格)
  includeScore: boolean; // 是否包含分数
  includeMatches: boolean; // 是否包含匹配信息
  minMatchCharLength: number; // 最小匹配字符长度
  maxResults: number; // 最大结果数量
  keys: SearchKey[]; // 搜索字段配置
}

/**
 * 搜索字段配置
 */
export interface SearchKey {
  name: string; // 字段名
  weight: number; // 权重 (0-1)
}

/**
 * 搜索索引项 - 用于Fuse.js
 */
export interface SearchIndexItem {
  id: string;
  type: SearchResultType;
  title: string;
  content: string;
  pageId?: string;
  sectionId?: string;
  importance?: 'High' | 'Medium' | 'Low';
  filePaths?: string[];
  breadcrumb: string[];
}

/**
 * 搜索查询参数
 */
export interface SearchQuery {
  query: string; // 搜索关键词
  type?: SearchResultType[]; // 限制搜索类型
  importance?: ('High' | 'Medium' | 'Low')[]; // 限制重要性
  pageId?: string; // 限制在特定页面内搜索
}

/**
 * 搜索结果
 */
export interface SearchResults {
  query: string;
  total: number;
  items: SearchResultItem[];
  suggestions?: string[]; // 搜索建议
  executionTime: number; // 执行时间(ms)
}

/**
 * 搜索统计信息
 */
export interface SearchStats {
  totalDocuments: number; // 总文档数
  totalPages: number; // 总页面数
  totalSections: number; // 总章节数
  indexSize: number; // 索引大小
  lastUpdated: Date; // 最后更新时间
}

/**
 * 搜索过滤器
 */
export interface SearchFilters {
  types: SearchResultType[];
  importance: ('High' | 'Medium' | 'Low')[];
  pages: string[]; // 页面ID列表
}

/**
 * 搜索历史记录
 */
export interface SearchHistoryItem {
  id: string;
  query: string;
  timestamp: Date;
  resultCount: number;
}
