//! Core trait definitions

use crate::error::WikifyResult;
use crate::types::*;
use async_trait::async_trait;

/// Repository processor trait
#[async_trait]
pub trait RepositoryProcessor {
    /// Clone or update repository
    async fn clone_repository(&self, repo_info: &RepoInfo) -> WikifyResult<String>;

    /// Read repository documents
    async fn read_documents(&self, repo_path: &str) -> WikifyResult<Vec<DocumentInfo>>;

    /// Get repository statistics
    async fn get_repo_stats(&self, repo_path: &str) -> WikifyResult<RepoStats>;
}

/// Document indexer trait
#[async_trait]
pub trait DocumentIndexer {
    /// 索引文档
    async fn index_documents(&self, documents: Vec<DocumentInfo>) -> WikifyResult<IndexStats>;

    /// 搜索相关文档
    async fn search(&self, query: &str, top_k: usize) -> WikifyResult<Vec<DocumentInfo>>;

    /// 获取索引统计
    async fn get_index_stats(&self) -> WikifyResult<IndexStats>;
}

/// RAG系统trait
#[async_trait]
pub trait RagSystem {
    /// 普通问答
    async fn query(&self, question: &str) -> WikifyResult<RagResponse>;

    /// 深度研究
    async fn deep_research(
        &self,
        topic: &str,
        max_iterations: usize,
    ) -> WikifyResult<ResearchResult>;

    /// 添加对话历史
    async fn add_conversation(&self, question: &str, answer: &str) -> WikifyResult<()>;
}

/// Wiki生成器trait - 使用泛型以保持核心模块的通用性
#[async_trait]
pub trait WikiGenerator<WikiStructure, WikiPage> {
    /// 生成Wiki结构
    async fn generate_structure(&self, documents: &[DocumentInfo]) -> WikifyResult<WikiStructure>;

    /// 生成Wiki页面
    async fn generate_page(
        &self,
        page_id: &str,
        context: &[DocumentInfo],
    ) -> WikifyResult<WikiPage>;

    /// 生成可视化图表
    async fn generate_diagrams(&self, documents: &[DocumentInfo]) -> WikifyResult<Vec<Diagram>>;
}

/// 存储系统trait - 使用泛型以保持核心模块的通用性
#[async_trait]
pub trait StorageSystem<WikiStructure> {
    /// 保存Wiki数据
    async fn save_wiki(&self, wiki: &WikiStructure) -> WikifyResult<()>;

    /// 加载Wiki数据
    async fn load_wiki(&self, repo_id: &str) -> WikifyResult<Option<WikiStructure>>;

    /// 保存索引数据
    async fn save_index(&self, repo_id: &str, index_data: &[u8]) -> WikifyResult<()>;

    /// 加载索引数据
    async fn load_index(&self, repo_id: &str) -> WikifyResult<Option<Vec<u8>>>;
}

/// 辅助数据结构
#[derive(Debug, Clone)]
pub struct RepoStats {
    pub total_files: usize,
    pub code_files: usize,
    pub doc_files: usize,
    pub total_lines: usize,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub total_documents: usize,
    pub total_chunks: usize,
    pub embedding_dimension: usize,
    pub index_size_mb: f64,
}

#[derive(Debug, Clone)]
pub struct ResearchResult {
    pub topic: String,
    pub iterations: Vec<ResearchIteration>,
    pub final_answer: String,
    pub sources: Vec<DocumentInfo>,
}

#[derive(Debug, Clone)]
pub struct ResearchIteration {
    pub iteration: usize,
    pub plan: String,
    pub findings: String,
    pub next_steps: String,
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub diagram_type: DiagramType,
    pub title: String,
    pub content: String, // Mermaid格式
}

#[derive(Debug, Clone)]
pub enum DiagramType {
    Architecture,
    DataFlow,
    ClassDiagram,
    SequenceDiagram,
}
