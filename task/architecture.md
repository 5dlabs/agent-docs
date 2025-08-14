# Doc Server Architecture

## Overview

The Doc Server is a comprehensive documentation server that transforms from a single-purpose Rust documentation MCP server into a multi-type documentation platform. The system supports semantic search across diverse technical documentation including infrastructure tools, blockchain platforms, and programming resources.

## Current Implementation Status

### ✅ Completed Infrastructure
- **Database**: Migrated from `rust_docs_vectors` to `docs` with harmonized schema
- **Production Database**: PostgreSQL with pgvector extension in dedicated cluster
- **MCP Server**: Streamable HTTP server (JSON-only MVP) with Toolman integration on port 3001
- **Data Storage**: 184MB database dump with 40+ Rust crates, BirdEye API docs, Solana documentation
- **Embeddings**: 4,000+ documents with OpenAI text-embedding-3-large (3072 dimensions)

### ✅ Working Query Tools
- `rust_query` - Fully implemented and tested in Cursor MCP
- Database contains BirdEye and Solana documentation (ingestion completed)

### 🔄 Next Priority
- Task 35: Project State Evaluation (for new implementation agent)
 - Task 17: Keep-alive/heartbeat without SSE (e.g., periodic ping); SSE intentionally disabled per security policy

## System Architecture

### Core Components

1. **MCP Server** (`crates/mcp/`)
   - Axum-based HTTP server using Streamable HTTP (JSON-only mode); SSE disabled by policy; GET /mcp returns 405
   - Tool registration and request handling
   - Connection management and health checks

2. **Database Layer** (`crates/database/`)
   - PostgreSQL with pgvector extension
   - Harmonized schema supporting multiple doc types
   - SQLx connection pooling

3. **Embeddings Service** (`crates/embeddings/`)
   - OpenAI text-embedding-3-large integration
   - Content truncation and rate limiting
   - Batch processing capabilities (planned)

4. **Document Loaders** (`scripts/ingestion/`)
   - Python scripts for various documentation sources
   - BirdEye API extraction (OpenAPI specs)
   - Solana documentation processing (markdown, PDFs, diagrams)

### Production Infrastructure

- **Database**: PostgreSQL with pgvector extension in dedicated Kubernetes cluster
- **Container Registry**: GitHub Container Registry for automated builds
- **Deployment**: Helm-based Kubernetes deployment with production configuration
- **Data Migration**: 184MB database ready for cluster deployment

## Database Schema

### Harmonized Tables

```sql
-- Primary documents table
CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    doc_type VARCHAR(50) NOT NULL CHECK (doc_type IN (
        'rust', 'jupyter', 'birdeye', 'cilium', 'talos', 
        'meteora', 'solana', 'ebpf', 'raydium', 'rust_best_practices'
    )),
    source_name VARCHAR(255) NOT NULL,
    doc_path TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    embedding vector(3072), -- OpenAI text-embedding-3-large dimensions
    token_count INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE(doc_type, source_name, doc_path)
);

-- Source configuration table
CREATE TABLE document_sources (
    id SERIAL PRIMARY KEY,
    doc_type VARCHAR(50) NOT NULL,
    source_name VARCHAR(255) NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    enabled BOOLEAN DEFAULT true,
    last_checked TIMESTAMPTZ,
    last_populated TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name)
);

-- Performance indexes
CREATE INDEX idx_documents_doc_type ON documents(doc_type);
CREATE INDEX idx_documents_source_name ON documents(source_name);
CREATE INDEX idx_documents_created_at ON documents(created_at DESC);

-- Note: No vector index due to pgvector 2000-dimension limit
-- OpenAI embeddings are 3072 dimensions, queries work but slower
```

### Current Data Contents

- **Rust Documentation**: 40+ crates with complete documentation
- **BirdEye API**: 600+ endpoints with OpenAPI specifications  
- **Solana Documentation**: 400+ documents including:
  - Core documentation (markdown)
  - Architecture diagrams (BOB format)
  - ZK cryptography specifications (PDFs)
  - Sequence charts (MSC format)

## Tool Architecture

### Query Tools (Per Documentation Type)

Each documentation type has its own specific query tool for better relevance:

```rust
// Currently implemented
"rust_query" -> RustQueryTool

// Planned implementations  
"birdeye_query" -> BirdeyeQueryTool
"solana_query" -> SolanaQueryTool
"jupyter_query" -> JupyterQueryTool
"cilium_query" -> CiliumQueryTool
"talos_query" -> TalosQueryTool
"meteora_query" -> MeteoraQueryTool
"raydium_query" -> RaydiumQueryTool
"ebpf_query" -> EbpfQueryTool
"rust_best_practices_query" -> RustBestPracticesQueryTool
```

### Management Tools (Rust Only)

Only Rust crates support dynamic management via MCP tools:

```rust
"add_rust_crate" -> Add new Rust crate
"remove_rust_crate" -> Remove Rust crate  
"list_rust_crates" -> List available crates
"check_rust_status" -> Check population status
```

### Tool Naming Convention

- **Query tools**: `{specific_name}_query`
  - ✅ Good: `solana_query`, `talos_query`, `cilium_query`
  - ❌ Bad: `blockchain_query`, `linux_query`, `docs_query`
- **Management tools**: `{action}_{doctype}_{noun}` (Rust only)
- **General tools**: `{action}_{scope}`

**Key Principle**: Tool names must clearly indicate what specific documentation is available.

## Connection Architecture

### MCP Transport (MVP)

- **Protocol**: Streamable HTTP (MCP 2025-06-18), JSON-only responses for MVP
- **Port**: 3001 (configurable via PORT environment variable)
- **Endpoints**:
  - `/health` - Health check
  - `/mcp` - Tool requests and responses (POST only; GET returns 405)

### Streaming policy

- Transport: Streamable HTTP per MCP 2025-06-18, JSON-only responses for all requests
- `/mcp` GET: 405 Method Not Allowed (no SSE endpoint exposed)
- Accept header: clients may advertise `text/event-stream`, but server returns `application/json`
- Rationale: SSE disabled due to security posture (DNS rebinding surface) and current project scope

### Toolman Integration

- Enhanced health endpoints for monitoring
- Connection lifecycle events
- Graceful degradation on timeout
- Persistent connection management

## Embedding Strategy

### Current Implementation

- **Provider**: OpenAI text-embedding-3-large (3072 dimensions)
- **Processing**: Individual API calls with content truncation
- **Limits**: 30,000 characters per embedding (~7,500 tokens)
- **Storage**: No vector index due to pgvector dimension limits

### Planned Optimizations

- **Batch Processing**: 100 embeddings per request (70% cost reduction)
- **Rate Limiting**: 3,000 RPM / 1M TPM compliance
- **Queue Management**: Async batch processing with retry logic
- **Cost Optimization**: Reduced API calls through intelligent batching

## Document Type Metadata

Each documentation type stores specific metadata in JSONB format:

### Rust Crates
```json
{
  "version": "1.0.0",
  "features": ["full", "macros"],
  "crate_type": "library",
  "docs_url": "https://docs.rs/crate/version"
}
```

### BirdEye API
```json
{
  "api_version": "v1",
  "endpoint": "/defi/price",
  "method": "GET",
  "parameters": {...},
  "response_schema": {...}
}
```

### Solana Documentation  
```json
{
  "category": "core|architecture|crypto",
  "format": "markdown|pdf|bob|msc",
  "section": "consensus|networking|validators",
  "complexity": "beginner|intermediate|advanced"
}
```

## Development Environment

### Quick Start

```bash
# Start with full database dump (recommended)
./scripts/dev.sh --with-data

# Or start with empty database
./scripts/dev.sh

# Stop and optionally clean volumes
./scripts/stop.sh
```

### Production Configuration

- **MCP Server**: Port 3001 (configurable via environment)
- **Health Check**: `/health` endpoint
- **PostgreSQL**: Cluster connection via environment configuration
- **Session Storage**: In-memory (sufficient for single-user with 5-6 agents)

### Environment Variables

```bash
# Required for AI functionality
OPENAI_API_KEY=your_openai_key

# Database connection (auto-configured in dev environment)
DATABASE_URL=postgresql://docserver:password@localhost:5433/docs

# Server configuration
PORT=3001
RUST_LOG=info,doc_server=debug

# Planned batch processing settings
BATCH_SIZE=100
RATE_LIMIT_RPM=3000
```

## Migration History

### From Original Implementation

**Database Changes**:
- `rust_docs_vectors` → `docs` (renamed)
- `doc_embeddings` → `documents` (harmonized schema)
- `crate_configs` → `document_sources` (unified configuration)

**Tool Changes**:
- `query_rust_docs` → `rust_query` (type-specific)
- `add_crate` → `add_rust_crate` (explicit naming)
- `list_crates` → `list_rust_crates` (clear scope)

**Architecture Changes**:
- Single-type → Multi-type documentation support
- Voyage AI → OpenAI only (simplified embedding stack)
- Synchronous → Planned batch processing
- Generic queries → Type-specific tools

### Data Preservation

- ✅ All 40 existing Rust crates preserved and migrated
- ✅ 4,133 embeddings transferred with full fidelity
- ✅ Metadata and relationships maintained
- ✅ Search functionality verified and working

## Future Extensibility

### Adding New Documentation Types

1. **Database**: Add new doc_type to enum constraint
2. **Loader**: Create ingestion script in `scripts/ingestion/`
3. **Tool**: Implement new query tool in `crates/mcp/src/tools.rs`
4. **Registration**: Add tool to handler in `crates/mcp/src/handlers.rs`

### Planned Documentation Types

- **Jupyter Notebooks**: Interactive notebook documentation
- **Infrastructure Tools**: Cilium, Talos, eBPF documentation
- **Blockchain Platforms**: Additional DEX and DeFi protocol docs
- **Best Practices**: Language and framework best practice guides

## Performance Considerations

### Current Performance

- Vector search without index (slower but functional)
- Individual embedding API calls (higher cost)
- Basic connection management (timeout issues)

### Optimization Roadmap

1. **Batch Processing**: 70% cost reduction for embeddings
2. **Connection Reliability**: Streamable HTTP transport implementation  
3. **Query Optimization**: Improved indexing strategies
4. **Caching**: In-memory caching for frequently accessed content
5. **Scaling**: Kubernetes horizontal pod autoscaling

## Security & Compliance

### Current Implementation

- API key management via environment variables
- Database connection encryption
- No sensitive data logging
- PostgreSQL authentication

### Planned Enhancements

- Rate limiting implementation
- Request validation and sanitization
- Audit logging for tool usage
- Enhanced error handling without data exposure

## Monitoring & Observability

### Health Endpoints

- `/health` - Basic service health
- Planned: `/metrics` - Performance metrics
- Planned: `/status` - Detailed system status

### Logging Strategy

- Structured logging with tracing crate
- Debug level for development
- Info level for production
- Error tracking and alerting (planned)

## Deployment Architecture

### Development

- Direct PostgreSQL cluster access
- GitHub Actions workflow for automated builds
- Container image building and registry
- Hot reloading for development

### Production

- Kubernetes deployment with Helm charts
- PostgreSQL with pgvector extension in dedicated cluster
- In-memory session and query caching
- Load balancing via Kubernetes ingress
- Monitoring and alerting with Prometheus/Grafana