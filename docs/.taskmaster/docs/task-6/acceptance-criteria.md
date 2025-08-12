# Acceptance Criteria: Streamable HTTP Response Semantics, Keep-Alive, and Error Handling

## Functional Requirements

### FR-1: Streamable HTTP Response Behavior
- [ ] Unified `/mcp` endpoint supports POST (JSON-RPC) and GET (event-stream)
- [ ] GET responds with `Content-Type: text/event-stream` and proper SSE events
- [ ] Heartbeat messages emitted every 30 seconds on active streams
- [ ] Idle streams closed after 90 seconds of inactivity

### FR-2: MCP Header Compliance
- [ ] `MCP-Protocol-Version: 2025-06-18` included in all responses
- [ ] `Mcp-Session-Id: <uuid>` included in all responses
- [ ] Accept header negotiation supports `application/json,text/event-stream`

### FR-3: Error Handling
- [ ] HTTP status mapping for common conditions (400, 401, 403, 404, 405, 413, 429, 500)
- [ ] JSON-RPC 2.0 compliant error objects (code, message, optional data)
- [ ] No sensitive data in error bodies; include stable error codes and correlation id

### FR-4: Observability
- [ ] Structured logs include request id, session id, and negotiated protocol version
- [ ] Metrics counters for streams opened/closed, heartbeats sent, timeouts

## Test Cases

### TC-1: Stream Initialization
**Given**: Client requests GET `/mcp` with `Accept: text/event-stream`
**Then**: Response 200 with event-stream and initial event delivered

### TC-2: Heartbeat and Timeout
**Given**: Active stream with no messages
**Then**: Heartbeat every 30s; connection closed after 90s idle

### TC-3: JSON-RPC Error Formatting
**When**: Malformed JSON-RPC request sent via POST `/mcp`
**Then**: 400 with JSON-RPC error object; no sensitive data

### TC-4: Header Compliance
**Then**: All responses include `Mcp-Session-Id` and `MCP-Protocol-Version`

## Deliverables
- [ ] Stream response implementation with keep-alive and idle timeout
- [ ] Error handling module with HTTP mapping and JSON-RPC formatting
- [ ] Logging and metrics for stream lifecycle and errors

## Production Validation (4-step)
1. Push branch to GitHub (build triggers)
2. CI builds container and runs clippy/tests
3. Deploy via Helm to Kubernetes
4. Real-world testing with a compliant MCP client (verify streams, headers, errors)