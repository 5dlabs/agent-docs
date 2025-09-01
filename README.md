# Agent Docs MCP Server

A high-performance MCP (Model Context Protocol) server for Rust crate documentation management and AI-powered code assistance.

## 🚀 Overview

Agent Docs is a comprehensive documentation server that provides AI-powered assistance for Rust development. It combines:

- **MCP Protocol Support** - Standardized AI assistant integration
- **Vector Search** - Semantic search through documentation using embeddings
- **Crate Management** - Automated ingestion and management of Rust crates
- **Multi-format Support** - JSON-RPC and HTTP transport layers
- **Production Ready** - Docker containerization and Kubernetes deployment

## 🏗️ Architecture

This project is organized as a Rust workspace with specialized crates:

### Core Crates

- **`mcp/`** - Main MCP server implementation with HTTP transport
- **`db/`** - PostgreSQL database layer with pgvector integration
- **`embed/`** - OpenAI embedding client for vector operations
- **`llm/`** - LLM client for AI-powered interactions
- **`loader/`** - Data loading and ingestion pipeline

### Key Features

- **13 MCP Tools** - Complete toolset for Rust development assistance
- **Vector Embeddings** - Semantic search through documentation
- **Batch Processing** - Efficient document processing with cost optimization
- **Real-time Updates** - Live documentation synchronization
- **Health Monitoring** - Comprehensive metrics and observability

## 🛠️ Quick Start

### Prerequisites

- Rust 1.70+
- PostgreSQL 15+ with pgvector extension
- Docker (for containerized deployment)

### Local Development

1. **Clone the repository**
   ```bash
   git clone https://github.com/5dlabs/agent-docs.git
   cd agent-docs
   ```

2. **Set up the database**
   ```bash
   # Create PostgreSQL database with pgvector
   createdb agent_docs
   psql agent_docs -c "CREATE EXTENSION vector;"
   ```

3. **Configure environment**
   ```bash
   cp .env.example .env
   # Edit .env with your database URL and API keys
   ```

4. **Run the server**
   ```bash
   cargo run --bin http_server
   ```

The server will start on `http://localhost:3001` with health checks available at `/health`.

### Docker Development

```bash
# Build and run with Docker Compose
docker-compose up --build
```

## 🔧 Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `OPENAI_API_KEY` | OpenAI API key for embeddings | Required |
| `PORT` | Server port | `3001` |
| `RUST_LOG` | Logging level | `info,doc_server=debug` |

### Database Setup

The server requires PostgreSQL with the pgvector extension for vector operations:

```sql
-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector";
```

## 📡 API Usage

### MCP Protocol

The server implements the Model Context Protocol for AI assistant integration:

```json
// Tool discovery
POST /mcp
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}

// Tool execution
POST /mcp
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "rust_query",
    "arguments": {
      "query": "async file operations"
    }
  }
}
```

### Available Tools

1. **`rust_query`** - Search Rust documentation
2. **`birdeye_query`** - BirdEye protocol documentation
3. **`solana_query`** - Solana blockchain documentation
4. **`jupyter_query`** - Jupyter notebook documentation
5. **`cilium_query`** - Cilium networking documentation
6. **`talos_query`** - Talos OS documentation
7. **`meteora_query`** - Meteora DEX documentation
8. **`raydium_query`** - Raydium DEX documentation
9. **`ebpf_query`** - eBPF documentation
10. **`rust_best_practices_query`** - Rust best practices
11. **`add_rust_crate`** - Add new crate to index
12. **`remove_rust_crate`** - Remove crate from index
13. **`list_rust_crates`** - List available crates
14. **`check_rust_status`** - System health check

## 🚢 Deployment

### Docker

```bash
# Build production image
docker build -t agent-docs:latest .

# Run container
docker run -p 3001:3001 \
  -e DATABASE_URL="postgresql://..." \
  -e OPENAI_API_KEY="..." \
  agent-docs:latest
```

### Kubernetes

The project includes production-ready Kubernetes manifests:

```bash
# Deploy to Kubernetes
kubectl apply -f k8s/

# Check deployment status
kubectl get pods -l app=doc-server
```

### CI/CD

GitHub Actions workflow provides:

- **Automated Testing** - Unit and integration tests
- **Code Quality** - Clippy linting and formatting
- **Security Scanning** - Vulnerability assessment
- **Docker Build** - Multi-platform image building
- **Kubernetes Deployment** - Automatic rollout updates

## 🔍 Monitoring

### Health Checks

```bash
# Health endpoint
curl http://localhost:3001/health

# Metrics endpoint
curl http://localhost:3001/metrics
```

### Logs

```bash
# View application logs
kubectl logs -l app=doc-server

# Follow logs in real-time
kubectl logs -f -l app=doc-server
```

## 🧪 Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
# Run with database
TEST_DATABASE_URL="postgresql://..." cargo test --test integration
```

### Performance Testing

```bash
# Run performance benchmarks
./scripts/performance-benchmark.sh

# Cost validation
./scripts/cost-validation.sh
```

## 📚 Development

### Code Organization

```
├── mcp/           # Main MCP server
│   ├── src/
│   │   ├── bin/       # Server binaries
│   │   ├── handlers/  # Request handlers
│   │   ├── tools/     # MCP tool implementations
│   │   └── transport/ # HTTP transport layer
│   └── tests/         # Integration tests
├── db/            # Database layer
├── embed/         # Embedding service
├── llm/           # LLM client
└── loader/        # Data ingestion
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Ensure CI passes
5. Submit a pull request

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run tests with coverage
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
```

## 📄 License

This project is licensed under the terms specified in the [LICENSE](LICENSE) file.

## 🤝 Support

For issues and questions:

- **Issues**: [GitHub Issues](https://github.com/5dlabs/agent-docs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/5dlabs/agent-docs/discussions)
- **Documentation**: See `docs/` directory for detailed guides

## 🔄 Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and updates.

---

**Built with ❤️ using Rust**
