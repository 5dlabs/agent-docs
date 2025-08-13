# Autonomous Agent Prompt: MCP Protocol and Transport Bootstrap (MVP)

## Mission
Update the codebase to align with the latest MCP protocol version and prepare a minimal transport API surface for the upcoming MVP transport task.

## Context
- Current main uses legacy `2024-11-05` and claims SSE capability
- We are moving to POST-only MVP with `2025-06-18` and no streaming/legacy support

## Primary Objectives
1. Align initialize response to `protocolVersion: "2025-06-18"` and remove `sse`
2. Create `headers.rs` with constants and helpers for protocol/session headers
3. Scaffold `transport.rs` with `TransportConfig`, `TransportError`, and `unified_mcp_handler` signature

## Step-by-Step Implementation
1. Edit `crates/mcp/src/handlers.rs` to update initialize
2. Add new module `crates/mcp/src/headers.rs` with constants and helpers
3. Add new module `crates/mcp/src/transport.rs` with the public API signature (no logic yet)
4. Ensure `lib.rs` exposes modules as needed (already includes `transport`)

## Success Criteria
- [ ] Initialize returns `2025-06-18` and no `sse`
- [ ] `headers.rs` compiles and exports constants/helpers
- [ ] `transport.rs` compiles and exports required types and function
- [ ] Router exposes GET `/mcp` that returns 405; POST `/mcp` includes `MCP-Protocol-Version`

## Testing
- Add minimal tests `initialize_mvp.rs`, `headers_compile.rs`, and `transport_compile.rs` to verify presence and API surface
- Add a routing test asserting GET `/mcp` returns 405
- Add a header test asserting POST `/mcp` includes `MCP-Protocol-Version: 2025-06-18`

## Notes
- Keep changes minimal; Task 3 will implement logic and wire routing to the new handler
