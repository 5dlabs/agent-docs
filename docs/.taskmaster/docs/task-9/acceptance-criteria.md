# Acceptance Criteria: Task 9 - BirdEye Query Tool Implementation

## Functional Requirements

### 1. Tool Implementation
- [ ] BirdEyeQueryTool struct created in `crates/mcp/src/tools.rs`
- [ ] Implements Tool trait with proper definition
- [ ] Semantic search using pgvector similarity (<=> operator)
- [ ] Metadata parsing for BirdEye-specific fields
- [ ] Result ranking with relevance scores implemented
- [ ] Cache mechanism for frequently accessed endpoints

### 2. Database Integration  
- [ ] `birdeye_vector_search` method added to DocumentQueries
- [ ] Filters documents by doc_type='birdeye'
- [ ] Vector similarity search functional
- [ ] Metadata JSONB fields properly parsed
- [ ] Query performance < 2 seconds

### 3. MCP Registration
- [ ] Tool registered in McpHandler::new()
- [ ] Appears in tools/list response
- [ ] JSON-RPC invocation working
- [ ] Parameter validation for query and limit
- [ ] Error handling for invalid requests

### 4. Response Formatting
- [ ] Endpoint details included in responses
- [ ] Example usage generated for each result
- [ ] Parameter descriptions extracted
- [ ] Response schema documented
- [ ] Relevance scores displayed

## Non-Functional Requirements

### 1. Performance
- [ ] Query response time < 2 seconds
- [ ] Cache hit rate > 60% for popular endpoints
- [ ] Concurrent query handling supported
- [ ] Memory usage optimized for cache
- [ ] Database connection pooling utilized

### 2. Data Quality
- [ ] All BirdEye endpoints searchable
- [ ] Metadata accurately extracted
- [ ] No duplicate results in responses
- [ ] Relevance ranking accurate
- [ ] All API versions supported (v1, v2)

### 3. Error Handling
- [ ] Graceful handling of missing embeddings
- [ ] Database connection failures handled
- [ ] Invalid query parameters rejected
- [ ] Meaningful error messages returned
- [ ] Fallback for unavailable cache

## Test Cases

### Test Case 1: Basic Query
**Given**: BirdEye documentation in database
**When**: Query "defi price" submitted
**Then**: 
- Results include price-related endpoints
- Response time < 2 seconds
- Metadata includes endpoint and method

### Test Case 2: Metadata Filtering
**Given**: Multiple API versions present
**When**: Query specifies api_version="v1"
**Then**:
- Only v1 endpoints returned
- Filtering correctly applied
- No v2 endpoints in results

### Test Case 3: Cache Functionality
**Given**: Popular endpoint queried multiple times
**When**: Same query repeated
**Then**:
- Second query faster than first
- Cache hit recorded
- Results identical

### Test Case 4: Parameter Validation
**Given**: Tool invoked via MCP
**When**: Invalid limit (e.g., 100) provided
**Then**:
- Error returned with validation message
- No database query executed
- 400 status code returned

### Test Case 5: Response Formatting
**Given**: Query returns multiple results
**When**: Results formatted for output
**Then**:
- Each result has endpoint URL
- HTTP method specified
- Parameters documented
- Example curl command included

## Deliverables

### Code Artifacts
- [ ] BirdEyeQueryTool implementation in tools.rs
- [ ] Database query methods in queries.rs
- [ ] Cache implementation with TTL
- [ ] Integration tests in tests/
- [ ] Documentation comments in code

### Documentation
- [ ] Tool usage examples
- [ ] API endpoint coverage report
- [ ] Performance benchmarks
- [ ] Cache configuration guide
- [ ] Troubleshooting guide

## Validation Criteria

### Automated Tests
```bash
# Unit tests for tool implementation
cargo test birdeye_query

# Integration tests with database
cargo test --test integration birdeye

# Performance benchmarks
cargo bench birdeye_query
```

### Manual Validation
1. Query various BirdEye endpoints
2. Verify metadata extraction accuracy
3. Test cache effectiveness
4. Validate response formatting
5. Check MCP integration

## Definition of Done

Task 8 is complete when:

1. **Tool fully implemented**: All code components working
2. **Database integrated**: Vector search functional
3. **MCP registered**: Tool accessible via server
4. **Cache operational**: Frequently accessed data cached
5. **Tests passing**: All unit and integration tests pass
6. **Performance met**: < 2 second response time
7. **Documentation complete**: Usage guide and examples provided

## Success Metrics

- 100% of BirdEye endpoints searchable
- Query response time consistently < 2 seconds
- Cache hit rate > 60% in production
- Zero critical bugs in implementation
- Tool usage in production environment### NFR-0: Code Quality and Automation
- [ ] After adding any new function, run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and fix all warnings before continuing
- [ ] Prior to submission, ensure `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` all pass locally
- [ ] All changes pushed to a feature branch; GitHub Actions must complete successfully (including deployment) before opening a PR
- [ ] PR creation is gated on a green CI pipeline and successful deployment of the server artifact
