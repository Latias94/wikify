# Wikify

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

**Wikify** is a self-hosted, open-source tool that automatically generates comprehensive wikis for code repositories using advanced RAG (Retrieval-Augmented Generation) technology.

## 🎯 Project Goals

Wikify aims to replicate and enhance DeepWiki functionality using the Rust ecosystem:

- **🏠 Self-hosted**: Complete control over your data and infrastructure
- **🔓 Open Source**: Transparent, extensible, and community-driven
- **🚀 High Performance**: Built with Rust for speed and reliability
- **🤖 AI-Powered**: Advanced RAG system for intelligent documentation
- **🔌 Extensible**: CLI, API, and MCP integration support

## ✨ Features

### Core Capabilities
- **📚 Automatic Wiki Generation**: Transform any repository into a structured wiki
- **🔍 Intelligent Q&A**: Ask questions about your codebase and get accurate answers
- **🧠 Deep Research**: Multi-iteration research for complex topics
- **📊 Code Analysis**: Understand repository structure and relationships
- **🌐 Multi-Platform**: Support for GitHub, GitLab, Bitbucket, and local repositories

### Technical Features
- **⚡ RAG-Powered**: Built on [cheungfun](https://github.com/YumchaLabs/cheungfun) RAG framework
- **🎛️ Multi-LLM Support**: Unified interface via [siumai](https://crates.io/crates/siumai) (OpenAI, Anthropic, Google, Ollama)
- **🗄️ Local-First**: FAISS-based vector storage with optional distributed options
- **🔄 Streaming**: Real-time response generation
- **📝 Markdown Output**: Beautiful, readable documentation

## 🏗️ Architecture

Wikify follows a modular workspace architecture with **two independent applications**:

```
wikify/
├── wikify-core/          # Core data structures and traits
├── wikify-indexing/      # Document indexing using cheungfun
├── wikify-rag/           # RAG system implementation
├── wikify-wiki/          # Wiki generation engine
├── wikify-cli/           # 🔧 Command-line application (wikify)
└── wikify-web/           # 🌐 Web server application (wikify-web)
```

### **Two Independent Applications**

1. **🔧 wikify** - Developer-focused command-line tool
2. **🌐 wikify-web** - User-friendly web interface

Both applications share the same core libraries but can be deployed and used independently.

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+
- Git
- LLM API key (OpenAI, Anthropic, etc.) or local Ollama setup

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/wikify.git
cd wikify

# Build both applications
cargo build --release

# Install CLI tool (optional)
cargo install --path wikify-cli
```

### Basic Usage

#### **🔧 Command Line Interface**

```bash
# Ask questions about a repository
cargo run --bin wikify -- ask "How does authentication work?"

# Generate wiki documentation
cargo run --bin wikify -- wiki ./my-repo --output ./docs

# Help and options
cargo run --bin wikify -- --help
```

#### **🌐 Web Interface**

```bash
# Start the web server
cargo run --bin wikify-web -- --dev --port 8080

# Or with custom configuration
cargo run --bin wikify-web -- --host 0.0.0.0 --port 3000

# Access the web interface at http://localhost:8080
```

### Configuration

Create `~/.wikify/config.toml`:

```toml
[llm]
provider = "openai"
model = "gpt-4"
api_key = "your-openai-api-key"

[embedding]
provider = "openai"
model = "text-embedding-3-small"
api_key = "your-openai-api-key"

[repository]
max_size_mb = 1000
excluded_dirs = [".git", "node_modules", "target"]
```

## 📖 Usage Examples

### 🔧 CLI Interface

```bash
# Ask questions about your codebase
cargo run --bin wikify -- ask "How does the authentication system work?"

# Generate comprehensive wiki documentation
cargo run --bin wikify -- wiki ./my-project --output ./docs

# Get help and see all options
cargo run --bin wikify -- --help
```

### 🌐 Web Interface

```bash
# Start the development server
cargo run --bin wikify-web -- --dev

# Start production server
cargo run --bin wikify-web -- --host 0.0.0.0 --port 8080

# With custom configuration
cargo run --bin wikify-web -- --config ./config.toml
```

### 🔌 REST API

Once the web server is running, you can use the REST API:

```bash
# Health check
curl http://localhost:8080/api/health

# Initialize a repository
curl -X POST http://localhost:8080/api/repositories \
  -H "Content-Type: application/json" \
  -d '{"repo_path": "/path/to/repo"}'

# Ask questions via API
curl -X POST http://localhost:8080/api/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "How does authentication work?", "session_id": "session-123"}'
```

### Programmatic Usage

```rust
use wikify_core::*;
use wikify_rag::RagSystem;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the system
    let config = WikifyConfig::default();
    let rag_system = RagSystem::new(config).await?;
    
    // Process a repository
    let repo_info = RepoInfo {
        owner: "tokio-rs".to_string(),
        name: "tokio".to_string(),
        repo_type: RepoType::GitHub,
        url: "https://github.com/tokio-rs/tokio".to_string(),
        access_token: None,
        local_path: None,
    };
    
    rag_system.index_repository(&repo_info).await?;
    
    // Ask questions
    let response = rag_system.query("How does Tokio's scheduler work?").await?;
    println!("Answer: {}", response.answer);
    
    Ok(())
}
```

## 🔧 Development

### Building from Source

```bash
# Clone with submodules
git clone --recursive https://github.com/your-org/wikify.git
cd wikify

# Build all components
cargo build

# Run tests
cargo test

# Build specific component
cargo build -p wikify-cli
```

### Development Setup

```bash
# Install development dependencies
cargo install cargo-watch cargo-nextest

# Run with auto-reload
cargo watch -x "run --bin wikify-cli"

# Run tests with nextest
cargo nextest run
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed development guidelines.

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Areas for Contribution

- **Core Features**: RAG improvements, new LLM integrations
- **Visualization**: Code relationship diagrams, interactive charts
- **Integrations**: IDE plugins, CI/CD tools, documentation platforms
- **Performance**: Optimization, caching, distributed processing
- **Documentation**: Examples, tutorials, API documentation

## 📊 Roadmap

### Phase 1: Core Foundation ✅

- [x] Modular workspace architecture
- [x] Core data structures and traits
- [x] Document indexing with cheungfun
- [x] RAG system with siumai integration
- [x] **Independent CLI application (wikify)**
- [x] **Independent Web application (wikify-web)**
- [x] **Professional Web UI with API endpoints**
- [x] **WebSocket support for real-time features**

### Phase 2: Advanced Features 🚧

- [ ] Complete CLI functionality (ask, wiki generation)
- [ ] Repository processing and analysis
- [ ] Advanced code understanding
- [ ] Multi-format wiki export
- [ ] Session management and persistence
- [ ] File tree visualization

### Phase 3: Polish & Extensions 📋

- [ ] Performance optimization and caching
- [ ] Plugin system and extensibility
- [ ] CI/CD integrations
- [ ] Enterprise features (auth, teams)
- [ ] Mobile-responsive UI
- [ ] Advanced visualization and diagrams

## 📄 License

This project is dual-licensed under:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

## 🙏 Acknowledgments

- [DeepWiki](https://github.com/AsyncFuncAI/deepwiki-open) - Original inspiration
- [cheungfun](https://github.com/YumchaLabs/cheungfun) - RAG framework
- [siumai](https://crates.io/crates/siumai) - Unified LLM interface
- [LlamaIndex](https://github.com/run-llama/llama_index) - Design philosophy

## 📞 Community

- **GitHub Issues**: [Bug reports and feature requests](https://github.com/your-org/wikify/issues)
- **Discussions**: [Community discussions](https://github.com/your-org/wikify/discussions)
- **Documentation**: [API docs and guides](https://docs.rs/wikify)

---

*Built with ❤️ in Rust for the developer community*
