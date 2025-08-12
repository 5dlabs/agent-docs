# Autonomous Agent Prompt: Streamable HTTP Response Semantics, Keep-Alive, and Error Handling

## Mission
Align response behavior with MCP 2025-06-18 Streamable HTTP: unify response formats, implement keep-alive heartbeats/timeouts on streams, and ensure JSON-RPC error handling with proper HTTP mapping.

## Primary Objectives
1. Unify `/mcp` POST (JSON) and GET (event-stream) behaviors
2. Implement 30s heartbeat and 90s idle timeout for streams
3. Include `Mcp-Session-Id` and `MCP-Protocol-Version` in responses
4. Map transport and application errors to correct HTTP status codes
5. Add structured logging and counters for stream lifecycle

## Implementation Steps
1. Update GET `/mcp` to return `text/event-stream` with properly formatted events
2. Emit heartbeat every 30s; close idle streams after 90s
3. Ensure headers are set on all responses (session id, protocol version)
4. Implement JSON-RPC error formatting and HTTP mapping
5. Add logs/metrics for stream opened/closed, heartbeat, timeout

## Success Criteria
- [ ] Heartbeats observed every 30s; idle close at ~90s
- [ ] Correct headers present on all responses
- [ ] Errors return correct HTTP status and JSON-RPC body
- [ ] Logs/metrics capture stream lifecycle events

## Deployment Validation (Mandatory 4-step)
1. Push → CI build/tests
2. Deploy via Helm
3. Live test with MCP client (streams/headers/errors)
4. Record validation results in task artifacts