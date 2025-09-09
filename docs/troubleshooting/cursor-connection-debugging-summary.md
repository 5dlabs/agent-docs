# Cursor Connection Debugging Summary

## Current Status
As of September 9, 2025, we're debugging an issue where the Cursor client cannot establish a stable connection to the MCP server.

## Symptoms
1. Cursor client shows "0 tools found" initially, then "15 tools found" 2 seconds later
2. Client logs show "fetch failed" and "other side closed" errors
3. Both streamableHttp and SSE fallback attempts fail
4. Server successfully handles `initialize` requests but connection drops afterward

## What's Working
- POST /mcp with JSON-RPC works correctly (returns 15 tools)
- Server accepts `initialize` and returns proper capabilities
- Server accepts `notifications/initialized` (previously was rejecting it)
- SSE endpoint returns 200 OK with proper headers
- Session management with `MCP_CLIENT_ID` is functional

## What's Not Working
- SSE stream establishes connection but sends no data
- Client cannot maintain stable connection after initial handshake
- Streamable HTTP transport fails immediately with "fetch failed"

## Investigation Progress

### 1. Fixed Issues
- ✅ Enabled SSE via `MCP_ENABLE_SSE: "true"` environment variable
- ✅ Fixed protocol handler to accept `notifications/initialized` messages
- ✅ Implemented client-based session management with `MCP_CLIENT_ID`
- ✅ Added comprehensive logging for Cursor requests

### 2. Current Investigation
- **SSE Data Flow Issue**: The SSE endpoint returns 200 OK with proper headers but no event data is being sent
- **Simplified Implementation**: Created a minimal SSE stream to isolate the issue
- **Pending**: Waiting for CI/CD to deploy the simplified implementation

### 3. Debugging Approach
```rust
// Simplified SSE stream for testing
let stream = async_stream::stream! {
    info!("SSE: Starting stream generation");
    
    // Send initial event immediately
    let init_event = Event::default()
        .id("0")
        .event("message")
        .data(/* JSON payload */);
    yield Ok::<Event, Infallible>(init_event);
    
    // Send test event after 1s
    tokio::time::sleep(Duration::from_secs(1)).await;
    let test_event = Event::default()
        .id("1")
        .event("message")
        .data("{\"test\":\"message\"}");
    yield Ok::<Event, Infallible>(test_event);
    
    // Keep-alive loop
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let keep_alive = Event::default().comment("keep-alive");
        yield Ok::<Event, Infallible>(keep_alive);
    }
};
```

## Test Commands

### Test SSE Stream
```bash
# Test SSE endpoint (should return event data)
curl -N http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Accept: text/event-stream" \
  -H "MCP-Protocol-Version: 2025-06-18"
```

### Test Full Flow
```bash
# Step 1: Initialize
curl -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"TestClient","version":"1.0"}},"id":1}'

# Step 2: List tools
curl -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -H "Mcp-Session-Id: <session-id-from-step-1>" \
  -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}'
```

## Next Steps
1. Deploy and test simplified SSE implementation
2. Verify event data is actually being sent over the wire
3. Check if axum's SSE implementation requires specific configuration
4. Consider implementing a basic HTTP long-polling fallback if SSE continues to fail

## Configuration Requirements

### Server (.cursor/mcp.json)
```json
{
  "mcpServers": {
    "doc-server": {
      "url": "http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp",
      "env": {
        "MCP_CLIENT_ID": "cursor-[username]-[project]"
      }
    }
  }
}
```

### Helm Chart (values.yaml)
```yaml
env:
  MCP_ENABLE_SSE: "true"
```
