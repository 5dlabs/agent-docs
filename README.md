# Agent Docs MCP Server

A high-performance MCP (Model Context Protocol) server for documentation management and AI-powered assistance with any type of technical documentation.

## 🚀 Overview

Agent Docs is a flexible documentation server that provides AI-powered assistance across various technical domains. It combines:

- **MCP Protocol Support** - Standardized AI assistant integration
- **Vector Search** - Semantic search through any documentation using embeddings
- **Configurable Tools** - Customizable toolset for different documentation types
- **Multi-format Support** - JSON-RPC and HTTP transport layers
- **Production Ready** - Docker containerization and Kubernetes deployment

## 🏗️ Architecture

This project is organized as a Rust workspace with specialized crates:

### Core Crates

- `mcp/`: Main MCP server implementation with HTTP transport
- `db/`: PostgreSQL database layer with pgvector integration
- `embed/`: OpenAI embedding client for vector operations
- `loader/`: Data loading and ingestion pipeline

### Key Features

- **Configurable MCP Tools** - Extensible toolset for any documentation type
- **Vector Embeddings** - Semantic search through custom documentation
- **Batch Processing** - Efficient document processing with cost optimization
- **Real-time Updates** - Live documentation synchronization
- **Health Monitoring** - Comprehensive metrics and observability
- **Multi-Domain Support** - APIs, protocols, frameworks, and technical docs

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
   cargo run -p mcp --bin http_server
   ```

The server will start on `http://localhost:3001` with health checks available at `/health`.

### Docker

Build an image with the included Dockerfile and run it with your environment:

```bash
docker build -t agent-docs .
docker run --rm -p 3001:3001 \
  --env-file .env \
  -v $(pwd)/tools.json:/app/tools.json:ro \
  agent-docs
```

Note: The container expects `DATABASE_URL` and (optionally) `OPENAI_API_KEY`. See `.env.example`.

## 🔧 Configuration

### Environment Variables

Key variables used by the server and helpers:

- `DATABASE_URL`: PostgreSQL connection string (required)
- `OPENAI_API_KEY`: OpenAI API key for embeddings (optional if embeddings are not used locally)
- `PORT` or `MCP_PORT`: Server port (defaults to 3001)
- `MCP_HOST`: Bind address (defaults to 0.0.0.0)
- `RUST_LOG`: Logging level (e.g., `info,doc_server=debug`)
- `MCP_ENABLE_SSE`: Enable experimental SSE on `GET /mcp` when set to `1` or `true` (defaults to disabled; `GET /mcp` returns 405). This keeps acceptance tests green while allowing opt-in SSE during development.
  - Note: When enabled, the SSE stream now relays per-session responses and periodic keep‑alives.
- `MCP_ALLOWED_ORIGINS`: Comma-separated list of allowed origins for POST security checks (default allows localhost variants only). Example: `https://cursor.sh,https://your.domain`
- `MCP_STRICT_ORIGIN_VALIDATION`: If `true`, enforce scheme + allow‑list checks on `Origin` header for POST (default: `true`). Set to `false` to be permissive for native clients that send `Origin: null`.
- `MCP_REQUIRE_ORIGIN_HEADER`: If `true`, require `Origin` header on all POST requests (default: `false`).
- `MCP_LOCALHOST_ONLY`: If `true`, restrict server bind validation to localhost (default: `true`). Has effect when using `McpServer::serve`.
- `TOOLS_CONFIG_PATH` or `TOOLS_CONFIG`: Path or inline JSON for tool configuration
- `LOADER_BIN`: Path to the loader binary (defaults to `/app/loader` in container)
- `CLAUDE_BINARY_PATH`: Path to Claude Code binary (defaults to `claude`)

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
    "name": "your_doc_query",
    "arguments": {
      "query": "search term or question",
      "docType": "your_doc_type",
      "limit": 10
    }
  }
}
```

### Intelligent Ingest API

The server provides an asynchronous endpoint to ingest a GitHub repository using the bundled `loader` binary and Claude Code for intelligent discovery.
See also: `docs/llm-roles.md` for the clear separation of LLM responsibilities.

- POST `/ingest/intelligent`
  - Body: `{ "url": "https://github.com/org/repo", "doc_type": "<required>" }`
  - Returns: `{ "job_id": "<uuid>" }`
  - Behavior: enqueues ingestion and returns immediately. The `doc_type` you supply is enforced for DB insertion.

- GET `/ingest/jobs/{job_id}`
  - Returns: `{ status: queued|running|succeeded|failed, started_at, finished_at, error? }`

Requirements
- `DATABASE_URL` must be set for the server
- `LOADER_BIN` defaults to `/app/loader`
- `CLAUDE_BINARY_PATH` defaults to `claude`

### LLM Roles (Summary)

- Claude Code: used only for intelligent document ingestion and discovery (repo analysis and strategy). No fallback to OpenAI.
- OpenAI: used only for embeddings and batch/vector operations (search, similarity). Not used for ingestion/discovery.

For details, refer to `docs/llm-roles.md`.

Example

```bash
curl -s -X POST http://localhost:3001/ingest/intelligent \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://github.com/cilium/cilium","doc_type":"cilium","yes":true}'
# => { "job_id": "8ab7b0a8-…" }

curl -s http://localhost:3001/ingest/jobs/8ab7b0a8-…
# => { "status": "running", … }
```

### Loader CLI

The `loader` binary performs on-host ingestion. Claude Code only proposes a plan; `loader` executes it.

- Intelligent (Claude-guided):
  - Server calls discovery to analyze the repo and generate a plan, then executes steps (typically `git clone` → `loader cli` → `loader database`).
  - `LOADER_BIN` can point to a packaged loader binary used for plan execution.

- CLI (direct parse):
  - `cargo run -p loader -- cli <path> --extensions md,rs,txt,json,yaml,toml --recursive -o ./out`
  - Parses files with the UniversalParser and writes `DocPage` JSON to the output directory. Used by analyzer-generated plans.

- Database (load JSON docs):
  - `cargo run -p loader -- database --input-dir ./out --doc-type <type> --source-name <name> --yes`
  - Inserts previously emitted JSON docs into PostgreSQL.

Note: Legacy `github` and `web` subcommands were removed. Use the intelligent ingest endpoint for repo ingestion and the CLI for local parsing.

### Tool Configuration

The system is highly configurable and can work with **any type of documentation**. Tools are defined in the `tools.json` configuration file, allowing you to:

- **Query Tools**: Search and retrieve information from documentation
- **Management Tools**: Add, remove, and manage documentation sources
- **Custom Documentation Types**: Support any documentation format or source

#### Configuring Tools

Tools are configured in `tools.json` with the following structure:

```json
{
  "tools": [
    {
      "name": "your_doc_query",
      "docType": "your_doc_type",
      "title": "Your Documentation Query",
      "description": "Search your custom documentation...",
      "enabled": true,
      "metadataHints": {
        "supported_formats": ["markdown", "html", "pdf"],
        "supported_categories": ["api", "guides", "examples"],
        "custom_metadata": "any additional configuration"
      }
    }
  ]
}
```

#### Example Tool Types

The system supports various documentation sources:

- **API Documentation** (OpenAPI, GraphQL, REST APIs)
- **Code Documentation** (Rust crates, libraries, frameworks)
- **Technical Documentation** (architecture, deployment, operations)
- **Protocol Documentation** (blockchain, networking, standards)
- **Educational Content** (tutorials, guides, best practices)

#### Built-in Tool Categories

1. **Query Tools** (`*_query`) - Search documentation by type
2. **Management Tools** (`add_*`, `remove_*`, `list_*`) - Manage content
3. **Status Tools** (`check_*_status`) - System health and statistics

Each tool can be:
- ✅ **Enabled/Disabled** individually
- ✅ **Configured** with custom metadata hints
- ✅ **Extended** with new documentation types
- ✅ **Monitored** for performance and usage

### Adding Custom Documentation Types

To add support for new documentation types:

1. **Define the tool** in `tools.json`:
```json
{
  "name": "kubernetes_query",
  "docType": "kubernetes",
  "title": "Kubernetes Documentation Query",
  "description": "Search Kubernetes documentation and best practices",
  "enabled": true,
  "metadataHints": {
    "supported_formats": ["markdown", "yaml"],
    "supported_categories": ["pods", "services", "deployments", "configmaps"],
    "supported_versions": ["v1.24+", "latest"]
  }
}
```

2. **Configure data ingestion** for your documentation source
3. **Restart the server** to load the new configuration
4. **Test the new tool** via MCP protocol

The system automatically creates:
- Query tool: `kubernetes_query`
- Management tools: `add_kubernetes`, `remove_kubernetes`, `list_kubernetes`
- Status tool: `check_kubernetes_status`

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

**Test PR Note**: This is a test pull request to verify CI/CD deployment conditions.

---

**Built with ❤️ using Rust**
# Test CI/CD trigger
