# Autonomous Agent Prompt: Implement Streamable HTTP Transport Foundation

> IMPORTANT: At the end of this job, submit a pull request with your changes. Create a branch, open a PR to `main`, include a clear title and summary, and link back to `docs/.taskmaster/docs/task-3/task.md`.

## Process and CI Requirements

- Always review the existing implementation first to determine whether the task is already completed or needs improvements/refactors rather than re-implementation.
- Before opening a PR:
  - Ensure your feature branch and `main` are both up to date:
    - `git fetch --all --prune`
    - Rebase/merge latest `origin/main` into your feature branch
  - Push the feature branch to trigger CI and wait for it to complete successfully.
  - CI must pass all checks: formatting, tests, Clippy, and Clippy Pedantic.
    - Formatting: `cargo fmt --all -- --check`
    - Lints: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
    - Tests: `cargo test --all-features`
- Follow repository standards at all times:
  - GitHub workflow conventions in `github-guidelines.md` (also see `docs/github-guidelines.md`).
  - Rust code conventions in `coding-guidelines.md` (also see `docs/coding-guidelines.md`).
- Generate a Markdown report at the end named `task-3-runtime-report.md` (commit it under `docs/.taskmaster/docs/task-3/`):
  - Note any tool-call issues (e.g., failed external tools, API timeouts) and dependency problems.
  - Provide actionable suggestions to improve reliability and runtime performance.
  - Include links to relevant logs or CI runs where applicable.

## Mission

You are tasked with implementing the critical migration from deprecated HTTP+SSE transport to the new Streamable HTTP transport following MCP 2025-06-18 specification. This task is essential for maintaining compatibility with modern MCP clients and ensuring reliable communication for the Doc Server project.

## Context

The current Doc Server implementation uses deprecated HTTP+SSE transport (protocol version 2024-11-05) that is no longer supported. You must implement the new Streamable HTTP transport (protocol version 2025-06-18) to ensure compatibility with Toolman, Cursor, and other modern MCP clients.

## Primary Objectives

1. **Create Core Transport Module (MVP)**: Implement `crates/mcp/src/transport.rs` with Streamable HTTP POST-only support using Axum 0.7
2. **Unified MCP Endpoint (MVP)**: Create single `/mcp` endpoint supporting POST (JSON) only; return 405 for GET
3. **No Streaming**: Skip SSE streaming for MVP; clients will receive single JSON responses
4. **No Legacy Support**: Require `MCP-Protocol-Version: 2025-06-18`; reject others with 400
5. **Server Integration**: Wire new transport into existing MCP server infrastructure, removing any `/sse` references

## Step-by-Step Implementation

### Step 1: Foundation Setup

1. Create `crates/mcp/src/transport.rs` with core imports:
   ```rust
    use axum::{
        extract::{Request, State},
        http::{HeaderMap, Method, StatusCode},
        response::Response,
        routing::post,
        Json, Router
    };
   use serde_json::{json, Value};
   use std::collections::HashMap;
   use std::sync::{Arc, RwLock};
   use std::time::{Duration, Instant};
    use tokio::sync::broadcast;
   use uuid::Uuid;
   ```

2. Define core transport types:
   ```rust
   #[derive(Clone, Debug)]
   pub struct TransportConfig {
       pub protocol_version: String,
       pub session_timeout: Duration,
       pub heartbeat_interval: Duration,
   }
   
   pub type SessionId = Uuid;
   
   #[derive(Debug, Clone)]
   pub struct McpSession {
       pub id: SessionId,
       pub created_at: Instant,
       pub last_activity: Arc<RwLock<Instant>>,
       pub message_sender: broadcast::Sender<SseMessage>,
   }
   ```

3. Add protocol constants:
   ```rust
   pub const MCP_PROTOCOL_VERSION: &str = "MCP-Protocol-Version";
   pub const MCP_SESSION_ID: &str = "Mcp-Session-Id";
   pub const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";
    // MVP: Only latest supported
   ```

### Step 2: Session Management

1. Implement `SessionManager` struct with thread-safe session storage
2. Create session creation, retrieval, and cleanup methods
3. Add automatic session expiration based on `session_timeout`
4. Implement session activity tracking for proper cleanup

### Step 3: Unified MCP Endpoint

1. Create `unified_mcp_handler` function accepting both POST and GET requests
2. Implement protocol version extraction from headers with validation
3. Add session ID extraction or generation logic
4. Route requests based on HTTP method and Accept headers:
   - POST with `application/json` → JSON-RPC request processing
   - GET with `text/event-stream` → SSE stream initialization

### Step 4: JSON-RPC Processing

1. Implement `handle_json_rpc_request` for POST requests
2. Parse JSON-RPC messages from request body
3. Process through existing MCP handler infrastructure
4. Return proper JSON-RPC responses with required headers:
   - `MCP-Protocol-Version: 2025-06-18`
   - `Mcp-Session-Id: {session_uuid}`
   - `Content-Type: application/json`

### Step 5: SSE Streaming

1. Implement `handle_sse_stream_request` for GET requests
2. Validate `Accept: text/event-stream` header
3. Create or retrieve session for the request
4. Set up SSE stream with proper event formatting:
   - Event ID for resumability
   - Proper JSON-RPC message encoding
   - Heartbeat messages every 30 seconds
5. Handle stream cleanup on client disconnect

### Step 6: Backward Compatibility

1. Create `detect_legacy_transport` function checking for:
   - Missing `MCP-Protocol-Version` header
   - Protocol version `2024-11-05`
2. Implement `handle_legacy_transport` returning appropriate errors:
   - Status: 426 Upgrade Required
   - JSON response with upgrade instructions
3. Add logging for legacy transport detection

### Step 7: Server Integration

1. Update `crates/mcp/src/server.rs` to use new transport:
   - Replace `/mcp` POST endpoint with unified handler
   - Update `/sse` endpoint or redirect to unified handler
   - Add transport configuration to server state
2. Update router creation with new endpoint:
   ```rust
   Router::new()
       .route("/mcp", post(unified_mcp_handler).get(unified_mcp_handler))
       .route("/health", get(health_check))
       .layer(CorsLayer::permissive())
   ```
3. Initialize transport manager in server constructor

### Step 8: Error Handling

1. Define transport-specific error types:
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum McpTransportError {
       #[error("Protocol version not supported: {0}")]
       UnsupportedProtocolVersion(String),
       #[error("Session not found: {0}")]
       SessionNotFound(Uuid),
       #[error("Invalid session ID: {0}")]
       InvalidSessionId(String),
   }
   ```

2. Implement proper error responses for each error type
3. Add structured logging for debugging transport issues

## Required Tools

**Use these tools in this specific order:**

1. **read_file**: Read existing MCP server files to understand current architecture
   - `crates/mcp/src/server.rs` - Current server implementation
   - `crates/mcp/src/handlers.rs` - Existing MCP handlers
   - `crates/mcp/src/lib.rs` - Module structure

2. **create_directory**: Ensure proper directory structure exists
   - Verify `crates/mcp/src/` directory
   - Create test directories if needed

3. **write_file**: Create new transport module
   - `crates/mcp/src/transport.rs` - Main transport implementation
   - Update `crates/mcp/src/lib.rs` to include new module

4. **edit_file**: Update existing files for integration
   - Modify `crates/mcp/src/server.rs` to use new transport
   - Update `Cargo.toml` dependencies if needed
   - Modify any related configuration files

5. **write_file**: Create comprehensive tests
   - `crates/mcp/tests/transport_tests.rs` - Integration tests
   - Unit tests within transport module

## Key Integration Points

1. **Existing MCP Handler**: Preserve all existing JSON-RPC processing logic
2. **CORS Configuration**: Maintain existing CORS setup for web clients
3. **Health Endpoints**: Keep existing health check functionality
4. **Error Types**: Integrate with existing error handling infrastructure
5. **Logging**: Use existing tracing setup for structured logging

## Success Criteria

### Functional Requirements (MVP)
- [ ] Single `/mcp` endpoint supports POST only; GET returns 405
- [ ] Proper `MCP-Protocol-Version: 2025-06-18` header handling
- [ ] Session management with UUID-based session IDs
- [ ] JSON-RPC request/response cycle maintained

### Technical Requirements
- [ ] All existing MCP tools continue to work without modification
- [ ] No breaking changes to JSON-RPC message handling
- [ ] Memory-safe session management with automatic cleanup
- [ ] Thread-safe concurrent request handling
- [ ] Proper UTF-8 encoding for all message types

### Integration Requirements
- [ ] Successful testing with Cursor MCP client
- [ ] Compatible with Toolman integration requirements
- [ ] Health checks continue to function properly
- [ ] CORS policies maintained for web-based clients
- [ ] Existing logging and monitoring preserved

## Testing Strategy

1. **Unit Tests**: Test each transport component independently
   - Session management (creation, cleanup, expiration)
   - Protocol version detection and validation
   - Message serialization and deserialization
   - Error handling for various failure scenarios

2. **Integration Tests**: Test complete request/response cycles
   - POST requests with JSON-RPC messages
   - 405 on GET to /mcp
   - Session tracking across multiple requests
   - Concurrent session handling

3. **Compatibility Tests**: Verify client integration
   - Cursor MCP client connection and tool usage
   - Protocol version fixed handling

4. **Performance Tests**: Validate under load
   - Multiple concurrent sessions (~20 connections)
   - Session cleanup performance
   - Memory usage under various loads

## Critical Implementation Notes

1. **Thread Safety**: All session management must be thread-safe using `Arc<RwLock<>>`
2. **Memory Management**: Implement proper session cleanup to prevent memory leaks
3. **Protocol Compliance**: Strict adherence to MCP 2025-06-18 specification
4. **Error Handling**: Comprehensive error responses with helpful debugging information
5. **Backward Compatibility**: Out of scope for MVP

## Focus Areas

- **Reliability**: Ensure stable connections and message delivery
- **Performance**: Minimize latency and memory usage
- **Compatibility**: Support both modern and legacy clients during transition
- **Maintainability**: Clear, well-documented code that integrates smoothly with existing architecture

## Expected Deliverables

1. **Core Transport Module**: Complete `transport.rs` implementation
2. **Updated Server Integration**: Modified server files using new transport
3. **Comprehensive Tests**: Unit and integration tests covering MVP functionality
4. **Validation Results**: Evidence of successful testing with MCP clients

Implement this foundation carefully as it forms the basis for all future MCP communication in the Doc Server project. The success of this task is critical for the reliability and compatibility of the entire system.