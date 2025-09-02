//! Integration tests for enhanced indexing functionality

use cheungfun_core::Document;
use cheungfun_indexing::node_parser::config::ChunkingStrategy;
use std::collections::HashMap;
use wikify_rag::{create_enhanced_indexer, EnhancedDocumentIndexer, EnhancedIndexingConfig};

#[tokio::test]
async fn test_enhanced_indexer_creation() {
    // Test that we can create different types of enhanced indexers
    let code_indexer = create_enhanced_indexer().expect("Should create code indexer");
    let enterprise_indexer =
        EnhancedDocumentIndexer::for_enterprise().expect("Should create enterprise indexer");
    let doc_indexer =
        EnhancedDocumentIndexer::for_documentation().expect("Should create documentation indexer");

    // Verify they have different configurations
    let code_stats = code_indexer.get_enhanced_stats();
    let enterprise_stats = enterprise_indexer.get_enhanced_stats();
    let doc_stats = doc_indexer.get_enhanced_stats();

    // They should have different chunking strategies
    assert_ne!(
        code_stats.chunking_strategy_used,
        doc_stats.chunking_strategy_used
    );

    // All should support multiple languages
    assert!(!code_stats.languages_processed.is_empty());
    assert!(!enterprise_stats.languages_processed.is_empty());
}

#[tokio::test]
async fn test_enhanced_indexer_with_text_documents() {
    let indexer = create_enhanced_indexer().expect("Should create indexer");

    let documents = vec![
        Document::new("This is a simple text document. It has multiple sentences. Each sentence should be processed correctly."),
        Document::new("Another document with different content. This one also has multiple sentences for testing purposes."),
    ];

    let nodes = indexer
        .index_documents(documents)
        .await
        .expect("Should index documents");

    // Should create multiple nodes from the documents
    assert!(!nodes.is_empty());
    assert!(nodes.len() >= 2); // At least one node per document

    // Each node should have content
    for node in &nodes {
        assert!(!node.content.trim().is_empty());
    }
}

#[tokio::test]
async fn test_enhanced_indexer_with_code_documents() {
    let indexer =
        EnhancedDocumentIndexer::for_code_repository().expect("Should create code indexer");

    let rust_code = r#"
use std::collections::HashMap;

pub struct TestStruct {
    name: String,
    data: HashMap<String, i32>,
}

impl TestStruct {
    pub fn new(name: String) -> Self {
        Self {
            name,
            data: HashMap::new(),
        }
    }
    
    pub fn add_item(&mut self, key: String, value: i32) {
        self.data.insert(key, value);
    }
}
"#;

    let documents = vec![Document::new(rust_code)
        .with_metadata("file_type", "code")
        .with_metadata("language", "rust")
        .with_metadata("file_extension", "rs")];

    let nodes = indexer
        .index_documents(documents)
        .await
        .expect("Should index code documents");

    // Should create nodes from the code
    assert!(!nodes.is_empty());

    // Nodes should contain code content
    let has_code_content = nodes.iter().any(|node| {
        node.content.contains("struct")
            || node.content.contains("impl")
            || node.content.contains("fn")
    });
    assert!(has_code_content, "Should contain code-related content");
}

#[tokio::test]
async fn test_enhanced_indexer_with_markdown_documents() {
    let indexer =
        EnhancedDocumentIndexer::for_documentation().expect("Should create documentation indexer");

    let markdown_content = r#"
# Main Title

This is the introduction section with some content.

## Section 1

This section contains important information about the topic.

### Subsection 1.1

More detailed information here.

## Section 2

Another section with different content.

- List item 1
- List item 2
- List item 3

### Code Example

```rust
fn main() {
    println!("Hello, world!");
}
```

## Conclusion

Final thoughts and summary.
"#;

    let documents = vec![Document::new(markdown_content)
        .with_metadata("file_type", "text")
        .with_metadata("language", "markdown")
        .with_metadata("file_extension", "md")];

    let nodes = indexer
        .index_documents(documents)
        .await
        .expect("Should index markdown documents");

    // Should create multiple nodes from the structured markdown
    assert!(!nodes.is_empty());

    // Should preserve some markdown structure
    let has_markdown_content = nodes.iter().any(|node| {
        node.content.contains("#")
            || node.content.contains("Section")
            || node.content.contains("List item")
    });
    assert!(
        has_markdown_content,
        "Should contain markdown-related content"
    );
}

#[tokio::test]
async fn test_enhanced_indexer_configuration() {
    // Test custom configuration
    let config = EnhancedIndexingConfig {
        text_chunking_strategy: ChunkingStrategy::Fine,
        code_chunking_strategy: ChunkingStrategy::Optimal,
        chunk_size: 200,
        chunk_overlap: 50,
        enable_ast_code_splitting: true,
        preserve_markdown_structure: true,
        ..Default::default()
    };

    let indexer = EnhancedDocumentIndexer::with_config(config)
        .expect("Should create indexer with custom config");

    let documents = vec![
        Document::new("A simple test document for configuration testing. This should be processed according to the custom configuration settings."),
    ];

    let nodes = indexer
        .index_documents(documents)
        .await
        .expect("Should index with custom config");

    // Should create nodes
    assert!(!nodes.is_empty());

    // Verify configuration is applied
    let stats = indexer.get_enhanced_stats();
    assert!(stats.chunking_strategy_used.contains("Fine"));
    assert!(stats.chunking_strategy_used.contains("Optimal"));
}

#[tokio::test]
async fn test_enhanced_indexer_empty_documents() {
    let indexer = create_enhanced_indexer().expect("Should create indexer");

    let documents = vec![];

    let nodes = indexer
        .index_documents(documents)
        .await
        .expect("Should handle empty document list");

    // Should return empty nodes list
    assert!(nodes.is_empty());
}

#[tokio::test]
async fn test_enhanced_indexer_mixed_document_types() {
    let indexer = create_enhanced_indexer().expect("Should create indexer");

    let documents = vec![
        // Text document
        Document::new("Regular text content for testing purposes."),
        // Code document
        Document::new("fn test() { println!(\"test\"); }")
            .with_metadata("file_type", "code")
            .with_metadata("language", "rust"),
        // Markdown document
        Document::new("# Test\n\nMarkdown content here.").with_metadata("language", "markdown"),
    ];

    let nodes = indexer
        .index_documents(documents)
        .await
        .expect("Should index mixed document types");

    // Should create nodes from all document types
    assert!(!nodes.is_empty());
    assert!(nodes.len() >= 3); // At least one node per document type

    // Should handle different content types appropriately
    let content_types: Vec<String> = nodes.iter().map(|node| node.content.clone()).collect();
    let combined_content = content_types.join(" ");

    assert!(combined_content.contains("Regular text"));
    assert!(combined_content.contains("test") || combined_content.contains("println"));
    assert!(combined_content.contains("Test") || combined_content.contains("Markdown"));
}
