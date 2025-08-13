# Acceptance Criteria: MCP Protocol and Transport Bootstrap (MVP)

## Functional Requirements

### FR-1: Initialize Response Alignment
- [ ] `protocolVersion` in initialize is `2025-06-18`
- [ ] No `sse` capability is advertised in initialize

### FR-2: Header Utilities
- [ ] `crates/mcp/src/headers.rs` exists and compiles
- [ ] Constants defined: `MCP-Protocol-Version`, `Mcp-Session-Id`, `2025-06-18`
- [ ] `validate_protocol_version` enforces `2025-06-18` (returns 400 otherwise)
- [ ] `set_standard_headers` sets required headers on responses

### FR-3: Transport Scaffold
- [ ] `crates/mcp/src/transport.rs` exists with `TransportConfig`, `TransportError`, and `unified_mcp_handler` signature
- [ ] Publicly accessible from crate (via `lib.rs`)

### FR-4: Minimal Routing/Headers Integration (to satisfy tests)
- [ ] Router defines GET `/mcp` and returns `405 Method Not Allowed`
- [ ] POST `/mcp` responses include `MCP-Protocol-Version: 2025-06-18`

## Non-Functional Requirements
- [ ] Code compiles with no warnings related to new modules
- [ ] Basic unit tests compile and run

## Test Cases

### TC-1: Initialize Version
**Given**: Server running
**When**: Initialize is called via `/mcp`
**Then**: Response contains `protocolVersion: "2025-06-18"` and no `sse`

### TC-2: Header Utilities Compile
**Then**: A test imports `headers` constants and helpers successfully

### TC-3: Transport API Compile
**Then**: A test references `unified_mcp_handler` and types successfully

### TC-4: GET /mcp Returns 405
**When**: Client issues GET `/mcp`
**Then**: Response status is `405 Method Not Allowed`

### TC-5: Protocol Header on POST
**When**: Client issues POST `/mcp`
**Then**: Response includes header `MCP-Protocol-Version: 2025-06-18`

## Deliverables
- [ ] Updated `handlers.rs` initialize response
- [ ] New `headers.rs` module
- [ ] New `transport.rs` scaffold
- [ ] Minimal tests: `initialize_mvp.rs`, `headers_compile.rs`, `transport_compile.rs`
