//! Document indexer using cheungfun's indexing capabilities
//!
//! This module provides document indexing functionality using cheungfun's
//! text splitters and node parsers.

use cheungfun_core::{
    traits::{TypedData, TypedTransform},
    Document, Node,
};
use cheungfun_indexing::loaders::ProgrammingLanguage;
use cheungfun_indexing::node_parser::{
    text::{CodeSplitter, MarkdownNodeParser, SentenceSplitter, TokenTextSplitter},
    NodeParser,
};
use tracing::{debug, info};
use wikify_core::{ErrorContext, WikifyError, WikifyResult};

/// Configuration for document indexing
#[derive(Debug, Clone)]
pub struct IndexingConfig {
    /// Chunk size for text splitting (in characters)
    pub chunk_size: usize,
    /// Overlap between chunks (in characters)
    pub chunk_overlap: usize,
    /// Whether to use sentence-aware splitting
    pub sentence_aware: bool,
    /// Whether to use token-based splitting for code files
    pub token_based_for_code: bool,
    /// Maximum tokens per chunk (for token-based splitting)
    pub max_tokens_per_chunk: usize,
    /// Whether to preserve markdown structure
    pub preserve_markdown_structure: bool,
    /// Whether to use AST-aware code splitting
    pub use_ast_code_splitting: bool,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 350, // Following DeepWiki's approach
            chunk_overlap: 100,
            sentence_aware: true,
            token_based_for_code: true,
            max_tokens_per_chunk: 250,
            preserve_markdown_structure: true,
            use_ast_code_splitting: true,
        }
    }
}

/// Document indexer that processes documents into searchable nodes
pub struct DocumentIndexer {
    config: IndexingConfig,
    sentence_splitter: SentenceSplitter,
    token_splitter: TokenTextSplitter,
    markdown_parser: MarkdownNodeParser,
    code_splitters: std::collections::HashMap<ProgrammingLanguage, CodeSplitter>,
}

impl DocumentIndexer {
    /// Create a new document indexer with default configuration
    pub fn new() -> WikifyResult<Self> {
        Self::with_config(IndexingConfig::default())
    }

    /// Create a new document indexer with custom configuration
    pub fn with_config(config: IndexingConfig) -> WikifyResult<Self> {
        // Initialize sentence splitter
        let sentence_splitter =
            SentenceSplitter::from_defaults(config.chunk_size, config.chunk_overlap).map_err(
                |e| WikifyError::Indexing {
                    message: format!("Failed to create sentence splitter: {}", e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("document_indexer")
                        .with_operation("create_sentence_splitter"),
                },
            )?;

        // Initialize token splitter
        let token_splitter = TokenTextSplitter::from_defaults(
            config.max_tokens_per_chunk,
            config.chunk_overlap / 4, // Adjust overlap for token-based splitting
        )
        .map_err(|e| WikifyError::Indexing {
            message: format!("Failed to create token splitter: {}", e),
            source: Some(Box::new(e)),
            context: ErrorContext::new("document_indexer").with_operation("create_token_splitter"),
        })?;

        // Initialize markdown parser
        let markdown_parser = MarkdownNodeParser::new();

        // Initialize code splitters for common languages
        let mut code_splitters = std::collections::HashMap::new();
        if config.use_ast_code_splitting {
            let common_languages = vec![
                ProgrammingLanguage::Rust,
                ProgrammingLanguage::Python,
                ProgrammingLanguage::JavaScript,
                ProgrammingLanguage::TypeScript,
                ProgrammingLanguage::Java,
                ProgrammingLanguage::Cpp,
                ProgrammingLanguage::Go,
            ];

            for lang in common_languages {
                if let Ok(splitter) =
                    CodeSplitter::from_defaults(lang, config.chunk_size, config.chunk_overlap, 512)
                {
                    code_splitters.insert(lang, splitter);
                }
            }
        }

        Ok(Self {
            config,
            sentence_splitter,
            token_splitter,
            markdown_parser,
            code_splitters,
        })
    }

    /// Index a batch of documents into nodes
    pub async fn index_documents(&self, documents: Vec<Document>) -> WikifyResult<Vec<Node>> {
        info!("Indexing {} documents", documents.len());

        let mut all_nodes = Vec::new();

        for document in documents {
            let nodes = self.index_single_document(document).await?;
            all_nodes.extend(nodes);
        }

        info!("Generated {} nodes from documents", all_nodes.len());
        Ok(all_nodes)
    }

    /// Index a single document into nodes
    async fn index_single_document(&self, document: Document) -> WikifyResult<Vec<Node>> {
        debug!("Indexing document: {:?}", document.id);

        // Determine the appropriate splitter based on file type
        let file_type = document.metadata.get("file_type").and_then(|v| v.as_str());
        let language = document.metadata.get("language").and_then(|v| v.as_str());

        let nodes = match (file_type, language) {
            // Use markdown parser for markdown files
            (_, Some("markdown")) if self.config.preserve_markdown_structure => {
                self.split_with_markdown_parser(document).await?
            }
            // Use AST-aware code splitter for supported languages
            (Some("code"), Some(lang)) if self.config.use_ast_code_splitting => {
                if let Some(programming_lang) = self.detect_programming_language(lang) {
                    if let Some(code_splitter) = self.code_splitters.get(&programming_lang) {
                        self.split_with_code_splitter(&document, code_splitter)
                            .await?
                    } else {
                        // Fallback to token splitter for unsupported languages
                        self.split_with_token_splitter(document).await?
                    }
                } else {
                    // Fallback to token splitter for unknown languages
                    self.split_with_token_splitter(document).await?
                }
            }
            // Use token splitter for code files (fallback)
            (Some("code"), _) if self.config.token_based_for_code => {
                self.split_with_token_splitter(document).await?
            }
            // Use sentence splitter for everything else
            _ => self.split_with_sentence_splitter(document).await?,
        };

        debug!("Generated {} nodes for document", nodes.len());
        Ok(nodes)
    }

    /// Split document using sentence splitter
    async fn split_with_sentence_splitter(&self, document: Document) -> WikifyResult<Vec<Node>> {
        let input = TypedData::from_documents(vec![document]);

        let result = self.sentence_splitter.transform(input).await.map_err(|e| {
            Box::new(WikifyError::Indexing {
                message: format!("Sentence splitting failed: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("document_indexer").with_operation("sentence_split"),
            })
        })?;

        Ok(result.into_nodes())
    }

    /// Split document using token splitter
    async fn split_with_token_splitter(&self, document: Document) -> WikifyResult<Vec<Node>> {
        let input = TypedData::from_documents(vec![document]);

        let result = self.token_splitter.transform(input).await.map_err(|e| {
            Box::new(WikifyError::Indexing {
                message: format!("Token splitting failed: {}", e),
                source: Some(Box::new(e)),
                context: ErrorContext::new("document_indexer").with_operation("token_split"),
            })
        })?;

        Ok(result.into_nodes())
    }

    /// Detect programming language from file extension or language hint
    fn detect_programming_language(&self, language_hint: &str) -> Option<ProgrammingLanguage> {
        match language_hint.to_lowercase().as_str() {
            "rust" | "rs" => Some(ProgrammingLanguage::Rust),
            "python" | "py" => Some(ProgrammingLanguage::Python),
            "javascript" | "js" => Some(ProgrammingLanguage::JavaScript),
            "typescript" | "ts" => Some(ProgrammingLanguage::TypeScript),
            "java" => Some(ProgrammingLanguage::Java),
            "cpp" | "c++" | "cc" | "cxx" => Some(ProgrammingLanguage::Cpp),
            "go" => Some(ProgrammingLanguage::Go),
            _ => None,
        }
    }

    /// Split document using AST-aware code splitter
    async fn split_with_code_splitter(
        &self,
        document: &Document,
        _code_splitter: &CodeSplitter,
    ) -> WikifyResult<Vec<Node>> {
        debug!("AST-aware code splitting requested, but falling back to token splitter to avoid runtime conflicts");

        // TODO: Fix the async runtime conflict in CodeSplitter
        // For now, fallback to token splitter for code files
        self.split_with_token_splitter(document.clone()).await
    }

    /// Split document using markdown parser
    async fn split_with_markdown_parser(&self, document: Document) -> WikifyResult<Vec<Node>> {
        self.markdown_parser
            .parse_nodes(&[document], false)
            .await
            .map_err(|e| {
                Box::new(WikifyError::Indexing {
                    message: format!("Markdown parsing failed: {}", e),
                    source: Some(Box::new(e)),
                    context: ErrorContext::new("document_indexer").with_operation("markdown_parse"),
                })
            })
    }

    /// Get indexing statistics
    pub fn get_stats(&self) -> IndexingStats {
        IndexingStats {
            chunk_size: self.config.chunk_size,
            chunk_overlap: self.config.chunk_overlap,
            sentence_aware: self.config.sentence_aware,
            token_based_for_code: self.config.token_based_for_code,
            max_tokens_per_chunk: self.config.max_tokens_per_chunk,
        }
    }
}

impl Default for DocumentIndexer {
    fn default() -> Self {
        Self::new().expect("Failed to create default DocumentIndexer")
    }
}

/// Statistics about the indexing process
#[derive(Debug, Clone)]
pub struct IndexingStats {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub sentence_aware: bool,
    pub token_based_for_code: bool,
    pub max_tokens_per_chunk: usize,
}

/// Helper function to create a basic indexer with DeepWiki-compatible settings
pub fn create_deepwiki_compatible_indexer() -> WikifyResult<DocumentIndexer> {
    let config = IndexingConfig {
        chunk_size: 350,    // DeepWiki's chunk size
        chunk_overlap: 100, // DeepWiki's overlap
        sentence_aware: true,
        token_based_for_code: true,
        max_tokens_per_chunk: 250,
        preserve_markdown_structure: true,
        use_ast_code_splitting: true,
    };

    DocumentIndexer::with_config(config)
}
