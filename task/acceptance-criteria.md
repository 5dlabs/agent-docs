# Acceptance Criteria: Streamable HTTP Response Semantics, Keep-Alive, and Error Handling

## Functional Requirements

### FR-1: Streamable HTTP Response Behavior (MVP)
- [ ] Unified `/mcp` endpoint supports POST (JSON-RPC) only
- [ ] GET returns `405 Method Not Allowed`

### FR-2: MCP Header Compliance
- [ ] `MCP-Protocol-Version: 2025-06-18` included in all responses
- [ ] `Mcp-Session-Id: <uuid>` included in all responses

### FR-3: Error Handling
- [ ] HTTP status mapping for common conditions (400, 401, 403, 404, 405, 413, 429, 500)
- [ ] JSON-RPC 2.0 compliant error objects (code, message, optional data)
- [ ] No sensitive data in error bodies; include stable error codes and correlation id

### FR-4: Observability
- [ ] Structured logs include request id, session id, and negotiated protocol version
- [ ] Metrics counters for streams opened/closed, heartbeats sent, timeouts

## Test Cases

### TC-1: GET Method Handling
**Given**: Client requests GET `/mcp`
**Then**: Response 405 Method Not Allowed

### TC-2: JSON-RPC Error Formatting
**When**: Malformed JSON-RPC request sent via POST `/mcp`
**Then**: 400 with JSON-RPC error object; no sensitive data

### TC-3: Header Compliance
**Then**: All responses include `Mcp-Session-Id` and `MCP-Protocol-Version`

### TC-4: Logging
**Then**: Logs include request id, session id, and version

## Deliverables
- [ ] Error handling module with HTTP mapping and JSON-RPC formatting
- [ ] Logging

## Production Validation (4-step)
1. Push branch to GitHub (build triggers)
2. CI builds container and runs clippy/tests
3. Deploy via Helm to Kubernetes
4. Real-world testing with a compliant MCP client (verify streams, headers, errors)### NFR-0: Code Quality and Automation
- [ ] After adding any new function, run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and fix all warnings before continuing
- [ ] Prior to submission, ensure `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` all pass locally
- [ ] All changes pushed to a feature branch; GitHub Actions must complete successfully (including deployment) before opening a PR
- [ ] PR creation is gated on a green CI pipeline and successful deployment of the server artifact
