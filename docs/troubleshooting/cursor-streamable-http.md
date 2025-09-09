# Troubleshooting Cursor/Curator Streamable HTTP Connection to MCP

This guide helps collect the right data to diagnose why Cursor/Curator’s initial Streamable HTTP request fails and falls back to SSE (which may currently be disabled by default).

Use these steps to verify connectivity, capture headers/status codes, and pinpoint where the flow breaks.

## Prerequisites

- Access to the environment where the MCP server runs (Kubernetes or Docker)
- `kubectl`, `curl`, and `jq` available locally
- If behind an ingress or proxy, access to its access logs

Assume the configured URL (from Cursor) is:

```
http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp
```

Replace with your actual URL in commands below.

## 1) Basic connectivity and health

Run these checks from the same network path Cursor uses.

```
curl -sv http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/health | jq .
curl -sv http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/metrics | head -n 5
```

Collect:
- Health JSON, HTTP status
- Any TLS or connection errors if using HTTPS

If these fail, focus on DNS, routing, or service mesh/ingress first.

## 2) Verify POST /mcp (JSON-RPC) works

Cursor can use Streamable HTTP, but JSON-RPC over POST should work independently. Validate end-to-end:

```
curl -sv \
  -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2025-06-18' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
```

Collect:
- HTTP status (expect 200)
- Response body (should contain result.tools)
- Response headers (should include MCP-Protocol-Version and Mcp-Session-Id)

If this fails, the problem is not specific to Streamable HTTP. Check server logs (Section 5) for Transport/Security errors.

## 3) Test GET /mcp behavior with and without SSE

The server intentionally gates SSE on GET /mcp behind `MCP_ENABLE_SSE`. If disabled, GET returns 405 to keep tests green.

3a) Current behavior (without changing anything):

```
curl -sv \
  -X GET http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H 'Accept: text/event-stream' \
  -H 'MCP-Protocol-Version: 2025-06-18'
```

Collect:
- HTTP status (expect 405 if SSE disabled)
- Response headers and body

3b) Enable SSE temporarily and re-test:

- Kubernetes: `kubectl -n <ns> set env deploy/<doc-server> MCP_ENABLE_SSE=true`
- Helm: add `--set env.MCP_ENABLE_SSE=true` or use `docs/charts/agent-docs/values-sse.yaml`
- Docker: add `-e MCP_ENABLE_SSE=true`

Then re-run the GET:

```
curl -sv \
  -X GET http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H 'Accept: text/event-stream' \
  -H 'MCP-Protocol-Version: 2025-06-18'
```

Collect:
- HTTP status (expect 200)
- Content-Type (expect text/event-stream)
- First SSE event payload (should include protocolVersion and capabilities)

If enabling SSE returns 406 Not Acceptable, the `Accept` header was not `text/event-stream`. Capture exact request headers (Section 4/5).

## 4) Capture Cursor client-side details

From the Cursor/Curator logs you already see:
- “Creating streamableHttp transport” → “Connecting to streamableHttp server” → “fetch failed”
- Then fallback: “Connecting to SSE server” → 405

Please collect:
- The exact URL Cursor tries first (Streamable HTTP).
- The method and any headers Cursor uses for that probe (esp. `Accept`, `Content-Type`, `Origin`).
- The HTTP status and error text for the initial fetch.

If possible, run a local capture to see the HTTP request Cursor sends (only if allowed in your environment):
- Use an HTTP proxy like mitmproxy/Charles and point Cursor to it, or
- Temporarily route the hostname to a local Nginx that logs headers, then proxy to the MCP service (advanced).

Key hypothesis to test: Some clients first try a GET with `Accept: application/json` to “probe” Streamable HTTP. Our server expects `text/event-stream` for GET and will return 406 if SSE is enabled, or 405 if SSE is disabled. The fallback to SSE then also fails with 405 if SSE is disabled.

## 5) Collect server-side logs with request context

Increase logging for transport, headers, and security modules and reproduce once:

- Helm/Docker env:
```
RUST_LOG=info,mcp::transport=debug,mcp::headers=debug,mcp::security=debug
```

Then tail logs while reproducing from Cursor:

```
kubectl -n <ns> logs -l app=doc-server -f | sed -n '1,200p'
```

Collect the lines that include:
- `mcp_request{request_id=... method=... uri=... protocol_version=...}`
- `Incoming request headers (summary)`
- Any `Transport error:` or `Security validation failed:` lines

Note the `request_id` and match it through subsequent log lines to see the exact path and header validation performed for that request.

## 6) Check ingress/proxy behavior (if applicable)

If you are fronting the service with an ingress (NGINX, Traefik, etc.), collect:
- Access logs for the failing timestamp, including request path and headers
- Any path rewrites or header manipulations in your ingress configuration

Confirm `/mcp` is not being rewritten, and `Accept` is preserved.

## 7) Run built-in acceptance tests (optional)

You can use the project’s acceptance tests to validate POST baseline behavior against your endpoint:

```
BASE_URL=http://doc-server-agent-docs-server.mcp.svc.cluster.local:80 ./scripts/acceptance-tests.sh
```

Note: By default these expect GET /mcp to return 405. If you enable SSE for Cursor, that specific test will fail—this is expected for environments where SSE is enabled.

## 8) Common causes and signals in logs

- SSE disabled (default):
  - GET /mcp → 405
  - Fix: set `MCP_ENABLE_SSE=true` and retry.

- Wrong `Accept` header for GET when SSE is enabled:
  - GET /mcp with `Accept: application/json` → 406 Not Acceptable
  - Fix: ensure client uses `Accept: text/event-stream` for SSE.

- Security validation errors (403/400):
  - Logs show “Security validation failed” messages (origin or DNS rebinding)
  - Fix: adjust Origin or allowed origins; ensure Host/Origin alignment when both present.

- Ingress path rewrite or missing headers:
  - Ingress logs show different path or stripped Accept headers
  - Fix: correct ingress configuration to pass through `/mcp` and headers.

## 9) Data to share for triage

Please paste the following:
- Output of Section 2 and 3 curl commands (full `-sv` including request/response headers)
- Relevant server logs from Section 5 with the matching `request_id`
- Any ingress access log lines for the same timestamp
- Cursor client-side log lines showing the initial fetch URL and the HTTP status it saw

With these, we can precisely identify whether the initial Streamable HTTP probe is:
- Blocked due to SSE gate (405),
- Using an unexpected Accept header (406),
- Blocked by security validation (403/400), or
- Impacted by network/ingress configuration.

