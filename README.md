# Doc Server

A high-performance documentation search server built in Rust, providing semantic search across multiple documentation types through the Model Context Protocol (MCP).

## 🎯 Overview

Doc Server aggregates and indexes documentation from various sources, enabling AI assistants to perform semantic search across:

- **Rust Crates** - Documentation from docs.rs
- **Jupyter Notebooks** - Interactive notebook documentation
- **Blockchain APIs** - Solana, BirdEye, Meteora, Raydium documentation
- **Infrastructure Tools** - Cilium, Talos Linux, eBPF guides
- **Best Practices** - Curated Rust development guides

## ✨ Key Features

- 🚀 **High Performance** - Built in Rust with async/await
- 🔍 **Semantic Search** - OpenAI embeddings with pgvector
- 🛠️ **MCP Integration** - Native Model Context Protocol support
- 📊 **Type-Specific Tools** - Dedicated query tools for each documentation type
- ⚡ **Batch Processing** - Optimized OpenAI API usage with 70% cost reduction
- 🔄 **SSE Keep-Alive** - Robust connection management for AI clients
- 🐳 **Container Ready** - Docker and Kubernetes deployment support

## 🏗️ Architecture

### Workspace Structure

```
docs/
├── Cargo.toml              # Workspace configuration
├── src/bin/                # Binaries
│   └── http_server.rs      # Main HTTP/SSE server
├── crates/                 # Individual crates
│   ├── database/           # PostgreSQL + pgvector integration
│   ├── mcp/               # MCP protocol implementation
│   ├── embeddings/        # OpenAI embedding generation
│   ├── doc-loader/        # Document parsing and loading
│   └── llm/               # LLM integration for summarization
├── docs/                  # Documentation
└── .taskmaster/           # Project management
```

### Technology Stack

- **Runtime**: Tokio async runtime
- **Database**: PostgreSQL 15+ with pgvector extension
- **Web Framework**: Axum with Tower middleware
- **Embeddings**: OpenAI text-embedding-3-large (3072 dimensions)
- **Protocol**: Model Context Protocol (MCP) over HTTP/SSE
- **Containerization**: Docker with multi-stage builds

## 🚀 Quick Start

### Prerequisites

- Rust 1.83+ 
- PostgreSQL 15+ with pgvector extension
- OpenAI API key

### Local Development

1. **Clone and setup**:
   ```bash
   git clone <repository-url>
   cd docs
   cp .env.example .env
   # Edit .env with your configuration
   ```

2. **Start database**:
   ```bash
   docker-compose up postgres -d
   ```

3. **Run migrations and start server**:
   ```bash
   cargo run --bin http_server
   ```

### Docker Development

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f doc-server
```

### Production Deployment

```bash
# Build optimized image
docker build -t doc-server:latest .

# Deploy with your orchestrator (Kubernetes, Docker Swarm, etc.)
```

## 🔧 Configuration

### Environment Variables

Key configuration options (see `.env.example` for complete list):

```bash
# Required
DATABASE_URL=postgresql://user:pass@localhost:5432/docs
OPENAI_API_KEY=sk-your-api-key

# Server
PORT=3000
RUST_LOG=info,doc_server=debug

# Optional optimizations
EMBEDDING_BATCH_SIZE=100
VECTOR_SEARCH_LIMIT=50
```

### Database Setup

```sql
-- Enable extensions
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create database
CREATE DATABASE docs;
```

## 🛠️ MCP Tools

The server exposes type-specific query tools for AI assistants:

### Query Tools
- `rust_query` - Search Rust crate documentation
- `jupyter_query` - Search Jupyter notebook content
- `solana_query` - Search Solana blockchain documentation
- `birdeye_query` - Search BirdEye API documentation
- `meteora_query` - Search Meteora DEX documentation
- `raydium_query` - Search Raydium DEX documentation
- `cilium_query` - Search Cilium networking documentation
- `talos_query` - Search Talos Linux documentation
- `ebpf_query` - Search eBPF programming guides
- `rust_best_practices_query` - Search Rust best practices

### Management Tools (Rust only)
- `add_rust_crate` - Dynamically add new Rust crates
- `remove_rust_crate` - Remove existing crates
- `list_rust_crates` - List available crates
- `check_rust_status` - Check crate indexing status

## 🧪 Development

### Building

```bash
# Build all crates
cargo build --workspace

# Build specific crate
cargo build -p doc-server-database

# Build release
cargo build --release --bin http_server
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p doc-server-mcp

# Integration tests
cargo test --test integration
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --all -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

## 📚 Documentation

- **[PROJECT_ARCHITECTURE.md](.taskmaster/docs/PROJECT_ARCHITECTURE.md)** - System architecture and design decisions
- **[IMPLEMENTATION_GUIDE.md](.taskmaster/docs/IMPLEMENTATION_GUIDE.md)** - Detailed implementation guide
- **[CONNECTION_RELIABILITY.md](.taskmaster/docs/CONNECTION_RELIABILITY.md)** - SSE connection management
- **[LOCAL_MIGRATION_PLAN.md](.taskmaster/docs/LOCAL_MIGRATION_PLAN.md)** - Database migration procedures

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes following the coding standards
4. Add tests for new functionality
5. Ensure all tests pass (`cargo test --workspace`)
6. Format code (`cargo fmt --all`)
7. Check lints (`cargo clippy --all -- -D warnings`)
8. Commit your changes (`git commit -m 'Add amazing feature'`)
9. Push to the branch (`git push origin feature/amazing-feature`)
10. Open a Pull Request

### Development Guidelines

- Follow Rust community conventions
- Write comprehensive tests for new features
- Update documentation for API changes
- Use meaningful commit messages
- Ensure backward compatibility when possible

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Model Context Protocol](https://github.com/modelcontextprotocol) for the MCP specification
- [pgvector](https://github.com/pgvector/pgvector) for PostgreSQL vector operations
- [OpenAI](https://openai.com) for embedding models
- [Toolman](https://github.com/5dlabs/toolman) for MCP proxy capabilities