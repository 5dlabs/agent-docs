# Acceptance Criteria: Streamable HTTP Transport Foundation

## Functional Requirements

### FR-1: Unified MCP Endpoint Implementation (MVP scope)
- [ ] Single `/mcp` endpoint implemented supporting POST method only
- [ ] POST handles JSON-RPC requests with `application/json` content type
- [ ] GET requests return `405 Method Not Allowed`
- [ ] Proper HTTP status codes returned (200 OK, 400 Bad Request, 405 Method Not Allowed)
- [ ] Basic Content-Type validation for requests and responses

### FR-2: MCP Protocol Compliance
- [ ] `MCP-Protocol-Version: 2025-06-18` header included in all responses
- [ ] Protocol version validation for incoming requests
- [ ] `Mcp-Session-Id` header support with UUID v4 session identifiers
- [ ] JSON-RPC message format compliance (request, response, notification)
- [ ] Proper UTF-8 encoding for all message content
- [ ] Error responses follow JSON-RPC 2.0 error specification

### FR-3: Session Management (right-sized)
- [ ] Session creation with unique UUID identifiers
- [ ] Session storage with thread-safe access (Arc<RwLock<HashMap>>)
- [ ] Session expiration after configurable timeout (default 5 minutes)
- [ ] Automatic cleanup of expired sessions
- [ ] Session activity tracking for proper lifecycle management
- [ ] Support for at least 20 concurrent sessions

### (Removed) SSE Streaming and Legacy Compatibility
- SSE streaming and legacy HTTP+SSE compatibility are out of scope for MVP

### FR-6: Integration with Existing MCP Infrastructure
- [ ] Seamless integration with existing `McpHandler` for JSON-RPC processing
- [ ] Preservation of all existing MCP tool functionality
- [ ] CORS configuration maintained for web client compatibility
- [ ] Health check endpoints continue to function
- [ ] Existing error handling and logging infrastructure preserved
- [ ] No modifications required to existing MCP tools

## Non-Functional Requirements

### NFR-1: Performance (scaled for single-user MVP)
- [ ] JSON-RPC request processing latency target: < 200ms (95th percentile)
- [ ] Memory usage appropriate for ~20 agents; no leaks under 30-minute sessions
- [ ] Session cleanup operation < 50ms for typical loads
- [ ] Support for ~20 concurrent connections without degradation

### NFR-2: Reliability (MVP)
- [ ] 99.9% uptime for session management operations
- [ ] Zero memory leaks during 24-hour stress testing
- [ ] Graceful handling of network interruptions
- [ ] Automatic recovery from connection failures
- [ ] Error recovery within 1 second of failure detection
  

### NFR-3: Scalability
- [ ] Horizontal scaling support through stateless design
- [ ] Linear performance scaling with concurrent connection count
- [ ] Memory usage scaling proportional to active session count
- [ ] No hardcoded limits preventing scale-up
- [ ] Efficient session storage with O(1) lookup performance
- [ ] Background cleanup operations don't impact active requests

### NFR-4: Security
- [ ] Session IDs use cryptographically secure UUID generation
- [ ] No sensitive data exposure in error messages or logs
- [ ] Proper input validation for all request parameters
- [ ] Protection against session fixation attacks
- [ ] Rate limiting consideration for session creation
- [ ] Secure session storage preventing unauthorized access

### NFR-5: Maintainability
- [ ] Code follows Rust best practices and style guidelines
- [ ] Comprehensive unit test coverage (>90% line coverage)
- [ ] Clear separation of concerns between transport and business logic
- [ ] Well-documented public API with usage examples
- [ ] Structured logging with appropriate log levels
- [ ] Error messages provide actionable debugging information

## Test Cases

### TC-1: JSON-RPC Request Processing
**Scenario**: Client sends POST request with valid JSON-RPC message
**Given**: MCP server is running with new transport
**When**: POST request sent to `/mcp` with `Content-Type: application/json`
**And**: Request body contains valid JSON-RPC message
**And**: `MCP-Protocol-Version: 2025-06-18` header present
**Then**: Server processes request through existing MCP handler
**And**: Returns 200 OK status with JSON-RPC response
**And**: Response includes `MCP-Protocol-Version` and `Mcp-Session-Id` headers
**And**: Session is created or retrieved for subsequent requests

### TC-2: Session Management
**Scenario**: Multiple requests with same session ID
**Given**: Client has established session with server
**When**: Multiple requests sent with same `Mcp-Session-Id` header
**Then**: All requests use same session context
**And**: Session activity is updated for each request
**And**: Session remains active until timeout period
**And**: Session cleanup occurs after timeout expires

### TC-3: Concurrent Session Handling
**Scenario**: Multiple clients connect simultaneously
**Given**: MCP server supports concurrent connections
**When**: 100 clients connect with different session IDs
**And**: Each client sends JSON-RPC requests
**Then**: All sessions are managed independently
**And**: No session data corruption occurs
**And**: Performance remains within acceptable limits
**And**: Memory usage scales linearly with session count

### TC-4: Error Handling
**Scenario**: Invalid requests and error conditions
**Given**: MCP server with comprehensive error handling
**When**: Malformed JSON-RPC request is sent
**Or**: Invalid session ID format is provided
**Or**: Unsupported protocol version is used
**Then**: Appropriate error response is returned
**And**: Error follows JSON-RPC error format
**And**: No server crash or instability occurs
**And**: Error details logged for debugging

### TC-5: Integration with Existing Tools
**Scenario**: Existing MCP tools continue to function
**Given**: Doc Server with rust_query and management tools
**When**: Client sends requests for existing tool operations
**Through**: New Streamable HTTP transport
**Then**: All tools respond correctly through new transport
**And**: Tool functionality is unchanged
**And**: Response formats remain consistent
**And**: No tool registration changes required

## Deliverables

### D-1: Core Transport Implementation (MVP)
- [ ] `crates/mcp/src/transport.rs` - Complete transport module implementation
- [ ] Session management with thread-safe storage
- [ ] Unified endpoint handler for POST method (GET returns 405)
- [ ] No SSE streaming implementation required
- [ ] No legacy/backward compatibility handling required
- [ ] Integration with existing MCP handler infrastructure

### D-2: Server Integration
- [ ] Updated `crates/mcp/src/server.rs` using new transport
- [ ] Router configuration with unified `/mcp` endpoint
- [ ] Transport configuration management
- [ ] Dependency updates in `Cargo.toml` if required
- [ ] Module declarations in `crates/mcp/src/lib.rs`

### D-3: Test Suite
- [ ] Unit tests for transport components (session management, protocol handling)
- [ ] Integration tests for end-to-end request/response cycles
- [ ] Concurrency tests for multiple simultaneous connections
- [ ] Error handling tests for various failure scenarios
- [ ] Performance benchmarks for latency and throughput
- [ ] Compatibility tests with existing MCP tools

### D-4: Documentation
- [ ] Code documentation with comprehensive doc comments
- [ ] API documentation for transport configuration
- [ ] Migration guide from old transport to new transport
- [ ] Troubleshooting guide for common transport issues
- [ ] Performance tuning recommendations

## Validation Criteria

### V-1: Functional Validation
- [ ] All functional requirements pass automated test suite
- [ ] Manual testing with Cursor MCP client successful
- [ ] Integration testing with Toolman client successful

- [ ] All existing MCP tools function correctly through new transport

### V-2: Performance Validation (right-sized)
- [ ] Validation with 5–6 concurrent agents passes targets
- [ ] Memory stable during 30-minute interactive sessions
- [ ] Latency benchmarks meet adjusted targets
- [ ] Session cleanup time within requirement under typical load

### V-3: Integration Validation
- [ ] Kubernetes deployment successful with new transport
- [ ] Health checks continue to function in production environment
- [ ] Monitoring and logging work correctly with new transport
- [ ] CORS policies function properly for web-based clients
- [ ] Database connectivity and tool operations unaffected

### V-4: Reliability Validation
- [ ] Connection stability testing under various network conditions
- [ ] Graceful handling of client disconnections and reconnections
- [ ] Error recovery testing with simulated failures
- [ ] Session timeout and cleanup functionality verified
- [ ] No data loss or corruption during transport operations

## Definition of Done

**This task is considered complete when:**

1. **All Functional Requirements** (FR-1 through FR-6) are implemented and verified
2. **All Non-Functional Requirements** (NFR-1 through NFR-5) meet specified criteria
3. **All Test Cases** (TC-1 through TC-7) pass in automated test suite
4. **All Deliverables** (D-1 through D-4) are completed and reviewed
5. **All Validation Criteria** (V-1 through V-4) are satisfied through testing
6. **Production Deployment** successful in Kubernetes cluster with new transport
7. **Client Integration** verified with at least two MCP clients (Cursor, Toolman)
8. **Performance Benchmarks** meet all specified performance requirements
9. **Code Review** completed with approval from technical reviewers
10. **Documentation** complete and accessible for future maintenance

**Additional Completion Criteria:**
- Zero critical or high-severity bugs in transport functionality
- Memory usage stable over extended testing periods
- All existing functionality preserved without regression
- Migration path documented and tested
- Monitoring and alerting functional for new transport layer