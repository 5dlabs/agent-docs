# Task 2: MCP Protocol and Transport Bootstrap (MVP)

## Overview

Prepare the codebase for Streamable HTTP MVP by aligning the MCP protocol version, removing legacy SSE capability claims, introducing header constants/utilities, and scaffolding the transport module API to be implemented in Task 3. This reduces risk and clarifies the target surface for subsequent tasks.

## Goals

- Fix protocol version to `2025-06-18` and eliminate legacy references
- Provide a central place for required MCP header handling
- Establish a compilable `transport.rs` API surface (`unified_mcp_handler`) for Task 3

## Implementation Guide

### Phase 1: Initialize Response Alignment

- Update `crates/mcp/src/handlers.rs` initialize response:
  - Set `protocolVersion: "2025-06-18"`
  - Remove `sse` capability claim

Example adjustment:
```rust
// in handlers.rs (initialize response)
Ok(json!({
  "protocolVersion": "2025-06-18",
  "capabilities": {
    "tools": {}
  },
  "serverInfo": { "name": "doc-server-mcp", "version": env!("CARGO_PKG_VERSION") }
}))
```

### Phase 2: Header Utilities

Create `crates/mcp/src/headers.rs` with:
- Constants:
  - `pub const MCP_PROTOCOL_VERSION: &str = "MCP-Protocol-Version";`
  - `pub const MCP_SESSION_ID: &str = "Mcp-Session-Id";`
  - `pub const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";`
- Helpers:
  - `validate_protocol_version(headers: &HeaderMap) -> Result<(), StatusCode>`
  - `set_standard_headers(headers: &mut HeaderMap, session_id: Option<Uuid>)`

These will be consumed by Task 3 when implementing the unified handler.

### Phase 3: Transport Module Scaffold

Add `crates/mcp/src/transport.rs` with a minimal, compilable API:
```rust
#[derive(Clone, Debug)]
pub struct TransportConfig {
    pub protocol_version: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("method not allowed")] MethodNotAllowed,
}

pub async fn unified_mcp_handler(
    State(_): State<Arc<McpServerState>>,
    _headers: HeaderMap,
    _req: Request<Body>,
) -> Result<Response<Body>, TransportError> {
    // Implementation deferred to Task 3
    Err(TransportError::MethodNotAllowed)
}
```
Export `transport` from `lib.rs` (already present) so Task 3 can wire it.

## Acceptance Criteria

- Initialize response returns `protocolVersion: 2025-06-18` and no `sse` field
- `headers.rs` exists with constants and helpers (compiles)
- `transport.rs` exists with a public `unified_mcp_handler` signature and types (compiles)

## Test Plan

- Add `crates/mcp/tests/initialize_mvp.rs` verifying `protocolVersion == "2025-06-18"`
- Add `crates/mcp/tests/headers_compile.rs` importing `headers` and using constants
- Add `crates/mcp/tests/transport_compile.rs` referencing `unified_mcp_handler` to ensure API is in place

## Dependencies

- Task 1: Basic MCP server infrastructure available (present on main)

## Next Tasks

- Task 3: Implement Streamable HTTP Transport Foundation (MVP POST-only handler)
- Task 5: Fixed protocol header handling in request/response paths
