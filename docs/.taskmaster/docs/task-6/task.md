# Task 6: Streamable HTTP Response Semantics, Keep-Alive, and Error Handling

## Overview
Align response behavior with MCP 2025-06-18 Streamable HTTP. Implement server-side keep-alive heartbeats, session-aware headers, and comprehensive JSON-RPC error handling. SSE is not a standalone transport; event-stream is used only as the streaming response mechanism of Streamable HTTP.

## Implementation Guide
- Response format unification for the unified `/mcp` endpoint
  - POST: JSON-RPC request/response with `Content-Type: application/json`
  - GET (stream request): `text/event-stream` with properly formatted SSE events
- Keep-alive and connection management
  - Heartbeat every 30s on streams; timeout after 90s of inactivity
  - Include last event id when available to support client-side resume
  - Add `Mcp-Session-Id` to responses; echo negotiated `MCP-Protocol-Version`
- Error model
  - Map transport-layer failures to HTTP codes (400, 401, 403, 404, 405, 413, 429, 500)
  - JSON-RPC 2.0 compliant error objects for application errors
  - Never leak sensitive details; include stable error codes and correlation id
- Logging/observability
  - Structured logs for request id, session id, negotiated protocol version
  - Emit counters for stream opens/closes, heartbeats sent, timeouts

## Technical Requirements
- MCP 2025-06-18 headers
  - Request/Response: `MCP-Protocol-Version: 2025-06-18`
  - Response: `Mcp-Session-Id: <uuid>`
  - Content negotiation: `Accept: application/json,text/event-stream`
- Stream keep-alive
  - 30s heartbeat message over event-stream
  - 90s idle timeout with cleanup
- JSON-RPC 2.0 compliant error objects
- Structured logging with tracing context

## Acceptance Criteria
- Stream responses include periodic heartbeats; idle streams close after 90s
- All responses include correct `Mcp-Session-Id` and `MCP-Protocol-Version`
- Errors return appropriate HTTP status and JSON-RPC error bodies
- Logs include request id, session id, and version; metrics exposed for streams

## Validation (Mandatory 4-step)
1. Push branch to GitHub to trigger build
2. CI builds container and runs clippy/tests
3. Deploy via Helm to Kubernetes
4. Real-world testing with a compliant MCP client; verify streams, headers, and errors