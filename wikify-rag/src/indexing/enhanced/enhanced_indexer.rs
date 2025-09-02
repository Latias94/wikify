//! Enhanced document indexer using cheungfun's complete NodeParser ecosystem
//!
//! This module provides a sophisticated document indexing system that leverages
//! cheungfun's advanced features including:
//! - SweepAI and LlamaIndex style AST-aware code splitting
//! - Hierarchical node parsing with proper trait design
//! - Type-safe transformations using TypedTransform system
//! - Comprehensive configuration system with scenario-based presets

use async_trait::async_trait;
use cheungfun_core::{
    deduplication::DocstoreStrategy,
    traits::{TypedData, TypedTransform},
    Document, Node,
};
use cheungfun_indexing::{
    loaders::ProgrammingLanguage,
    node_parser::{
        config::{ChunkingStrategy, CodeSplitterConfig},
        text::{
            CodeSplitter, MarkdownNodeParser, SemanticSplitter, SentenceSplitter, TokenTextSplitter,
        },
        NodeParser,
    },
    pipeline::indexing::PipelineConfig as CheungfunPipelineConfig,
};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use wikify_core::{ErrorContext, WikifyError, WikifyResult};

use crate::indexing::traits::{DocumentIndexerImpl, IndexingConfig, IndexingStats};

/// Enhanced configuration for document indexing with cheungfun integration
#[derive(Debug, Clone)]
pub struct EnhancedIndexingConfig {
    /// Chunking strategy for different content types
    pub text_chunking_strategy: ChunkingStrategy,
    pub code_chunking_strategy: ChunkingStrategy,

    /// Base chunk configuration
    pub chunk_size: usize,
    pub chunk_overlap: usize,

    /// Advanced features
    pub enable_semantic_splitting: bool,
    pub enable_ast_code_splitting: bool,
    pub preserve_markdown_structure: bool,
    pub enable_hierarchical_parsing: bool,

    /// Processing options
    pub batch_size: usize,
    pub max_concurrency: usize,
    pub continue_on_error: bool,

    /// Language-specific configurations
    pub language_configs: HashMap<ProgrammingLanguage, CodeSplitterConfig>,
}

impl Default for EnhancedIndexingConfig {
    fn default() -> Self {
        Self {
            text_chunking_strategy: ChunkingStrategy::Balanced,
            code_chunking_strategy: ChunkingStrategy::Optimal, // Uses SweepAI algorithm
            chunk_size: 350,
            chunk_overlap: 100,
            enable_semantic_splitting: false, // Requires embedding model
            enable_ast_code_splitting: true,
            preserve_markdown_structure: true,
            enable_hierarchical_parsing: false, // Advanced feature
            batch_size: 32,
            max_concurrency: 4,
            continue_on_error: true,
            language_configs: HashMap::new(),
        }
    }
}

impl EnhancedIndexingConfig {
    /// Create configuration optimized for code repositories
    pub fn for_code_repository() -> Self {
        let mut config = Self::default();
        config.code_chunking_strategy = ChunkingStrategy::Optimal; // Uses SweepAI algorithm
        config.enable_ast_code_splitting = true;
        config.chunk_size = 400; // Larger chunks for code context
        config.chunk_overlap = 80;

        // Add optimized configurations for common languages
        config.language_configs.insert(
            ProgrammingLanguage::Rust,
            CodeSplitterConfig::optimal(ProgrammingLanguage::Rust),
        );
        config.language_configs.insert(
            ProgrammingLanguage::Python,
            CodeSplitterConfig::optimal(ProgrammingLanguage::Python),
        );
        config.language_configs.insert(
            ProgrammingLanguage::JavaScript,
            CodeSplitterConfig::optimal(ProgrammingLanguage::JavaScript),
        );
        config.language_configs.insert(
            ProgrammingLanguage::TypeScript,
            CodeSplitterConfig::optimal(ProgrammingLanguage::TypeScript),
        );

        config
    }

    /// Create configuration optimized for large enterprise codebases
    pub fn for_enterprise() -> Self {
        let mut config = Self::for_code_repository();
        config.code_chunking_strategy = ChunkingStrategy::Enterprise;
        config.chunk_size = 600; // Larger context for complex systems
        config.chunk_overlap = 120;
        config.batch_size = 16; // Smaller batches for stability
        config.max_concurrency = 2;

        // Use enterprise configurations for all languages
        let languages: Vec<ProgrammingLanguage> = config.language_configs.keys().cloned().collect();
        for lang in languages {
            config
                .language_configs
                .insert(lang, CodeSplitterConfig::enterprise(lang));
        }

        config
    }

    /// Create configuration optimized for documentation
    pub fn for_documentation() -> Self {
        let mut config = Self::default();
        config.text_chunking_strategy = ChunkingStrategy::Fine;
        config.preserve_markdown_structure = true;
        config.enable_hierarchical_parsing = true;
        config.chunk_size = 300; // Smaller chunks for precise retrieval
        config.chunk_overlap = 75;
        config
    }
}

impl From<IndexingConfig> for EnhancedIndexingConfig {
    fn from(config: IndexingConfig) -> Self {
        Self {
            text_chunking_strategy: ChunkingStrategy::Balanced,
            code_chunking_strategy: ChunkingStrategy::Optimal, // Uses SweepAI
            chunk_size: config.chunk_size,
            chunk_overlap: config.chunk_overlap,
            enable_semantic_splitting: config.enable_semantic_splitting,
            enable_ast_code_splitting: config.enable_ast_code_splitting,
            preserve_markdown_structure: config.preserve_markdown_structure,
            enable_hierarchical_parsing: false, // Advanced feature, default off
            batch_size: config.batch_size,
            max_concurrency: config.max_concurrency,
            continue_on_error: config.continue_on_error,
            language_configs: HashMap::new(), // Will be populated during initialization
        }
    }
}

/// Enhanced document indexer that leverages cheungfun's complete ecosystem
#[derive(Debug)]
pub struct EnhancedDocumentIndexer {
    config: IndexingConfig,
    enhanced_config: EnhancedIndexingConfig,

    // Text splitters for different strategies
    sentence_splitter: SentenceSplitter,
    token_splitter: TokenTextSplitter,
    semantic_splitter: Option<SemanticSplitter>,

    // Specialized parsers
    markdown_parser: MarkdownNodeParser,

    // Code splitters with different strategies
    code_splitters: HashMap<ProgrammingLanguage, CodeSplitter>,

    // Pipeline for batch processing
    pipeline_config: CheungfunPipelineConfig,

    // Statistics tracking
    stats: IndexingStats,
}

impl EnhancedDocumentIndexer {
    /// Create a new enhanced document indexer with default configuration
    pub fn new() -> WikifyResult<Self> {
        Self::with_unified_config(IndexingConfig::default())
    }

    /// Create a new enhanced document indexer with unified configuration
    pub fn with_unified_config(config: IndexingConfig) -> WikifyResult<Self> {
        let enhanced_config = EnhancedIndexingConfig::from(config.clone());
        Self::with_enhanced_config(config, enhanced_config)
    }

    /// Create a new enhanced document indexer with custom enhanced configuration
    pub fn with_enhanced_config(
        config: IndexingConfig,
        enhanced_config: EnhancedIndexingConfig,
    ) -> WikifyResult<Self> {
        info!(
            "Creating enhanced document indexer with config: {:?}",
            enhanced_config
        );

        // Initialize text splitters
        let sentence_splitter = SentenceSplitter::from_defaults(
            enhanced_config.chunk_size,
            enhanced_config.chunk_overlap,
        )
        .map_err(|e| WikifyError::Indexing {
            message: format!("Failed to create sentence splitter: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("enhanced_indexer")
                .with_operation("create_sentence_splitter"),
        })?;

        let token_splitter = TokenTextSplitter::from_defaults(
            enhanced_config.chunk_size,
            enhanced_config.chunk_overlap,
        )
        .map_err(|e| WikifyError::Indexing {
            message: format!("Failed to create token splitter: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("enhanced_indexer").with_operation("create_token_splitter"),
        })?;

        // Initialize semantic splitter if enabled (requires embedding model)
        let semantic_splitter = if enhanced_config.enable_semantic_splitting {
            warn!("Semantic splitting requested but not yet implemented - requires embedding model integration");
            None
        } else {
            None
        };

        // Initialize markdown parser
        let markdown_parser = MarkdownNodeParser::new();

        // Initialize code splitters with advanced configurations
        let mut code_splitters = HashMap::new();
        if enhanced_config.enable_ast_code_splitting {
            let languages_to_configure = vec![
                ProgrammingLanguage::Rust,
                ProgrammingLanguage::Python,
                ProgrammingLanguage::JavaScript,
                ProgrammingLanguage::TypeScript,
                ProgrammingLanguage::Java,
                ProgrammingLanguage::Cpp,
                ProgrammingLanguage::Go,
                ProgrammingLanguage::CSharp,
            ];

            for lang in languages_to_configure {
                let splitter_config = enhanced_config
                    .language_configs
                    .get(&lang)
                    .cloned()
                    .unwrap_or_else(|| match enhanced_config.code_chunking_strategy {
                        ChunkingStrategy::Optimal => CodeSplitterConfig::optimal(lang), // Uses SweepAI
                        ChunkingStrategy::Enterprise => CodeSplitterConfig::enterprise(lang),
                        ChunkingStrategy::Fine => CodeSplitterConfig::fine_grained(lang),
                        ChunkingStrategy::Balanced => CodeSplitterConfig::balanced(lang),
                        ChunkingStrategy::Coarse => CodeSplitterConfig::coarse_grained(lang),
                        ChunkingStrategy::Minimal => CodeSplitterConfig::fine_grained(lang), // Use fine as fallback
                    });

                match CodeSplitter::new(splitter_config) {
                    Ok(splitter) => {
                        debug!(
                            "Created {:?} code splitter for {:?}",
                            enhanced_config.code_chunking_strategy, lang
                        );
                        code_splitters.insert(lang, splitter);
                    }
                    Err(e) => {
                        warn!("Failed to create code splitter for {:?}: {}", lang, e);
                        // Continue with other languages
                    }
                }
            }
        }

        // Initialize pipeline configuration
        let pipeline_config = CheungfunPipelineConfig {
            max_concurrency: enhanced_config.max_concurrency,
            batch_size: enhanced_config.batch_size,
            continue_on_error: enhanced_config.continue_on_error,
            operation_timeout_seconds: Some(300), // 5 minutes
            enable_progress_reporting: true,
            enable_caching: true,
            cache_ttl_seconds: 3600, // 1 hour
            docstore_strategy: DocstoreStrategy::Upserts,
            enable_deduplication: true,
        };

        let stats = IndexingStats::new("enhanced");

        Ok(Self {
            config,
            enhanced_config,
            sentence_splitter,
            token_splitter,
            semantic_splitter,
            markdown_parser,
            code_splitters,
            pipeline_config,
            stats,
        })
    }

    /// Create indexer optimized for code repositories
    pub fn for_code_repository() -> WikifyResult<Self> {
        Self::with_unified_config(IndexingConfig::for_code_repository())
    }

    /// Create indexer optimized for enterprise codebases
    pub fn for_enterprise() -> WikifyResult<Self> {
        Self::with_unified_config(IndexingConfig::for_enterprise())
    }

    /// Create indexer optimized for documentation
    pub fn for_documentation() -> WikifyResult<Self> {
        Self::with_unified_config(IndexingConfig::for_documentation())
    }
}

impl Default for EnhancedDocumentIndexer {
    fn default() -> Self {
        Self::new().expect("Failed to create default EnhancedDocumentIndexer")
    }
}

impl EnhancedDocumentIndexer {
    /// Index a batch of documents using the enhanced pipeline
    pub async fn index_documents(&self, documents: Vec<Document>) -> WikifyResult<Vec<Node>> {
        info!("Enhanced indexing {} documents", documents.len());

        let start_time = std::time::Instant::now();
        let mut all_nodes = Vec::new();
        let mut stats = EnhancedIndexingStats::new();

        // Process documents in batches for better performance
        let chunks: Vec<_> = documents.chunks(self.config.batch_size).collect();

        for (batch_idx, batch) in chunks.iter().enumerate() {
            debug!("Processing batch {}/{}", batch_idx + 1, chunks.len());

            let batch_nodes = self.process_document_batch(batch.to_vec()).await?;
            all_nodes.extend(batch_nodes);
        }

        stats.total_documents = documents.len();
        stats.total_nodes = all_nodes.len();
        stats.processing_time_ms = start_time.elapsed().as_millis();
        stats.avg_nodes_per_document = if documents.is_empty() {
            0.0
        } else {
            all_nodes.len() as f64 / documents.len() as f64
        };

        info!(
            "Enhanced indexing completed: {} nodes from {} documents in {}ms",
            all_nodes.len(),
            documents.len(),
            stats.processing_time_ms
        );

        Ok(all_nodes)
    }

    /// Process a batch of documents with intelligent parser selection
    async fn process_document_batch(&self, documents: Vec<Document>) -> WikifyResult<Vec<Node>> {
        let mut batch_nodes = Vec::new();

        for document in documents {
            let nodes = self.index_single_document(document).await?;
            batch_nodes.extend(nodes);
        }

        Ok(batch_nodes)
    }

    /// Index a single document with intelligent parser selection
    async fn index_single_document(&self, document: Document) -> WikifyResult<Vec<Node>> {
        debug!("Enhanced indexing document: {:?}", document.id);

        // Analyze document to determine optimal parsing strategy
        let parsing_strategy = self.determine_parsing_strategy(&document);
        debug!(
            "Selected parsing strategy: {:?} for document {:?}",
            parsing_strategy, document.id
        );

        let document_id = document.id.clone();
        let nodes = match parsing_strategy {
            ParsingStrategy::AdvancedCode { language } => {
                self.parse_with_advanced_code_splitter(&document, language)
                    .await?
            }
            ParsingStrategy::Markdown => self.parse_with_markdown_parser(document).await?,
            ParsingStrategy::Semantic => self.parse_with_semantic_splitter(document).await?,
            ParsingStrategy::Token => self.parse_with_token_splitter(document).await?,
            ParsingStrategy::Sentence => self.parse_with_sentence_splitter(document).await?,
        };

        debug!(
            "Generated {} nodes for document {:?}",
            nodes.len(),
            document_id
        );
        Ok(nodes)
    }

    /// Determine the optimal parsing strategy for a document
    fn determine_parsing_strategy(&self, document: &Document) -> ParsingStrategy {
        // Check file type and language metadata
        let file_type = document.metadata.get("file_type").and_then(|v| v.as_str());
        let language = document.metadata.get("language").and_then(|v| v.as_str());
        let file_extension = document
            .metadata
            .get("file_extension")
            .and_then(|v| v.as_str());

        // Priority 1: Advanced code splitting for supported languages
        if self.config.enable_ast_code_splitting {
            if let Some(lang_str) = language.or(file_extension) {
                if let Some(programming_lang) = self.detect_programming_language(lang_str) {
                    if self.code_splitters.contains_key(&programming_lang) {
                        return ParsingStrategy::AdvancedCode {
                            language: programming_lang,
                        };
                    }
                }
            }
        }

        // Priority 2: Markdown structure preservation
        if self.config.preserve_markdown_structure {
            if let Some(lang) = language {
                if lang == "markdown" || lang == "md" {
                    return ParsingStrategy::Markdown;
                }
            }
            if let Some(ext) = file_extension {
                if ext == "md" || ext == "markdown" {
                    return ParsingStrategy::Markdown;
                }
            }
        }

        // Priority 3: Semantic splitting (if available and enabled)
        if self.config.enable_semantic_splitting && self.semantic_splitter.is_some() {
            return ParsingStrategy::Semantic;
        }

        // Priority 4: Token-based splitting for code files (fallback)
        if file_type == Some("code") {
            return ParsingStrategy::Token;
        }

        // Default: Sentence-based splitting
        ParsingStrategy::Sentence
    }

    /// Parse document using advanced AST-aware code splitter
    async fn parse_with_advanced_code_splitter(
        &self,
        document: &Document,
        language: ProgrammingLanguage,
    ) -> WikifyResult<Vec<Node>> {
        if let Some(code_splitter) = self.code_splitters.get(&language) {
            debug!(
                "Using advanced {:?} code splitter with {:?} strategy",
                language, self.enhanced_config.code_chunking_strategy
            );

            let input = TypedData::from_documents(vec![document.clone()]);
            let result =
                code_splitter
                    .transform(input)
                    .await
                    .map_err(|e| WikifyError::Indexing {
                        message: format!(
                            "Advanced code splitting failed for {:?}: {}",
                            language, e
                        ),
                        source: Some(Box::new(e)),
                        context: ErrorContext::new("enhanced_indexer")
                            .with_operation("advanced_code_split"),
                    })?;

            Ok(result.into_nodes())
        } else {
            warn!(
                "Advanced code splitter not available for {:?}, falling back to token splitter",
                language
            );
            self.parse_with_token_splitter(document.clone()).await
        }
    }

    /// Parse document using markdown parser with structure preservation
    async fn parse_with_markdown_parser(&self, document: Document) -> WikifyResult<Vec<Node>> {
        debug!("Using markdown parser with structure preservation");

        Ok(
            NodeParser::parse_nodes(&self.markdown_parser, &[document], false)
                .await
                .map_err(|e| WikifyError::Indexing {
                    message: format!("Markdown parsing failed: {}", e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("enhanced_indexer").with_operation("markdown_parse"),
                })?,
        )
    }

    /// Parse document using semantic splitter (when available)
    async fn parse_with_semantic_splitter(&self, document: Document) -> WikifyResult<Vec<Node>> {
        if let Some(ref semantic_splitter) = self.semantic_splitter {
            debug!("Using semantic splitter for intelligent content-aware splitting");

            let input = TypedData::from_documents(vec![document]);
            let result =
                semantic_splitter
                    .transform(input)
                    .await
                    .map_err(|e| WikifyError::Indexing {
                        message: format!("Semantic splitting failed: {}", e),
                        source: Some(Box::new(e)),
                        context: ErrorContext::new("enhanced_indexer")
                            .with_operation("semantic_split"),
                    })?;

            Ok(result.into_nodes())
        } else {
            // Fallback to sentence splitter
            warn!("Semantic splitter not available, falling back to sentence splitter");
            self.parse_with_sentence_splitter(document).await
        }
    }

    /// Parse document using token splitter
    async fn parse_with_token_splitter(&self, document: Document) -> WikifyResult<Vec<Node>> {
        debug!("Using token splitter for precise token-based chunking");

        let input = TypedData::from_documents(vec![document]);
        let result =
            self.token_splitter
                .transform(input)
                .await
                .map_err(|e| WikifyError::Indexing {
                    message: format!("Token splitting failed: {}", e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("enhanced_indexer").with_operation("token_split"),
                })?;

        Ok(result.into_nodes())
    }

    /// Parse document using sentence splitter
    async fn parse_with_sentence_splitter(&self, document: Document) -> WikifyResult<Vec<Node>> {
        debug!("Using sentence splitter for natural language processing");

        let input = TypedData::from_documents(vec![document]);
        let result =
            self.sentence_splitter
                .transform(input)
                .await
                .map_err(|e| WikifyError::Indexing {
                    message: format!("Sentence splitting failed: {}", e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("enhanced_indexer").with_operation("sentence_split"),
                })?;

        Ok(result.into_nodes())
    }

    /// Detect programming language from string identifier
    fn detect_programming_language(&self, language_hint: &str) -> Option<ProgrammingLanguage> {
        match language_hint.to_lowercase().as_str() {
            "rust" | "rs" => Some(ProgrammingLanguage::Rust),
            "python" | "py" => Some(ProgrammingLanguage::Python),
            "javascript" | "js" => Some(ProgrammingLanguage::JavaScript),
            "typescript" | "ts" => Some(ProgrammingLanguage::TypeScript),
            "java" => Some(ProgrammingLanguage::Java),
            "cpp" | "c++" | "cc" | "cxx" => Some(ProgrammingLanguage::Cpp),
            "c" => Some(ProgrammingLanguage::C),
            "go" => Some(ProgrammingLanguage::Go),
            "csharp" | "cs" | "c#" => Some(ProgrammingLanguage::CSharp),
            _ => None,
        }
    }

    /// Get enhanced indexing statistics
    pub fn get_enhanced_stats(&self) -> EnhancedIndexingStats {
        EnhancedIndexingStats {
            total_documents: 0,
            total_nodes: 0,
            nodes_by_type: HashMap::new(),
            processing_time_ms: 0,
            avg_nodes_per_document: 0.0,
            chunking_strategy_used: format!(
                "Text: {:?}, Code: {:?}",
                self.enhanced_config.text_chunking_strategy,
                self.enhanced_config.code_chunking_strategy
            ),
            languages_processed: self
                .code_splitters
                .keys()
                .map(|lang| format!("{:?}", lang))
                .collect(),
            errors: Vec::new(),
        }
    }
}

/// Parsing strategy selection for different document types
#[derive(Debug, Clone)]
enum ParsingStrategy {
    /// Advanced AST-aware code splitting with specific language
    AdvancedCode { language: ProgrammingLanguage },
    /// Markdown structure-preserving parsing
    Markdown,
    /// Semantic content-aware splitting
    Semantic,
    /// Token-based precise splitting
    Token,
    /// Sentence-based natural language splitting
    Sentence,
}

/// Enhanced indexing statistics with detailed breakdown
#[derive(Debug, Clone)]
pub struct EnhancedIndexingStats {
    pub total_documents: usize,
    pub total_nodes: usize,
    pub nodes_by_type: HashMap<String, usize>,
    pub processing_time_ms: u128,
    pub avg_nodes_per_document: f64,
    pub chunking_strategy_used: String,
    pub languages_processed: Vec<String>,
    pub errors: Vec<String>,
}

impl EnhancedIndexingStats {
    fn new() -> Self {
        Self {
            total_documents: 0,
            total_nodes: 0,
            nodes_by_type: HashMap::new(),
            processing_time_ms: 0,
            avg_nodes_per_document: 0.0,
            chunking_strategy_used: String::new(),
            languages_processed: Vec::new(),
            errors: Vec::new(),
        }
    }
}

// Implement the internal DocumentIndexerImpl trait
#[async_trait]
impl DocumentIndexerImpl for EnhancedDocumentIndexer {
    fn config(&self) -> &IndexingConfig {
        &self.config
    }

    async fn index_documents(&self, documents: Vec<Document>) -> WikifyResult<Vec<Node>> {
        self.index_documents_impl(documents).await
    }

    fn get_stats(&self) -> IndexingStats {
        self.stats.clone()
    }

    fn supported_languages(&self) -> Vec<String> {
        self.code_splitters
            .keys()
            .map(|lang| format!("{:?}", lang).to_lowercase())
            .collect()
    }
}

impl EnhancedDocumentIndexer {
    /// Internal implementation of index_documents to avoid naming conflicts
    async fn index_documents_impl(&self, documents: Vec<Document>) -> WikifyResult<Vec<Node>> {
        info!("Enhanced indexing {} documents", documents.len());

        let start_time = std::time::Instant::now();
        let mut all_nodes = Vec::new();

        // Process documents in batches for better performance
        let chunks: Vec<_> = documents.chunks(self.enhanced_config.batch_size).collect();

        for (batch_idx, batch) in chunks.iter().enumerate() {
            debug!("Processing batch {}/{}", batch_idx + 1, chunks.len());

            let batch_nodes = self.process_document_batch(batch.to_vec()).await?;
            all_nodes.extend(batch_nodes);
        }

        info!(
            "Enhanced indexing completed: {} nodes from {} documents",
            all_nodes.len(),
            documents.len()
        );

        Ok(all_nodes)
    }
}
