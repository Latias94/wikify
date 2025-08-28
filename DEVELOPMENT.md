# Wikify Development Documentation

## Project Overview

**Wikify** is a Rust-based implementation that replicates DeepWiki functionality, providing automated wiki generation for code repositories with intelligent Q&A capabilities.

### Core Objectives

- **Self-hosted open-source solution** without enterprise features initially
- **RAG-powered documentation generation** using cheungfun framework
- **Multi-LLM support** through siumai integration
- **CLI and API interfaces** for maximum flexibility
- **MCP integration support** for extensibility

## Architecture Overview

### Workspace Structure

```
wikify/
├── wikify-core/          # Core data structures and traits
├── wikify-repo/          # Repository processing and cloning
├── wikify-indexing/      # Document indexing using cheungfun
├── wikify-analysis/      # Code analysis and visualization
├── wikify-rag/           # RAG system implementation
├── wikify-web/           # Web API server (optional)
├── wikify-cli/           # Command-line interface
└── repo-ref/             # Reference implementations
    ├── deepwiki-open/    # Original DeepWiki reference
    ├── cheungfun/        # RAG framework
    └── siumai/           # LLM integration library
```

### Key Dependencies

- **cheungfun**: RAG framework (git dependency)
- **siumai v0.9.0**: Unified LLM interface
- **tokio**: Async runtime
- **git2**: Git operations
- **axum**: Web framework (optional)
- **clap**: CLI framework (optional)

## Core Components Analysis

### 1. Repository Processing Strategy

Based on DeepWiki analysis, our approach:

**Storage Pattern:**
- Local repos: `~/.wikify/repos/{owner}_{repo_name}`
- Database cache: `~/.wikify/databases/{owner}_{repo_name}.db`
- Wiki cache: `~/.wikify/wikicache/{owner}_{repo_name}.json`

**Processing Pipeline:**
1. **Repository Detection** - GitHub/GitLab/Bitbucket/Local
2. **Authentication** - Access token support for private repos
3. **Cloning** - Shallow clone with `--depth=1 --single-branch`
4. **File Filtering** - Extensive exclusion rules (see config)
5. **Document Reading** - Priority: code files first, then docs
6. **Token Validation** - Max 8192 tokens per file
7. **Metadata Extraction** - File type, language, importance

### 2. Indexing Strategy

**Text Splitting Configuration:**
- Chunk size: 350 tokens
- Overlap: 100 tokens
- Split by: word boundaries

**Embedding Configuration:**
- Default: OpenAI text-embedding-3-small
- Dimensions: 256
- Batch size: 500

**Vector Storage:**
- Local: FAISS index
- Retrieval: top_k=20

### 3. RAG System Design

**Query Processing:**
1. **Query Embedding** - Convert user question to vector
2. **Document Retrieval** - FAISS similarity search
3. **Context Assembly** - Combine relevant documents
4. **LLM Generation** - Generate answer with context
5. **Response Formatting** - Markdown output

**Deep Research Feature:**
- Multi-iteration research process (max 5 iterations)
- Research plan → Initial findings → Updates → Final conclusion
- Maintains focus on specific topic throughout iterations
- Conversation history preservation

## Implementation Plan

### Phase 1: Core Foundation (4-6 weeks)

#### Week 1-2: Core Infrastructure
- [ ] Complete `wikify-core` with all data structures
- [ ] Implement `wikify-repo` with Git operations
- [ ] Basic configuration management
- [ ] Error handling framework

#### Week 3-4: Indexing System
- [ ] Integrate cheungfun for document processing
- [ ] Implement file filtering and reading
- [ ] Vector embedding and storage
- [ ] Basic retrieval functionality

#### Week 5-6: RAG System
- [ ] Integrate siumai for LLM operations
- [ ] Implement basic Q&A functionality
- [ ] Conversation history management
- [ ] CLI interface development

### Phase 2: Advanced Features (3-4 weeks)

#### Week 7-8: Deep Research
- [ ] Multi-iteration research logic
- [ ] Research state management
- [ ] Advanced prompt engineering
- [ ] Result synthesis

#### Week 9-10: Web Interface
- [ ] REST API with axum
- [ ] WebSocket support for streaming
- [ ] Basic web frontend (optional)
- [ ] API documentation

### Phase 3: Polish & Optimization (2-3 weeks)

#### Week 11-12: Performance & Testing
- [ ] Performance optimization
- [ ] Comprehensive testing
- [ ] Documentation completion
- [ ] CI/CD setup

#### Week 13: Release Preparation
- [ ] Package publishing
- [ ] Example projects
- [ ] User documentation
- [ ] Community setup

## Key Design Decisions

### 1. Vector Database Choice
**Decision**: Start with local FAISS, add distributed options later
**Rationale**: 
- Simplicity for self-hosted deployment
- No external dependencies
- Good performance for moderate-sized repositories
- Easy to extend with Qdrant/other options

### 2. LLM Integration Strategy
**Decision**: Use siumai for unified LLM interface
**Rationale**:
- Multi-provider support (OpenAI, Anthropic, Google, Ollama)
- Type-safe parameter handling
- Streaming support
- Tool calling capabilities

### 3. Repository Processing
**Decision**: Clone to local filesystem, then process
**Rationale**:
- Consistent with DeepWiki approach
- Enables offline processing
- Better performance for repeated operations
- Supports incremental updates

### 4. Configuration Management
**Decision**: TOML-based configuration with sensible defaults
**Rationale**:
- Human-readable format
- Good Rust ecosystem support
- Easy to version control
- Flexible override system

## Development Guidelines

### Code Style
- Use English comments and documentation
- Follow Rust naming conventions
- Comprehensive error handling with `thiserror`
- Async-first design with `tokio`
- Modular architecture with clear separation of concerns

### Testing Strategy
- Unit tests for core logic
- Integration tests for end-to-end workflows
- Performance benchmarks for critical paths
- Example-driven documentation

### Documentation Requirements
- Comprehensive API documentation
- Usage examples for each module
- Architecture decision records
- Performance characteristics

## Monitoring & Observability

### Logging Strategy
- Structured logging with `tracing`
- Configurable log levels
- Performance metrics collection
- Error tracking and reporting

### Metrics to Track
- Repository processing time
- Indexing throughput
- Query response time
- Memory usage patterns
- Cache hit rates

## Security Considerations

### Access Token Handling
- Secure storage of API keys
- Token validation and refresh
- Audit logging for access attempts

### Data Privacy
- Local-first approach
- No data transmission to external services (except LLM APIs)
- Configurable data retention policies

## Future Enhancements

### Planned Features
- [ ] Distributed vector storage (Qdrant integration)
- [ ] Advanced visualization (architecture diagrams)
- [ ] Multi-language support
- [ ] Plugin system for custom processors
- [ ] Enterprise features (authentication, multi-tenancy)

### Integration Opportunities
- [ ] MCP (Model Context Protocol) support
- [ ] IDE extensions (VS Code, IntelliJ)
- [ ] CI/CD pipeline integration
- [ ] Documentation hosting platforms

## Getting Started

### Prerequisites
- Rust 1.70+
- Git
- OpenAI API key (or other LLM provider)

### Quick Start
```bash
# Clone the repository
git clone <repo-url>
cd wikify

# Build the project
cargo build

# Run CLI
cargo run --bin wikify-cli -- --help

# Run tests
cargo test
```

### Configuration
Create `~/.wikify/config.toml`:
```toml
[llm]
provider = "openai"
model = "gpt-4"
api_key = "your-api-key"

[embedding]
provider = "openai"
model = "text-embedding-3-small"
api_key = "your-api-key"
```

This documentation will be updated as the project evolves.
