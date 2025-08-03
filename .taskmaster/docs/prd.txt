# Product Requirements Document: Doc Server

## Executive Summary

Transform the existing Rust documentation MCP server into a comprehensive documentation server supporting multiple technical documentation types. The server will provide AI assistants with semantic search capabilities across diverse documentation sources including infrastructure tools, blockchain platforms, and programming resources.

## Current State

- **Database**: `rust_docs_vectors` containing 40 Rust crates with 4,133 embeddings
- **Functionality**: Single-purpose Rust crate documentation search
- **Architecture**: PostgreSQL with pgvector for semantic search
- **Issues**: Connection timeouts, limited to Rust documentation only

## Target State

A unified documentation server with:
- **Database**: Renamed to `docs` 
- **Content**: 10+ documentation types
- **Reliability**: SSE keep-alive for stable Toolman integration
- **Cost Efficiency**: 70% reduction via OpenAI batching

## Functional Requirements

### 1. Query Tools (Primary MCP Interface)

Each documentation type must have its own specific query tool:

- `rust_query` - Query Rust crate documentation
- `jupyter_query` - Query Jupyter notebook documentation
- `birdeye_query` - Query BirdEye blockchain API docs
- `cilium_query` - Query Cilium networking/security docs
- `talos_query` - Query Talos Linux documentation
- `meteora_query` - Query Meteora DEX documentation
- `raydium_query` - Query Raydium DEX documentation
- `solana_query` - Query Solana blockchain docs
- `ebpf_query` - Query eBPF documentation
- `rust_best_practices_query` - Query Rust best practices guide

### 2. Management Tools (Rust Only)

Only Rust crates support dynamic management via MCP:
- `add_rust_crate` - Add new Rust crate
- `remove_rust_crate` - Remove Rust crate
- `list_rust_crates` - List available crates
- `check_rust_status` - Check population status

### 3. Connection Reliability

- SSE heartbeat every 30 seconds
- Connection timeout detection at 90 seconds
- Automatic reconnection with exponential backoff
- Message buffering during disconnection
- Enhanced health monitoring endpoints

### 4. Data Management

- Preserve all existing Rust documentation
- Harmonized schema supporting multiple doc types
- JSONB metadata for type-specific information
- Unified search across all documentation types

## Technical Requirements

### Database Schema

```sql
-- Primary tables
documents (
    doc_type: VARCHAR(50) -- specific type identifier
    source_name: VARCHAR(255)
    doc_path: TEXT
    content: TEXT
    metadata: JSONB
    embedding: vector(3072)
)

document_sources (
    doc_type: VARCHAR(50)
    source_name: VARCHAR(255)
    config: JSONB
    enabled: BOOLEAN
)
```

### Performance Requirements

- Query response time < 2 seconds
- Support 100+ concurrent connections
- Batch embedding processing (100 items/request)
- Rate limiting: 3000 RPM / 1M TPM

### Integration Requirements

- Toolman as primary client
- MCP protocol compliance
- PostgreSQL with pgvector
- OpenAI API for embeddings/LLM

## Non-Functional Requirements

### Scalability
- Support for 100,000+ documents per type
- Horizontal scaling capability
- Efficient vector indexing

### Reliability
- 99.9% uptime target
- Graceful degradation on failures
- Comprehensive error handling

### Security
- API key management
- Database connection encryption
- No sensitive data in logs

### Maintainability
- Clear separation between doc types
- Extensible loader architecture
- Comprehensive logging and monitoring

## Implementation Priorities

### Phase 1: Foundation (Week 1-2)
1. Database migration to `docs`
2. Harmonized schema implementation
3. Preserve existing Rust documentation

### Phase 2: Reliability (Week 3)
1. SSE keep-alive implementation
2. Connection recovery mechanisms
3. Health monitoring improvements

### Phase 3: Query Tools (Week 4-5)
1. Implement all 10 query tools
2. Update tool naming conventions
3. Integration testing with Toolman

### Phase 4: Optimization (Week 6)
1. OpenAI batch processing
2. Performance tuning
3. Cost optimization

## Success Criteria

1. All 40 existing Rust crates remain searchable
2. Zero data loss during migration
3. 90% reduction in connection timeouts
4. 70% cost reduction via batching
5. Support for all 10 documentation types
6. Query performance ≤ 10% degradation

## Constraints

1. Must maintain backward compatibility for Rust queries
2. Database must be renamed from `rust_docs_vectors` to `docs`
3. Only Rust crates support dynamic addition via MCP
4. Other doc types require custom ingestion processes
5. Tool names must be specific, not generic

## Dependencies

- PostgreSQL with pgvector extension
- OpenAI API access
- Toolman for primary integration
- Documentation sources for each type

## Risks

1. **Migration Risk**: Data loss during schema migration
   - Mitigation: Comprehensive backup strategy
   
2. **Performance Risk**: Slower queries with more data
   - Mitigation: Optimized indexing strategies
   
3. **Integration Risk**: Toolman compatibility issues
   - Mitigation: Extensive testing with keep-alive

## Future Considerations

- Additional documentation types can be added
- Vector search optimization as data grows
- Multi-language support potential
- Cross-documentation linking capabilities