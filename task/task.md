# Task ID: 2
# Title: MCP Protocol and Transport Bootstrap (MVP)
# Status: pending
# Dependencies: 1
# Priority: high
# Description: Prepare the codebase for Streamable HTTP MVP by updating the MCP protocol version, removing legacy SSE capability claims, introducing header constants/utilities, and scaffolding the transport module API that Task 3 will implement.
# Details:
Update the MCP initialization response to `protocolVersion: "2025-06-18"` and omit any `sse` capability. Create a `headers.rs` module with constants and simple validators for `MCP-Protocol-Version` (accept only `2025-06-18`) and `Mcp-Session-Id` (pass-through for now). Add a `transport.rs` skeleton that defines `TransportConfig`, minimal error types, and a public `unified_mcp_handler` function signature that will handle POST-only requests (implementation deferred to Task 3). Keep the existing router using POST `/mcp` so Task 3 can wire the handler in.

# Test Strategy:
Create light tests to validate the initialize result protocol version and ensure the new modules compile. Add a test that POST `/mcp` responds successfully with JSON while GET is not supported (405 or not routed is acceptable for this task), leaving full behavior for Task 3.

# Subtasks:
## 1. Update Initialize to 2025-06-18 (remove SSE claim) [pending]
### Dependencies: None
### Description: Modify initialize response to latest protocol version and remove legacy streaming capability claims.
### Details:
Change `protocolVersion` to `"2025-06-18"` in `crates/mcp/src/handlers.rs` and remove any `sse` capability field from the result object.

## 2. Add Header Constants and Validators [pending]
### Dependencies: 2.1
### Description: Create a simple header utilities module to standardize required MCP headers.
### Details:
Create `crates/mcp/src/headers.rs` with:
- Constants: `MCP_PROTOCOL_VERSION`, `MCP_SESSION_ID`, `SUPPORTED_PROTOCOL_VERSION`
- Function `validate_protocol_version(&HeaderMap) -> Result<(), StatusCode>` that enforces `2025-06-18`
- Helper `set_standard_headers(headers: &mut HeaderMap, session_id: Option<Uuid>)` to add MCP headers (used by Task 3)

## 3. Scaffold Transport Module API [pending]
### Dependencies: 2.1, 2.2
### Description: Add a compilable `transport.rs` with MVP API surface for Task 3 to implement.
### Details:
Create `crates/mcp/src/transport.rs` with:
- `#[derive(Clone, Debug)] pub struct TransportConfig { pub protocol_version: String }`
- `#[derive(Debug, thiserror::Error)] pub enum TransportError { #[error("method not allowed")] MethodNotAllowed }`
- `pub async fn unified_mcp_handler(State(_): State<Arc<McpServerState>>, _headers: HeaderMap, _req: Request<Body>) -> Result<Response<Body>, TransportError>` (stub, returns MethodNotAllowed for non-POST; actual logic in Task 3)
- Public exports so server can import in Task 3

## 4. Minimal Tests [pending]
### Dependencies: 2.1
### Description: Add minimal tests covering initialize and compile checks for headers/transport modules.
### Details:
Add `crates/mcp/tests/initialize_mvp.rs` asserting `protocolVersion == "2025-06-18"`. Add a compile-only test referencing `headers` and `transport` to ensure API is in place.
