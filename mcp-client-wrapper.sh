#!/bin/bash
# MCP Client Wrapper with X-Client-Id header support
# This wrapper adds a stable client ID header to all MCP requests

# Generate a stable client ID based on machine and user
CLIENT_ID="${MCP_CLIENT_ID:-cursor-$(hostname -s)-$(whoami)}"
DOC_SERVER_URL="${DOC_SERVER_URL:-http://doc-server-agent-docs-server.mcp.svc.cluster.local:80}"

echo "Starting MCP client with Client-Id: $CLIENT_ID" >&2
echo "Connecting to: $DOC_SERVER_URL" >&2

# Use curl or another tool that supports custom headers to create a stdio transport
# This is a placeholder - the actual implementation depends on your MCP client library

exec npx @modelcontextprotocol/client-stdio \
  --url "$DOC_SERVER_URL/mcp" \
  --header "X-Client-Id: $CLIENT_ID" \
  "$@"