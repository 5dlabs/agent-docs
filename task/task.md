# Task ID: 2
# Title: Implement Streamable HTTP Transport Foundation
# Status: pending
# Dependencies: 1
# Priority: high
# Description: Replace the deprecated HTTP+SSE transport with the new Streamable HTTP transport following MCP 2025-06-18 specification, creating the core transport layer infrastructure.
# Details:
Implement `transport.rs` with Streamable HTTP MVP using Axum 0.7. Create a unified MCP endpoint that accepts POST only (JSON-RPC over `application/json`). Return `405 Method Not Allowed` for GET. Implement proper JSON-RPC message handling with UTF-8 encoding. Do NOT implement SSE streaming or legacy HTTP+SSE compatibility for MVP. Reference `.reference/transports.md` for the 2025-06-18 spec details. Use tower-http for CORS and tracing middleware integration.

# Test Strategy:
Create integration tests in `crates/mcp/tests/` validating POST-only handling (405 on GET), JSON-RPC request/response cycles, and proper error handling for malformed requests. Test with both Cursor and Toolman clients.

# Subtasks:
## 1. Create Core HTTP Transport Module Structure [pending]
### Dependencies: None
### Description: Set up the foundational transport module structure in transport.rs with necessary imports, types, and session management infrastructure following the Streamable HTTP specification.
### Details:
Create the base transport.rs module with imports for axum 0.7, tower-http, serde_json, and async-stream. Define core types including TransportConfig, SessionManager, McpSession with session ID generation using uuid v4. Implement session storage with HashMap and Arc<RwLock> for thread-safe access. Add MCP-Protocol-Version and Mcp-Session-Id header constants. Create error types for transport-specific failures. Set up logging with tracing for debug and error messages.

## 2. Implement Unified MCP Endpoint Handler [pending]
### Dependencies: 2.1
### Description: Create the main MCP endpoint handler supporting both POST and GET methods with proper content negotiation and request routing logic.
### Details:
Implement `unified_mcp_handler` in `transport.rs` handling POST requests only. Validate `Content-Type: application/json`. Process JSON-RPC requests, notifications, and responses with appropriate status codes (200 OK, 400 Bad Request). For any non-POST method (e.g., GET), return 405. Add `MCP-Protocol-Version` header extraction and validation (require 2025-06-18 only). Integrate session ID management from headers. Create request context structure for passing session and protocol information.

## (Removed) SSE Streaming Infrastructure
Out of scope for MVP. Do not implement.

## (Removed) Backward Compatibility Detection
Out of scope for MVP. Require `MCP-Protocol-Version: 2025-06-18` and return 400 for unsupported versions.

## 5. Integrate Transport with MCP Server [pending]
### Dependencies: 2.1
### Description: Wire the new Streamable HTTP transport into the existing MCP server infrastructure, replacing old endpoints and ensuring proper request flow.
### Details:
Update `server.rs` to use the new unified transport handler from `transport.rs`. Ensure `/mcp` routes POST to the handler; remove or ignore any `/sse` routing. Integrate `McpHandler` with the transport for JSON-RPC processing. Add transport configuration to `McpServerState`. Update CORS configuration as needed. Wire session management into server state. Add transport initialization in `McpServer::new`. Update router creation to use the transport endpoint with POST only.

