# Doc Server Architecture

## Overview

This project extends the original Rust documentation MCP server to support multiple documentation types. Currently, the database contains only Rust crate documentation (40 crates), but the architecture is designed to support additional types including BirdEye API, Jupyter notebooks, and others.

## Key Architectural Changes

### 1. **Per-Documentation-Type Query Tools**
Instead of a single generic query tool, we expose specific tools for each documentation type:

**Currently Implemented (based on database content):**
- `rust_query` - Query Rust crate documentation (40 crates available)

**Planned Query Tools (as content is added):**
- `birdeye_query` - Query BirdEye blockchain API docs
- `jupyter_query` - Query Jupyter notebook documentation
- Additional types as they are ingested

**Note**: Only Rust crates support dynamic addition via MCP tools (`add_rust_crate`). Other documentation types will be ingested through separate processes, with the model helping determine optimal extraction strategies for each format.

### 2. **Harmonized Database Schema**
Replace separate tables with a unified schema:

```sql
-- Unified documents table replacing doc_embeddings
CREATE TABLE documents (
    id SERIAL PRIMARY KEY,
    doc_type VARCHAR(50) NOT NULL, -- 'rust', 'birdeye', 'jupyter', etc.
    source_name VARCHAR(255) NOT NULL, -- crate name, repo name, etc.
    doc_path TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB, -- Flexible storage for type-specific data
    embedding vector(3072),
    token_count INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name, doc_path)
);

-- Configuration for all documentation sources
CREATE TABLE document_sources (
    id SERIAL PRIMARY KEY,
    doc_type VARCHAR(50) NOT NULL,
    source_name VARCHAR(255) NOT NULL,
    config JSONB NOT NULL, -- Type-specific configuration
    enabled BOOLEAN DEFAULT true,
    last_checked TIMESTAMPTZ,
    last_populated TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name)
);

-- Indexes for performance
CREATE INDEX idx_documents_doc_type ON documents(doc_type);
CREATE INDEX idx_documents_source_name ON documents(source_name);
CREATE INDEX idx_documents_doc_type_source ON documents(doc_type, source_name);
```

### 3. **OpenAI-Only Embeddings with Batching**
- Remove Voyage AI support
- Implement batch processing for OpenAI embeddings API
- Batch size: 100 chunks per request
- Rate limiting: 3000 RPM / 1M TPM
- Cost optimization through batching

### 4. **Document Type Metadata**
Each document type stores specific metadata in JSONB:

**Rust Crates:**
```json
{
  "version": "1.0.0",
  "features": ["full", "macros"],
  "crate_type": "library"
}
```

**BirdEye API:**
```json
{
  "api_version": "v1",
  "endpoint": "/defi/price",
  "method": "GET",
  "parameters": {...}
}
```

**Jupyter Notebooks:**
```json
{
  "kernel": "python3",
  "notebook_version": "6.5.2",
  "cell_types": ["code", "markdown"]
}
```

## Implementation Components

### Core Modules

1. **server.rs** - MCP request router with tool registration
2. **query_engine.rs** - Unified query handler for all doc types
3. **embeddings.rs** - OpenAI client with batching support
4. **batch_processor.rs** - Queue and batch management
5. **database.rs** - Unified database operations
6. **connection/keepalive.rs** - SSE keep-alive and timeout management
7. **connection/recovery.rs** - Connection resilience and auto-recovery

### Document Loaders

1. **rust_loader.rs** - Existing docs.rs HTML parser
2. **github_loader.rs** - GitHub API client for README/docs
3. **json_loader.rs** - Generic JSON/YAML parser
4. **markdown_loader.rs** - Markdown document parser

### MCP Tools Implementation

```rust
// Example tool registration in server.rs
impl RustDocsServer {
    pub fn register_tools(&mut self) {
        // Query tools - one per doc type
        self.register_tool("rust_query", rust_query_handler);
        self.register_tool("birdeye_query", birdeye_query_handler);
        self.register_tool("jupyter_query", jupyter_query_handler);
        self.register_tool("github_query", github_query_handler);
        self.register_tool("openapi_query", openapi_query_handler);
        
        // Rust crate management (dynamic addition)
        self.register_tool("add_rust_crate", add_rust_crate_handler);
        self.register_tool("remove_rust_crate", remove_rust_crate_handler);
        self.register_tool("list_rust_crates", list_rust_crates_handler);
        self.register_tool("check_rust_status", check_rust_status_handler);
        
        // Cross-documentation tools
        self.register_tool("list_doc_types", list_doc_types_handler);
        self.register_tool("search_all", search_all_handler);
    }
}
```

## Data Migration Strategy

1. **Preserve existing Rust documentation**:
   ```sql
   INSERT INTO documents (doc_type, source_name, doc_path, content, embedding, token_count, created_at)
   SELECT 'rust', crate_name, doc_path, content, embedding, token_count, created_at
   FROM doc_embeddings;
   ```

2. **Migrate crate configurations**:
   ```sql
   INSERT INTO document_sources (doc_type, source_name, config, enabled)
   SELECT 'rust', name, 
          jsonb_build_object(
              'version_spec', version_spec,
              'features', features,
              'expected_docs', expected_docs
          ),
          enabled
   FROM crate_configs;
   ```

## Deployment Considerations

1. **Environment Variables**:
   - `OPENAI_API_KEY` - Required
   - `DATABASE_URL` - PostgreSQL connection
   - `BATCH_SIZE` - Default: 100
   - `RATE_LIMIT_RPM` - Default: 3000

2. **Kubernetes Updates**:
   - Update Helm values for new configuration
   - Ensure pgvector extension supports 3072 dimensions
   - Configure resource limits for batch processing

## Connection Reliability

### SSE Keep-Alive System
- Heartbeat messages every 30 seconds
- Connection timeout detection (90 seconds)
- Automatic activity monitoring
- Non-intrusive SSE comments for heartbeats

### Recovery Mechanisms
- Client-side automatic reconnection
- Exponential backoff retry logic
- Message buffering during disconnection
- Connection state tracking

### Toolman Integration
- Enhanced health endpoints for monitoring
- Connection lifecycle events
- Graceful degradation on timeout
- Persistent connection management

See [CONNECTION_RELIABILITY.md](CONNECTION_RELIABILITY.md) for detailed implementation.

## Future Extensibility

The architecture supports easy addition of new documentation types:

1. Create a new loader module (e.g., `confluence_loader.rs`)
2. Add the doc type to the enum
3. Register a new query tool (e.g., `confluence_query`)
4. Document ingestion remains manual/agent-driven

## Tool Naming Convention

- Query tools: `{doctype}_query`
- Management tools: `{action}_{doctype}_{noun}`
- General tools: `{action}_{scope}`

This provides clarity for AI assistants and maintains consistency across the API surface.