# Toolman Guide: Task 8 - BirdEye Query Tool Implementation

## Overview

This task focuses on implementing the BirdEye query tool for semantic search across BirdEye blockchain API documentation. The selected tools provide file system access for code implementation and the birdeye_query tool for testing once implemented.

## Core Tools

### Filesystem Server Tools

Essential for implementing the query tool code and database integration.

#### read_file
**Purpose**: Examine existing code patterns and structures
**When to Use**:
- Review existing RustQueryTool implementation as template
- Check database query patterns in queries.rs
- Examine tool registration in handlers.rs
**Example**:
```
read_file("/workspace/crates/mcp/src/tools.rs")
```

#### write_file / edit_file
**Purpose**: Implement the BirdEyeQueryTool
**When to Use**:
- Create new tool implementation
- Add database query methods
- Update handler registration
**Example**:
```
edit_file("/workspace/crates/mcp/src/tools.rs", "impl BirdEyeQueryTool...")
```

#### search_files
**Purpose**: Find relevant code locations
**When to Use**:
- Locate tool trait definitions
- Find database query examples
- Search for metadata parsing patterns
**Example**:
```
search_files("*.rs", "Tool trait")
```

### Query Tool (Once Implemented)

#### birdeye_query
**Purpose**: Test the implemented BirdEye documentation search
**When to Use**:
- Validate search functionality after implementation
- Test metadata filtering capabilities
- Verify response formatting
**Parameters**:
- `query`: Search string for BirdEye endpoints
- `limit`: Maximum results (1-20)
- `api_version`: Optional filter for API version

## Implementation Flow

### Phase 1: Code Analysis
1. Use `read_file` to study RustQueryTool implementation
2. Examine database query patterns in queries.rs
3. Review tool registration in handlers.rs
4. Understand metadata structure for BirdEye docs

### Phase 2: Tool Implementation
1. Create BirdEyeQueryTool struct in tools.rs
2. Implement Tool trait with definition
3. Add semantic search functionality
4. Implement metadata parsing for BirdEye fields

### Phase 3: Database Integration
1. Add birdeye_vector_search to queries.rs
2. Implement pgvector similarity search
3. Add metadata filtering logic
4. Optimize query performance

### Phase 4: MCP Registration
1. Register tool in McpHandler::new()
2. Add to tools HashMap
3. Ensure proper error handling
4. Test registration success

### Phase 5: Testing
1. Use birdeye_query tool to test searches
2. Validate metadata extraction
3. Test cache functionality
4. Verify response formatting

## Best Practices

### Code Implementation
- Follow existing RustQueryTool patterns
- Maintain consistent error handling
- Use proper async/await patterns
- Implement comprehensive logging

### Database Queries
- Use parameterized queries for safety
- Implement proper connection pooling
- Handle pgvector operations correctly
- Optimize for performance

### Caching Strategy
- Cache frequently accessed endpoints
- Implement 15-minute TTL
- Use thread-safe HashMap
- Monitor cache hit rates

### Response Formatting
- Include all relevant endpoint details
- Generate useful examples
- Format for readability
- Include relevance scores

## Troubleshooting

### Common Issues

#### Vector Search Failures
- Verify pgvector extension installed
- Check embedding dimensions match (3072)
- Ensure proper vector operators used
- Validate database connectivity

#### Metadata Parsing Errors
- Check JSONB field structure
- Handle missing fields gracefully
- Validate JSON parsing logic
- Test with various metadata formats

#### Performance Issues
- Optimize database queries
- Implement proper indexing
- Use connection pooling
- Enable query caching

#### Registration Problems
- Verify tool name uniqueness
- Check handler initialization
- Validate tool definition JSON
- Ensure proper state management

## Task-Specific Implementation

### BirdEye Metadata Structure
```json
{
  "api_version": "v1",
  "endpoint": "/defi/price",
  "method": "GET",
  "parameters": {
    "address": "string",
    "chain": "solana"
  },
  "response_schema": {
    "price": "number",
    "timestamp": "unix"
  }
}
```

### Query Patterns
1. **Endpoint search**: Find specific API endpoints
2. **Method filtering**: Filter by GET/POST/PUT/DELETE
3. **Version filtering**: Limit to specific API versions
4. **Parameter search**: Find endpoints with specific parameters

### Response Format Example
```json
{
  "results": [
    {
      "endpoint": "/defi/price",
      "method": "GET",
      "description": "Get token price",
      "parameters": [...],
      "example": "curl -X GET ...",
      "relevance": 0.95
    }
  ]
}
```

## Performance Optimization

### Query Optimization
- Use appropriate pgvector indexes
- Limit result set early
- Optimize similarity thresholds
- Batch similar queries

### Cache Implementation
- Use async RwLock for thread safety
- Implement LRU eviction if needed
- Monitor memory usage
- Track cache statistics

## Validation Steps

1. **Unit Tests**: Test each component in isolation
2. **Integration Tests**: Test full query flow
3. **Performance Tests**: Benchmark response times
4. **Load Tests**: Verify concurrent query handling
5. **Acceptance Tests**: Validate against criteria

## Success Indicators

- All BirdEye endpoints searchable
- Response times < 2 seconds
- Cache improving performance
- Accurate metadata extraction
- Proper error handling
- Clean code structure
- Comprehensive test coverage