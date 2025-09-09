# Cursor MCP Connection Root Cause Analysis

## Problem Statement
Cursor client was stuck at "Loading tools..." when attempting to connect to the MCP server, despite SSE being enabled and the server appearing to work correctly.

## Investigation Timeline

### 1. Initial SSE Enablement
- **Issue**: Server returning 405 Method Not Allowed for GET /mcp requests
- **Cause**: `MCP_ENABLE_SSE` environment variable not set
- **Fix**: Set `MCP_ENABLE_SSE: "true"` in Helm chart values.yaml

### 2. SSE Stream Stability
- **Issue**: Client receiving "TypeError: fetch failed: other side closed"
- **Cause**: Minimal SSE implementation not maintaining persistent connection
- **Fix**: Implemented proper SSE streaming with:
  - Initial `serverInfo` event
  - Periodic keep-alive comments
  - Proper headers (`Access-Control-Allow-Origin`, `X-Accel-Buffering: no`)

### 3. Session Management
- **Issue**: Cursor losing session between reconnections
- **Cause**: Cursor not preserving `Mcp-Session-Id` header
- **Fix**: Implemented client-based session management using `MCP_CLIENT_ID` environment variable

### 4. Protocol Violation (Root Cause)
- **Issue**: Cursor sending `notifications/initialized` as first message
- **Cause**: Cursor caching session state but not following proper initialization flow
- **Expected Flow**:
  1. Client sends `initialize` request
  2. Server responds with capabilities
  3. Client sends `notifications/initialized` acknowledgment
- **Actual Flow**: Cursor skipping directly to step 3
- **Fix**: Return error when receiving `notifications/initialized` without prior `initialize`

## Enhanced Logging Output

The comprehensive logging revealed the exact issue:

```
2025-09-09T17:37:26.049315Z  INFO 🔍 CURSOR RAW REQUEST BODY: {"method":"notifications/initialized","jsonrpc":"2.0"}
2025-09-09T17:37:26.049365Z  INFO 🔍 CURSOR PARSED JSON-RPC: {
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}
```

This showed that Cursor was violating the MCP protocol by sending a notification that should only come AFTER initialization.

## Configuration Requirements

### Server Environment Variables
```yaml
env:
  MCP_ENABLE_SSE: "true"  # Enable SSE support
```

### Client Configuration (.cursor/mcp.json)
```json
{
  "doc-server": {
    "env": {
      "MCP_CLIENT_ID": "cursor-[username]-[project]"
    }
  }
}
```

## Testing Commands

### Verify SSE is enabled
```bash
curl -v -N http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Accept: text/event-stream" \
  -H "User-Agent: Cursor/1.5.11 (darwin arm64)"
```

### Test proper initialization flow
```bash
# Step 1: Initialize
curl -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "User-Agent: Cursor/1.5.11 (darwin arm64)" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"Cursor","version":"1.5.11"}},"id":1}'

# Step 2: Send initialized notification (after receiving response)
curl -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "User-Agent: Cursor/1.5.11 (darwin arm64)" \
  -H "Mcp-Session-Id: [session-id-from-step-1]" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}'
```

## Monitoring & Debugging

### Check enhanced logs for Cursor requests
```bash
kubectl logs -n mcp <pod-name> | grep "🔍"
```

### Key log indicators:
- `🔍 CURSOR REQUEST DETECTED`: Initial request received
- `🔍 CURSOR RAW REQUEST BODY`: Exact payload sent by Cursor
- `🔍 CURSOR PARSED JSON-RPC`: Processed request
- `🔍 CURSOR RESPONSE`: Server response
- `🔍 CURSOR SSE REQUEST`: SSE fallback attempt
- `🔍 CURSOR REQUEST FAILED`: Error conditions

## Lessons Learned

1. **Protocol Compliance**: Clients may cache state incorrectly and violate protocol flow
2. **Defensive Programming**: Servers should handle out-of-order messages gracefully
3. **Comprehensive Logging**: Enhanced logging with emoji markers makes debugging much easier
4. **Session Persistence**: Client-based session IDs help maintain state across reconnections
5. **SSE Requirements**: Proper headers and keep-alives are essential for stable SSE connections

## Future Improvements

1. **Session State Tracking**: Track initialization state per session to handle reconnections better
2. **Automatic Recovery**: When detecting protocol violations, automatically guide client to proper state
3. **Metrics**: Add specific metrics for protocol violations and recovery attempts
4. **Client Detection**: Different handling strategies based on detected client type/version

