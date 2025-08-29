/**
 * Wiki搜索引擎
 * 基于Fuse.js实现的全文搜索功能
 */

import Fuse from 'fuse.js';
import { WikiStructure, WikiPage, WikiSection } from '@/types/api';
import {
  SearchIndexItem,
  SearchResultItem,
  SearchResults,
  SearchQuery,
  SearchConfig,
  SearchStats,
  SearchResultType
} from '@/types/search';

/**
 * 默认搜索配置
 */
const DEFAULT_SEARCH_CONFIG: SearchConfig = {
  threshold: 0.4, // 搜索阈值，0.4表示60%匹配度
  includeScore: true,
  includeMatches: true,
  minMatchCharLength: 2,
  maxResults: 50,
  keys: [
    { name: 'title', weight: 0.4 }, // 标题权重最高
    { name: 'content', weight: 0.6 }, // 内容权重次之
  ]
};

/**
 * Wiki搜索引擎类
 */
export class WikiSearchEngine {
  private fuse: Fuse<SearchIndexItem> | null = null;
  private searchIndex: SearchIndexItem[] = [];
  private config: SearchConfig;
  private stats: SearchStats;

  constructor(config: Partial<SearchConfig> = {}) {
    this.config = { ...DEFAULT_SEARCH_CONFIG, ...config };
    this.stats = {
      totalDocuments: 0,
      totalPages: 0,
      totalSections: 0,
      indexSize: 0,
      lastUpdated: new Date()
    };
  }

  /**
   * 构建搜索索引
   */
  public buildIndex(wiki: WikiStructure): void {
    console.log('🔍 Building search index for wiki:', wiki.title);
    const startTime = performance.now();

    this.searchIndex = [];

    // 添加wiki概览
    this.addToIndex({
      id: 'overview',
      type: 'page',
      title: wiki.title,
      content: wiki.description,
      breadcrumb: [wiki.title]
    });

    // 索引所有页面
    wiki.pages.forEach(page => {
      this.indexPage(page, wiki.title);
    });

    // 索引根级章节
    wiki.sections.forEach(section => {
      this.indexSection(section, wiki.title, [wiki.title]);
    });

    // 创建Fuse实例
    this.fuse = new Fuse(this.searchIndex, {
      threshold: this.config.threshold,
      includeScore: this.config.includeScore,
      includeMatches: this.config.includeMatches,
      minMatchCharLength: this.config.minMatchCharLength,
      keys: this.config.keys.map(key => ({
        name: key.name,
        weight: key.weight
      }))
    });

    // 更新统计信息
    this.updateStats();

    const endTime = performance.now();
    console.log(`✅ Search index built in ${(endTime - startTime).toFixed(2)}ms`);
    console.log(`📊 Indexed ${this.stats.totalDocuments} documents`);
  }

  /**
   * 执行搜索
   */
  public search(query: SearchQuery): SearchResults {
    if (!this.fuse || !query.query.trim()) {
      return {
        query: query.query,
        total: 0,
        items: [],
        executionTime: 0
      };
    }

    const startTime = performance.now();

    // 执行Fuse搜索
    const fuseResults = this.fuse.search(query.query, {
      limit: this.config.maxResults
    });

    // 转换搜索结果
    let results = fuseResults.map(result => this.convertFuseResult(result));

    // 应用过滤器
    results = this.applyFilters(results, query);

    // 排序结果（按分数和重要性）
    results = this.sortResults(results);

    const endTime = performance.now();

    return {
      query: query.query,
      total: results.length,
      items: results,
      suggestions: this.generateSuggestions(query.query),
      executionTime: endTime - startTime
    };
  }

  /**
   * 获取搜索统计信息
   */
  public getStats(): SearchStats {
    return { ...this.stats };
  }

  /**
   * 清空索引
   */
  public clearIndex(): void {
    this.fuse = null;
    this.searchIndex = [];
    this.updateStats();
  }

  /**
   * 索引单个页面
   */
  private indexPage(page: WikiPage, wikiTitle: string): void {
    const breadcrumb = [wikiTitle, page.title];

    // 索引页面本身
    this.addToIndex({
      id: page.id,
      type: 'page',
      title: page.title,
      content: page.content,
      pageId: page.id,
      importance: page.importance,
      filePaths: page.file_paths,
      breadcrumb
    });

    // 索引页面的章节
    page.sections.forEach(section => {
      this.indexSection(section, wikiTitle, breadcrumb, page.id);
    });
  }

  /**
   * 索引单个章节（递归）
   */
  private indexSection(
    section: WikiSection,
    wikiTitle: string,
    parentBreadcrumb: string[],
    pageId?: string
  ): void {
    const breadcrumb = [...parentBreadcrumb, section.title];

    // 索引章节本身
    this.addToIndex({
      id: section.id,
      type: 'section',
      title: section.title,
      content: section.content,
      pageId,
      sectionId: section.id,
      breadcrumb
    });

    // 递归索引子章节
    section.subsections.forEach(subsection => {
      this.indexSection(subsection, wikiTitle, breadcrumb, pageId);
    });
  }

  /**
   * 添加项目到索引
   */
  private addToIndex(item: SearchIndexItem): void {
    this.searchIndex.push(item);
  }

  /**
   * 转换Fuse搜索结果
   */
  private convertFuseResult(fuseResult: Fuse.FuseResult<SearchIndexItem>): SearchResultItem {
    const item = fuseResult.item;
    const score = fuseResult.score || 0;

    // 生成摘录
    const excerpt = this.generateExcerpt(item.content, fuseResult.matches);

    // 生成高亮信息
    const highlights = fuseResult.matches?.map(match => ({
      field: match.key || '',
      indices: match.indices,
      value: match.value || ''
    })) || [];

    return {
      id: item.id,
      type: item.type,
      title: item.title,
      content: item.content,
      excerpt,
      score: 1 - score, // 转换为相关性分数 (越高越相关)
      highlights,
      metadata: {
        pageId: item.pageId,
        sectionId: item.sectionId,
        importance: item.importance,
        filePaths: item.filePaths,
        breadcrumb: item.breadcrumb
      }
    };
  }

  /**
   * 生成搜索摘录
   */
  private generateExcerpt(content: string, matches?: readonly Fuse.FuseResultMatch[]): string {
    if (!matches || matches.length === 0) {
      return content.slice(0, 200) + (content.length > 200 ? '...' : '');
    }

    // 找到第一个内容匹配
    const contentMatch = matches.find(match => match.key === 'content');
    if (!contentMatch || !contentMatch.indices.length) {
      return content.slice(0, 200) + (content.length > 200 ? '...' : '');
    }

    // 获取匹配位置周围的文本
    const firstMatch = contentMatch.indices[0];
    const start = Math.max(0, firstMatch[0] - 100);
    const end = Math.min(content.length, firstMatch[1] + 100);

    let excerpt = content.slice(start, end);
    if (start > 0) excerpt = '...' + excerpt;
    if (end < content.length) excerpt = excerpt + '...';

    return excerpt;
  }

  /**
   * 应用搜索过滤器
   */
  private applyFilters(results: SearchResultItem[], query: SearchQuery): SearchResultItem[] {
    let filtered = results;

    // 按类型过滤
    if (query.type && query.type.length > 0) {
      filtered = filtered.filter(item => query.type!.includes(item.type));
    }

    // 按重要性过滤
    if (query.importance && query.importance.length > 0) {
      filtered = filtered.filter(item => 
        item.metadata.importance && query.importance!.includes(item.metadata.importance)
      );
    }

    // 按页面过滤
    if (query.pageId) {
      filtered = filtered.filter(item => item.metadata.pageId === query.pageId);
    }

    return filtered;
  }

  /**
   * 排序搜索结果
   */
  private sortResults(results: SearchResultItem[]): SearchResultItem[] {
    return results.sort((a, b) => {
      // 首先按分数排序
      if (a.score !== b.score) {
        return b.score - a.score;
      }

      // 然后按重要性排序
      const importanceOrder = { 'High': 3, 'Medium': 2, 'Low': 1 };
      const aImportance = importanceOrder[a.metadata.importance || 'Low'];
      const bImportance = importanceOrder[b.metadata.importance || 'Low'];
      
      if (aImportance !== bImportance) {
        return bImportance - aImportance;
      }

      // 最后按类型排序 (页面 > 章节 > 内容)
      const typeOrder = { 'page': 3, 'section': 2, 'content': 1 };
      return typeOrder[b.type] - typeOrder[a.type];
    });
  }

  /**
   * 生成搜索建议
   */
  private generateSuggestions(query: string): string[] {
    // 简单的建议生成逻辑
    // 可以根据需要扩展为更复杂的算法
    const suggestions: string[] = [];
    
    // 基于索引中的标题生成建议
    const titles = this.searchIndex.map(item => item.title.toLowerCase());
    const queryLower = query.toLowerCase();
    
    titles.forEach(title => {
      if (title.includes(queryLower) && title !== queryLower) {
        suggestions.push(title);
      }
    });

    return suggestions.slice(0, 5); // 最多返回5个建议
  }

  /**
   * 更新统计信息
   */
  private updateStats(): void {
    const pages = this.searchIndex.filter(item => item.type === 'page');
    const sections = this.searchIndex.filter(item => item.type === 'section');

    this.stats = {
      totalDocuments: this.searchIndex.length,
      totalPages: pages.length,
      totalSections: sections.length,
      indexSize: JSON.stringify(this.searchIndex).length,
      lastUpdated: new Date()
    };
  }
}

/**
 * 全局搜索引擎实例
 */
export const wikiSearchEngine = new WikiSearchEngine();
