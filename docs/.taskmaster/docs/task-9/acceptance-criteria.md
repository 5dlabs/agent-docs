# Acceptance Criteria: Task 9 - Config-Driven Documentation Query Tools

## Functional Requirements

### 1. Dynamic Tool Implementation

- [ ] JSON config defined and validated (tools: name, docType, title, description, enabled)
- [ ] Tools dynamically registered at startup from config (Rust docs tools remain hardcoded; all others are config-driven)
- [ ] Unified query handler used by all dynamic tools
- [ ] Semantic search using pgvector similarity (<=> operator)
- [ ] Result ranking with relevance scores implemented

### 2. Database Integration

- [ ] Filters documents by `docType` from tool config
- [ ] Vector similarity search functional
- [ ] Metadata JSONB fields parsed when present
- [ ] Query performance < 2 seconds (typical inputs)

### 3. MCP Registration & Transport

- [ ] Tools registered dynamically during server startup
- [ ] Appear in tools/list response with names from config
- [ ] JSON-RPC 2.0 invocation working for each dynamic tool via JSON-only Streamable HTTP (MCP 2025-06-18)
- [ ] Parameter validation for `query` and `limit` (see constraints below)
- [ ] Invalid parameters return JSON-RPC error objects (e.g., code -32602 Invalid params) within an `application/json` response, not HTTP 400

### 4. Response Formatting

- [ ] Source attribution and relevance scores displayed
- [ ] Category-appropriate fields included when present based on stored metadata (e.g., API endpoint/method for API docs)

### 5. Parameter Constraints (Explicit)

- [ ] `query`: non-empty UTF-8 string, trimmed length 1–512 chars
- [ ] `limit`: integer 1–20 (default 10); values outside range produce JSON-RPC error code -32602 with a clear message

## Non-Functional Requirements

### 1. Performance

- [ ] Query response time < 2 seconds
- [ ] Concurrent query handling supported
- [ ] Database connection pooling utilized

### 2. Data Quality

- [ ] All configured `docType`s are searchable
- [ ] Metadata accurately extracted when available
- [ ] No duplicate results in responses
- [ ] Relevance ranking accurate

### 3. Error Handling

- [ ] Graceful handling of missing embeddings
- [ ] Database connection failures handled
- [ ] Invalid query parameters rejected with JSON-RPC error objects (e.g., -32602)
- [ ] Meaningful error messages returned

## Test Cases

### Test Case 1: Basic Query (docType)

**Given**: A configured tool `docs_api_query` with `docType` "api"

**When**: A query "get price" is submitted via that tool

**Then**: Results match the `docType` and include category-appropriate fields

**And**: Response time < 2 seconds

### Test Case 2: Metadata Filtering

**Given**: Multiple versions present in metadata (e.g., api_version)

**When**: Query specifies a metadata filter (e.g., api_version="v1")

**Then**:

- Only matching-version results are returned
- Filtering correctly applied
- No mismatched versions in results

### Test Case 3: Registration from Config

**Given**: Server starts with a config listing `docs_api_query` and `docs_solana_query`

**When**: The MCP client calls `tools/list`

**Then**: Both tools appear and invoke the same unified handler with their respective `docType`

### Test Case 4: Parameter Validation

**Given**: Tool invoked via MCP

**When**: Invalid `limit` (e.g., 100) provided

**Then**:

- A JSON-RPC error response is returned with code -32602 and a clear message
- No database query is executed
- HTTP status remains 200 with an `application/json` JSON-RPC error object (per JSON-only transport)

### Test Case 5: Response Formatting

**Given**: Query returns multiple results

**When**: Results are formatted for output

**Then**:

- Each result includes source attribution and relevance score
- Category-appropriate fields included when present (e.g., endpoint URL, method, parameters for API docs)

## Deliverables

### Code Artifacts

- [ ] JSON config and loader/validation
- [ ] Unified query implementation and db queries
- [ ] Dynamic tool registration code
- [ ] Integration tests in tests/
- [ ] Documentation comments in code

### Documentation

- [ ] Tool usage examples (MCP JSON-RPC request/response)
- [ ] Performance notes (< 2s target)
- [ ] Troubleshooting guide

## Validation Criteria

### Automated Tests

```bash
# Integration tests for dynamic registration and unified handler
cargo test --package doc-server-mcp --test dynamic_tools

# Database-backed query tests (requires test DB)
cargo test --package doc-server-mcp --test query_routing
```

### Manual Validation

1. Query multiple configured `docType`s via their tools
2. Verify metadata extraction accuracy
3. Validate response formatting and relevance
4. Check MCP `tools/list` includes all enabled tools

## Definition of Done

Task 9 is complete when:

1. **Tools implemented**: Config loader, dynamic registration, unified handler operational
2. **Database integrated**: Vector search functional
3. **MCP registered**: Tools accessible via `tools/list` and invocable
4. **Tests passing**: All integration tests pass
5. **Performance met**: < 2 second response time
6. **Documentation complete**: Usage guide and examples provided

## Success Metrics

- 100% of configured `docType`s queryable end-to-end
- Query response time consistently < 2 seconds
- Zero critical bugs in implementation
- Tool usage in production environment

### NFR-0: Code Quality and Automation

- [ ] After adding any new function, run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and fix all warnings before continuing
- [ ] Prior to submission, ensure `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` all pass locally
- [ ] All changes pushed to a feature branch; GitHub Actions must complete successfully (including deployment) before opening a PR
- [ ] If a PR already exists, push updates to the same PR branch rather than creating a new one
