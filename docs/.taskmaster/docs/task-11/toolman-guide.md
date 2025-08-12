# Toolman Guide: Task 11 - Additional Query Tools Suite Implementation

## Overview

This task implements seven specialized query tools (jupyter, cilium, talos, meteora, raydium, ebpf, rust_best_practices) following the established QueryTool pattern. The focus is on consistency, shared utilities, and domain-specific metadata handling.

## Core Tools

### Filesystem Server Tools

Essential for implementing the query tools suite with consistent patterns and comprehensive testing.

#### read_file
**Purpose**: Study existing implementations and analyze patterns for consistency
**When to Use**: 
- Examine RustQueryTool implementation for exact patterns
- Review database query utilities and patterns
- Study existing metadata parsing approaches
- Analyze shared utility functions in other modules

**Usage Example**:
```
read_file("/workspace/crates/mcp/src/tools.rs")
read_file("/workspace/crates/database/src/queries.rs")
```

#### write_file
**Purpose**: Create new query tool implementations and shared utilities
**When to Use**:
- Implement shared utility module (query_utils.rs)
- Create each of the seven query tool implementations
- Write comprehensive test suites for all tools
- Add database query methods for new documentation types

**Usage Example**:
```
write_file("/workspace/crates/mcp/src/query_utils.rs", "pub fn parse_metadata_field...")
write_file("/workspace/crates/mcp/tests/query_tools_integration.rs", "mod tests...")
```

#### edit_file
**Purpose**: Integrate new tools with existing codebase
**When to Use**:
- Add new tools to MCP handler registration
- Update database queries module with new methods
- Modify existing modules to export new functionality
- Add new tool dependencies to Cargo.toml

**Usage Example**:
```
edit_file("/workspace/crates/mcp/src/handlers.rs", register_all_query_tools)
edit_file("/workspace/crates/mcp/src/lib.rs", "pub mod query_utils;")
```

### Remote Query Tools

#### jupyter_query
**Purpose**: Query Jupyter notebook documentation with cell-type awareness
**When to Use**:
- Searching for Python data science examples
- Finding specific notebook implementations
- Testing metadata filtering by kernel type
- Validating cell output formatting

**Parameters**:
- `query`: Search terms for Jupyter content
- `limit`: Number of results (1-20)
- `kernel`: Filter by kernel type (python/julia/r)
- `cell_type`: Filter by cell type (code/markdown)

#### cilium_query
**Purpose**: Search Cilium networking and security policy documentation
**When to Use**:
- Finding network policy configurations
- Searching for security rule implementations
- Testing Kubernetes integration examples
- Validating policy formatting

**Parameters**:
- `query`: Search terms for Cilium content
- `limit`: Number of results
- `policy_type`: Filter by policy type (ingress/egress)
- `namespace`: Filter by Kubernetes namespace

#### talos_query
**Purpose**: Query Talos Kubernetes documentation and system configuration
**When to Use**:
- Searching for cluster configuration examples
- Finding boot sequence documentation
- Testing system setup procedures
- Validating configuration formatting

**Parameters**:
- `query`: Search terms for Talos content
- `limit`: Number of results
- `resource_type`: Filter by Kubernetes resource type
- `node_config`: Filter by node configuration type

#### meteora_query
**Purpose**: Search Meteora DeFi protocol documentation
**When to Use**:
- Finding liquidity pool strategies
- Searching for yield farming examples
- Testing financial calculation examples
- Validating DeFi parameter formatting

**Parameters**:
- `query`: Search terms for Meteora content
- `limit`: Number of results
- `pool_type`: Filter by liquidity pool type
- `complexity`: Filter by strategy complexity

#### raydium_query
**Purpose**: Query Raydium AMM documentation and trading mechanics
**When to Use**:
- Searching for swap implementation examples
- Finding AMM configuration details
- Testing Solana integration examples
- Validating trading parameter formatting

**Parameters**:
- `query`: Search terms for Raydium content
- `limit`: Number of results
- `amm_version`: Filter by AMM version
- `trading_pair`: Filter by specific trading pairs

#### ebpf_query
**Purpose**: Search eBPF kernel programming documentation
**When to Use**:
- Finding BPF program examples
- Searching for kernel hook implementations
- Testing performance optimization examples
- Validating code formatting and assembly output

**Parameters**:
- `query`: Search terms for eBPF content
- `limit`: Number of results
- `program_type`: Filter by BPF program type
- `kernel_version`: Filter by kernel compatibility

#### rust_best_practices_query
**Purpose**: Query Rust best practices and pattern documentation
**When to Use**:
- Finding code pattern examples
- Searching for best practice recommendations
- Testing anti-pattern documentation
- Validating before/after code examples

**Parameters**:
- `query`: Search terms for Rust practices
- `limit`: Number of results
- `practice_category`: Filter by category (error-handling/concurrency/etc)
- `complexity_level`: Filter by complexity (beginner/intermediate/advanced)

## Implementation Flow

### Phase 1: Shared Infrastructure Development
1. Use `read_file` to study RustQueryTool implementation patterns
2. Create shared utility module with common functions
3. Design consistent metadata parsing approach
4. Implement performance monitoring utilities

### Phase 2: First Batch Implementation (Jupyter & Cilium)
1. Implement JupyterQueryTool with notebook-specific handling
2. Implement CiliumQueryTool with network policy processing
3. Add corresponding database query methods
4. Test integration with MCP handler

### Phase 3: Second Batch Implementation (Talos & Meteora)
1. Implement TalosQueryTool with Kubernetes configuration handling
2. Implement MeteoraQueryTool with DeFi parameter processing
3. Add custom response formatting for technical specifications
4. Test metadata filtering functionality

### Phase 4: Final Batch Implementation (Raydium, eBPF, Rust Practices)
1. Implement RaydiumQueryTool with AMM parameter handling
2. Implement EbpfQueryTool with kernel programming specifics
3. Implement RustBestPracticesQueryTool with pattern comparisons
4. Add caching infrastructure for performance optimization

### Phase 5: Integration and Comprehensive Testing
1. Register all seven tools in MCP handler
2. Create integration tests covering all functionality
3. Validate performance benchmarks
4. Test concurrent usage scenarios

## Best Practices

### Consistency Implementation
- Follow RustQueryTool patterns exactly for all implementations
- Use shared utility functions to maintain consistency
- Implement identical error handling patterns
- Apply consistent logging and monitoring approaches

### Domain-Specific Handling
- Parse metadata fields appropriate to each documentation type
- Format responses optimally for the specific domain
- Handle missing or malformed metadata gracefully
- Provide meaningful defaults for optional parameters

### Performance Optimization
- Implement caching for frequently accessed content
- Use efficient database queries with proper indexing
- Monitor query execution times and optimize bottlenecks
- Batch operations where possible to reduce overhead

### Testing Strategy
- Test each tool independently for core functionality
- Validate metadata filtering across all supported parameters
- Test error scenarios and edge cases thoroughly
- Benchmark performance under various load conditions

## Task-Specific Implementation Guidelines

### 1. Shared Utility Module Structure
```rust
// crates/mcp/src/query_utils.rs
pub fn parse_metadata_field<T>(metadata: &serde_json::Value, field: &str) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    metadata.get(field).and_then(|v| serde_json::from_value(v.clone()).ok())
}

pub fn format_document_response(
    content: &str,
    metadata: &serde_json::Value,
    doc_type: &str,
) -> String {
    // Consistent formatting logic
}
```

### 2. Tool Implementation Pattern
```rust
pub struct JupyterQueryTool {
    db_pool: Arc<DatabasePool>,
    embedding_client: Arc<dyn EmbeddingClient>,
}

impl Tool for JupyterQueryTool {
    fn definition(&self) -> serde_json::Value {
        json!({
            "name": "jupyter_query",
            "description": "Query Jupyter notebook documentation...",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 20},
                    "kernel": {"type": "string", "enum": ["python", "julia", "r"]},
                    "cell_type": {"type": "string", "enum": ["code", "markdown"]}
                },
                "required": ["query"]
            }
        })
    }
}
```

### 3. Database Query Method Pattern
```rust
// In crates/database/src/queries.rs
impl DocumentQueries {
    pub async fn jupyter_vector_search(
        &self,
        query_embedding: &[f32],
        limit: i64,
        filters: &JupyterFilters,
    ) -> Result<Vec<Document>, DatabaseError> {
        let mut query = "
            SELECT d.id, d.content, d.metadata, d.doc_type,
                   e.embedding <=> $1 as similarity
            FROM documents d
            JOIN embeddings e ON d.id = e.document_id
            WHERE d.doc_type = 'jupyter'";
        
        // Add metadata filters based on parameters
        if let Some(kernel) = &filters.kernel {
            query = query + " AND d.metadata->>'kernel' = $n";
        }
        
        // Execute query with proper parameter binding
    }
}
```

### 4. Caching Implementation
```rust
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct QueryCache {
    cache: RwLock<HashMap<String, (String, std::time::Instant)>>,
    ttl: std::time::Duration,
}

impl QueryCache {
    pub async fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().await;
        cache.get(key)
            .filter(|(_, timestamp)| timestamp.elapsed() < self.ttl)
            .map(|(value, _)| value.clone())
    }
}
```

## Troubleshooting

### Common Implementation Issues

#### Metadata Parsing Failures
- Handle missing JSONB fields gracefully with defaults
- Validate metadata structure before parsing
- Log parsing errors with sufficient context
- Provide fallback behavior for malformed data

#### Database Query Performance
- Use proper indexes for metadata JSONB queries
- Optimize vector similarity operations
- Monitor query execution plans
- Implement query result caching

#### Tool Registration Problems
- Verify unique tool names across all implementations
- Check MCP handler initialization order
- Validate tool definition JSON schemas
- Test tool availability through MCP protocol

#### Response Formatting Inconsistencies
- Use shared utility functions consistently
- Test formatting across all documentation types
- Validate markdown output for readability
- Ensure consistent metadata display

### Performance Issues

#### Slow Query Response Times
- Profile database queries with EXPLAIN ANALYZE
- Implement result caching for frequently accessed content
- Optimize embedding similarity calculations
- Monitor memory usage during large result processing

#### Caching Problems
- Verify cache key generation consistency
- Monitor cache hit/miss ratios
- Implement proper cache invalidation
- Handle concurrent cache access safely

## Validation Steps

### Development Testing
1. **Unit Tests**: All utility functions and tool methods
2. **Integration Tests**: Complete query workflows for each tool
3. **Performance Tests**: Response time benchmarks
4. **Error Handling**: Invalid parameter and edge case testing

### System Testing
1. **Concurrent Usage**: Multiple tools used simultaneously
2. **Memory Usage**: Stability during extended operations
3. **Cache Performance**: Repeated query optimization
4. **Database Load**: Performance under query pressure

### Quality Assurance
```bash
# Run these validation commands
cargo test --package mcp query_tools
cargo test --package mcp --test query_tools_integration
cargo clippy --package mcp --lib -- -D warnings
cargo fmt --package mcp -- --check
cargo doc --package mcp --no-deps
```

## Success Indicators

- All seven query tools operational and registered in MCP
- Shared utility module reduces code duplication significantly
- Consistent metadata parsing and response formatting
- Performance targets met for all tools (< 2 seconds)
- Caching improves repeated query performance
- Comprehensive test coverage with all tests passing
- Error handling graceful and informative
- Code quality meets project standards with zero warnings