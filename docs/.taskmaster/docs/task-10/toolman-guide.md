# Toolman Guide: Task 9 - Solana Query Tool Implementation

## Overview

This task implements the SolanaQueryTool for semantic search across Solana blockchain documentation. The selected tools provide file system access for code development and testing of the documentation query capabilities.

## Core Tools

### Filesystem Server Tools

Essential for implementing and testing the Solana query functionality.

#### read_file
**Purpose**: Read and analyze existing code patterns and implementation details
**When to Use**: 
- Examining existing RustQueryTool implementation for patterns
- Reading Solana documentation files for testing
- Analyzing database schema and query utilities
- Reviewing existing tool implementations

**Usage Example**:
```
read_file("/workspace/crates/mcp/src/tools.rs")
```

#### write_file
**Purpose**: Create new implementation files and test cases
**When to Use**:
- Implementing SolanaQueryTool struct and methods
- Creating unit and integration tests
- Writing helper utilities for metadata parsing
- Adding documentation and examples

**Usage Example**:
```
write_file("/workspace/crates/mcp/src/tools.rs", "impl SolanaQueryTool { ... }")
```

#### edit_file
**Purpose**: Modify existing files to integrate new functionality
**When to Use**:
- Adding SolanaQueryTool to existing tools module
- Updating MCP handler registration
- Modifying database queries module
- Adding new dependencies to Cargo.toml

**Usage Example**:
```
edit_file("/workspace/crates/mcp/src/handlers.rs", add_solana_tool_registration)
```

#### list_directory
**Purpose**: Navigate project structure and understand codebase organization
**When to Use**:
- Exploring crates directory structure
- Finding existing test files for pattern reference
- Locating documentation and example files
- Understanding workspace organization

**Usage Example**:
```
list_directory("/workspace/crates/mcp/src")
```

### Remote Tools

#### rust_query
**Purpose**: Query existing Rust documentation for implementation patterns
**When to Use**:
- Understanding existing tool implementations
- Finding code patterns for semantic search
- Testing query functionality during development
- Validating tool integration works correctly

**Parameters**:
- `query`: Search terms for Rust documentation
- `limit`: Number of results to return

#### solana_query
**Purpose**: Test the newly implemented Solana query tool
**When to Use**:
- Validating tool registration and availability
- Testing semantic search functionality
- Verifying metadata filtering works correctly
- Checking format-specific content handling

**Parameters**:
- `query`: Search terms for Solana documentation
- `limit`: Number of results (1-20)
- `format`: Optional filter (markdown/pdf/bob/msc)
- `complexity`: Optional complexity level filter

## Implementation Flow

### Phase 1: Code Analysis and Pattern Understanding
1. Use `read_file` to examine existing RustQueryTool implementation
2. Study database query patterns and utilities
3. Understand MCP tool registration process
4. Review metadata parsing approaches

### Phase 2: Core Implementation
1. Use `write_file` to create SolanaQueryTool struct
2. Implement semantic search with metadata filtering
3. Add content formatting for different document types
4. Create comprehensive Tool trait implementation

### Phase 3: Integration and Registration
1. Use `edit_file` to register tool in MCP handler
2. Update tools module exports
3. Modify database queries if needed
4. Add proper error handling throughout

### Phase 4: Testing and Validation
1. Create unit tests using `write_file`
2. Use `solana_query` tool to test functionality
3. Validate different content format handling
4. Test metadata filtering capabilities

## Best Practices

### Code Development
- Follow existing RustQueryTool patterns exactly
- Use consistent error handling approaches
- Implement proper logging for debugging
- Add comprehensive documentation comments

### Testing Strategy
- Test all metadata filtering combinations
- Validate each content format handling
- Test error scenarios and edge cases
- Verify performance requirements are met

### Integration Approach
- Register tool properly in MCP handler
- Ensure database queries are optimized
- Add appropriate validation for all parameters
- Handle missing metadata gracefully

## Task-Specific Implementation Guidelines

### 1. SolanaQueryTool Structure
```rust
pub struct SolanaQueryTool {
    db_pool: Arc<DatabasePool>,
    embedding_client: Arc<dyn EmbeddingClient>,
}
```

### 2. Metadata Field Handling
- **category**: architecture-diagrams, sequence-diagrams, zk-cryptography
- **format**: markdown, pdf, bob, msc
- **section**: Document section identifier
- **complexity**: Difficulty level filter
- **topic**: Subject matter categorization

### 3. Content Formatting Logic
- **BOB diagrams**: Preserve ASCII structure with monospace formatting
- **MSC charts**: Maintain sequence flow visualization
- **PDF documents**: Extract and display metadata summary
- **Markdown**: Apply proper header hierarchy and code blocks

### 4. Performance Optimization
- Use pgvector similarity search with <=> operator
- Implement result ranking by relevance score
- Cache frequently accessed metadata patterns
- Optimize database query performance

## Troubleshooting

### Common Implementation Issues

#### Database Connection Errors
- Verify database pool initialization
- Check connection string configuration
- Ensure pgvector extension is available
- Test vector similarity operations

#### Metadata Parsing Problems
- Handle missing JSONB fields gracefully
- Validate metadata structure before parsing
- Provide default values for optional fields
- Log parsing errors for debugging

#### Tool Registration Failures
- Verify tool name 'solana_query' is unique
- Check MCP handler initialization order
- Ensure proper error handling in constructor
- Validate tool definition JSON schema

### Performance Issues
- Check vector index usage in queries
- Monitor database connection pool utilization
- Profile memory usage during large result sets
- Optimize metadata filtering query patterns

## Validation Steps

### Development Testing
1. **Compilation**: Ensure code compiles without warnings
2. **Unit Tests**: All SolanaQueryTool methods tested
3. **Integration**: Tool available through MCP protocol
4. **Performance**: Response times under 2 seconds

### Functional Testing
1. **Query Execution**: Test various search patterns
2. **Format Filtering**: Validate each content type
3. **Metadata Filtering**: Test complexity and topic filters
4. **Error Handling**: Verify graceful error responses

### Quality Assurance
```bash
# Run these commands to validate implementation
cargo test --package mcp solana_query
cargo clippy --package mcp --lib
cargo fmt --package mcp --check
cargo doc --package mcp --no-deps
```

## Success Indicators

- SolanaQueryTool registers successfully in MCP server
- Semantic search returns relevant Solana documentation
- All content formats handled appropriately
- Metadata filtering works as specified
- Performance targets consistently met
- Error handling graceful and informative
- Code quality meets project standards