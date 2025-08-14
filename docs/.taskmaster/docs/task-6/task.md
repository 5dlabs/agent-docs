# Task 6: Streamable HTTP Response Semantics, Keep-Alive, and Error Handling

## Overview (MVP scope)
Align response behavior with MCP 2025-06-18 Streamable HTTP for JSON-only POST handling. Implement session-aware headers and comprehensive JSON-RPC error handling. Streaming and keep-alive are out of scope for MVP.

## Implementation Guide
- Response format for `/mcp` endpoint
  - POST: JSON-RPC request/response with `Content-Type: application/json`
  - GET: return `405 Method Not Allowed`
- Headers
  - Add `Mcp-Session-Id` to responses; echo `MCP-Protocol-Version: 2025-06-18`
- Error model
  - Map transport-layer failures to HTTP codes (400, 401, 403, 404, 405, 413, 429, 500)
  - JSON-RPC 2.0 compliant error objects for application errors
  - Never leak sensitive details; include stable error codes and correlation id
- Logging/observability
  - Structured logs for request id, session id, protocol version

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

## Acceptance Criteria (MVP)
- All responses include correct `Mcp-Session-Id` and `MCP-Protocol-Version`
- Errors return appropriate HTTP status and JSON-RPC error bodies
- Logs include request id, session id, and version

## Validation (Mandatory 4-step)
1. Push branch to GitHub to trigger build
2. CI builds container and runs clippy/tests
3. Deploy via Helm to Kubernetes
4. Real-world testing with a compliant MCP client; verify headers and errors## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.
