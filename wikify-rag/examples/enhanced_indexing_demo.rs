//! Enhanced indexing demonstration
//!
//! This example demonstrates the new enhanced indexing capabilities
//! using cheungfun's advanced AST-aware code splitting.

use cheungfun_core::Document;
use std::collections::HashMap;
use wikify_rag::{
    create_documentation_indexer, create_enhanced_indexer, create_enterprise_indexer,
    EnhancedDocumentIndexer, EnhancedIndexingPipeline,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::init();

    println!("🚀 Enhanced Indexing Demo");
    println!("========================");

    // Demo 1: Enhanced Document Indexer
    demo_enhanced_indexer().await?;

    // Demo 2: Different indexer configurations
    demo_indexer_configurations().await?;

    // Demo 3: Code-specific indexing
    demo_code_indexing().await?;

    println!("\n✅ All demos completed successfully!");
    Ok(())
}

async fn demo_enhanced_indexer() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📚 Demo 1: Enhanced Document Indexer");
    println!("------------------------------------");

    // Create enhanced indexer for code repositories
    let indexer = create_enhanced_indexer()?;

    // Create sample documents
    let documents = create_sample_documents();

    println!("📄 Processing {} sample documents...", documents.len());

    // Index the documents
    let nodes = indexer.index_documents(documents).await?;

    println!("✅ Created {} nodes using enhanced indexing", nodes.len());

    // Get statistics
    let stats = indexer.get_enhanced_stats();
    println!("📊 Enhanced indexing stats:");
    println!("   - Chunking strategy: {}", stats.chunking_strategy_used);
    println!("   - Languages processed: {:?}", stats.languages_processed);

    Ok(())
}

async fn demo_indexer_configurations() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚙️ Demo 2: Different Indexer Configurations");
    println!("-------------------------------------------");

    let documents = create_sample_documents();

    // Test different configurations
    let configs = vec![
        ("Code Repository", create_enhanced_indexer()?),
        ("Enterprise", create_enterprise_indexer()?),
        ("Documentation", create_documentation_indexer()?),
    ];

    for (name, indexer) in configs {
        println!("\n🔧 Testing {} configuration...", name);

        let nodes = indexer.index_documents(documents.clone()).await?;
        let stats = indexer.get_enhanced_stats();

        println!("   - Nodes created: {}", nodes.len());
        println!("   - Strategy: {}", stats.chunking_strategy_used);
        println!("   - Languages: {:?}", stats.languages_processed);
    }

    Ok(())
}

async fn demo_code_indexing() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n💻 Demo 3: Code-Specific Indexing");
    println!("---------------------------------");

    // Create code-specific documents
    let code_documents = create_code_documents();

    let indexer = EnhancedDocumentIndexer::for_code_repository()?;

    println!("📄 Processing {} code documents...", code_documents.len());

    let nodes = indexer.index_documents(code_documents).await?;

    println!("✅ Created {} nodes from code documents", nodes.len());

    // Show some sample nodes
    for (i, node) in nodes.iter().take(3).enumerate() {
        println!("📝 Sample node {}: {} chars", i + 1, node.content.len());
        if let Some(metadata) = &node.metadata {
            if let Some(language) = metadata.get("language") {
                println!("   Language: {:?}", language);
            }
        }
    }

    Ok(())
}

fn create_sample_documents() -> Vec<Document> {
    vec![
        Document::new("# Sample Markdown\n\nThis is a sample markdown document with multiple sections.\n\n## Section 1\n\nContent here.\n\n## Section 2\n\nMore content.")
            .with_metadata("file_type", "text")
            .with_metadata("language", "markdown")
            .with_metadata("file_extension", "md"),
        Document::new("This is a regular text document. It contains multiple sentences. Each sentence provides some information. The enhanced indexer should handle this appropriately.")
            .with_metadata("file_type", "text"),
    ]
}

fn create_code_documents() -> Vec<Document> {
    vec![
        Document::new(
            r#"
// Rust code example
use std::collections::HashMap;

pub struct ExampleStruct {
    pub name: String,
    pub data: HashMap<String, i32>,
}

impl ExampleStruct {
    pub fn new(name: String) -> Self {
        Self {
            name,
            data: HashMap::new(),
        }
    }

    pub fn add_data(&mut self, key: String, value: i32) {
        self.data.insert(key, value);
    }

    pub fn get_data(&self, key: &str) -> Option<&i32> {
        self.data.get(key)
    }
}

fn main() {
    let mut example = ExampleStruct::new("test".to_string());
    example.add_data("key1".to_string(), 42);
    println!("Value: {:?}", example.get_data("key1"));
}
"#,
        )
        .with_metadata("file_type", "code")
        .with_metadata("language", "rust")
        .with_metadata("file_extension", "rs"),
        Document::new(
            r#"
# Python code example
class ExampleClass:
    def __init__(self, name):
        self.name = name
        self.data = {}

    def add_data(self, key, value):
        """Add data to the internal dictionary."""
        self.data[key] = value

    def get_data(self, key):
        """Get data from the internal dictionary."""
        return self.data.get(key)

    def process_data(self):
        """Process all data in the dictionary."""
        result = []
        for key, value in self.data.items():
            if isinstance(value, (int, float)):
                result.append(value * 2)
            else:
                result.append(str(value).upper())
        return result

def main():
    example = ExampleClass("test")
    example.add_data("key1", 42)
    example.add_data("key2", "hello")
    print("Processed:", example.process_data())

if __name__ == "__main__":
    main()
"#,
        )
        .with_metadata("file_type", "code")
        .with_metadata("language", "python")
        .with_metadata("file_extension", "py"),
        Document::new(
            r#"
// JavaScript code example
class ExampleClass {
    constructor(name) {
        this.name = name;
        this.data = new Map();
    }

    addData(key, value) {
        this.data.set(key, value);
    }

    getData(key) {
        return this.data.get(key);
    }

    processData() {
        const result = [];
        for (const [key, value] of this.data.entries()) {
            if (typeof value === 'number') {
                result.push(value * 2);
            } else {
                result.push(String(value).toUpperCase());
            }
        }
        return result;
    }
}

function main() {
    const example = new ExampleClass('test');
    example.addData('key1', 42);
    example.addData('key2', 'hello');
    console.log('Processed:', example.processData());
}

main();
"#,
        )
        .with_metadata("file_type", "code")
        .with_metadata("language", "javascript")
        .with_metadata("file_extension", "js"),
    ]
}
