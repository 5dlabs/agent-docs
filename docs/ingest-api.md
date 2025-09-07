# MCP Server Ingest API Documentation

This document describes the HTTP API endpoints available for interacting with the MCP Server's document ingestion functionality.

## Base URL

**Development (Local):**
```bash
http://localhost:3001
```

**Production (Kubernetes):**
```bash
https://doc-server.agent-platform.svc.cluster.local:3001
```

**Configuration:**

- Default port: `3001`
- Environment variables:
  - `PORT` - Primary port configuration
  - `MCP_PORT` - Alternative port configuration
- **Production Note:** The server runs in the `agent-platform` namespace in Kubernetes
- **Access:** External access requires Twingate VPN for security
- **Internal Access:** Other services in the cluster can use the internal service URL

## Authentication

Currently, no authentication is required for the ingest endpoints. However, all endpoints are subject to:

- CORS validation (allows all origins for development)
- DNS rebinding protection
- Session management for MCP protocol endpoints

## Endpoints

### 1. Intelligent Document Ingestion

**Endpoint:** `POST /ingest/intelligent`

Starts an intelligent document ingestion job for the specified URL and document type.

#### Request Format

**Content-Type:** `application/json`

**Body:**

```json
{
  "url": "string",      // Required: URL to ingest
  "doc_type": "string"  // Required: Document type (e.g., "github", "web", "api")
}
```

#### Intelligent Ingest Response

**Success (200):**

```json
{
  "job_id": "uuid"
}
```

**Error Responses:**

- `400 Bad Request` - Missing or invalid required fields
- `500 Internal Server Error` - Server error during job creation

#### Intelligent Ingest Example

**Development:**
```bash
curl -X POST http://localhost:3001/ingest/intelligent \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://github.com/example/repo",
    "doc_type": "github"
  }'
```

**Production:**
```bash
curl -X POST https://doc-server.agent-platform.svc.cluster.local:3001/ingest/intelligent \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://github.com/example/repo",
    "doc_type": "github"
  }'
```

**Response:**

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### 2. Get Ingest Job Status

**Endpoint:** `GET /ingest/jobs/{job_id}`

Retrieves the current status and details of a specific ingestion job.

#### Parameters

- `job_id` (path parameter): UUID of the ingestion job

#### Job Status Response

**Success (200):**

```json
{
  "job_id": "uuid",
  "status": "string",        // "queued", "running", "completed", "failed", "cancelled"
  "url": "string",          // Original URL being ingested
  "doc_type": "string",     // Document type
  "started_at": "datetime", // ISO 8601 datetime when job started
  "finished_at": "datetime|null", // ISO 8601 datetime when job finished (null if still running)
  "output": "string|null",  // Job output (null if still running)
  "error": "string|null"    // Error message (null if successful)
}
```

**Error Responses:**

- `404 Not Found` - Job ID not found

#### Job Status Example

**Development:**
```bash
curl http://localhost:3001/ingest/jobs/550e8400-e29b-41d4-a716-446655440000
```

**Production:**
```bash
curl https://doc-server.agent-platform.svc.cluster.local:3001/ingest/jobs/550e8400-e29b-41d4-a716-446655440000
```

#### Job Status Response Example

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "url": "https://github.com/example/repo",
  "doc_type": "github",
  "started_at": "2024-01-15T10:30:00Z",
  "finished_at": "2024-01-15T10:35:00Z",
  "output": "Successfully ingested 42 documents",
  "error": null
}
```

## MCP Protocol Endpoint

**Endpoint:** `POST /mcp`

The main MCP protocol endpoint for JSON-RPC communication. This endpoint requires specific protocol headers.

### MCP Headers and Authentication

#### Required Headers

- `MCP-Protocol-Version: 2025-06-18` - Protocol version (required)
- `Content-Type: application/json` - Request content type (required)
- `Accept: application/json` - Expected response content type (required)
- `Mcp-Session-Id: <uuid>` - Optional: Session ID for stateful communication

#### MCP Request Format

**Method:** POST

**Content-Type:** `application/json`

**Body:** JSON-RPC 2.0 formatted request

#### MCP Response Format

**Success (200):** JSON-RPC 2.0 formatted response

**Error Responses:**

- `400 Bad Request` - Missing required headers or invalid protocol version
- `405 Method Not Allowed` - GET requests are not supported (JSON-only policy)
- `415 Unsupported Media Type` - Invalid content type

#### MCP Usage Example

**Development:**
```bash
curl -X POST http://localhost:3001/mcp \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list",
    "params": {}
  }'
```

**Production:**
```bash
curl -X POST https://doc-server.agent-platform.svc.cluster.local:3001/mcp \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list",
    "params": {}
  }'
```

## Health Check Endpoints

The server also provides comprehensive health check endpoints (inherited from the health module):

- `GET /health` - Basic health check
- `GET /health/live` - Liveness probe
- `GET /health/ready` - Readiness probe
- `GET /health/detailed` - Detailed health information

## Error Handling

All endpoints follow consistent error response patterns:

```json
{
  "error": "Error message description"
}
```

Common HTTP status codes:

- `200` - Success
- `400` - Bad Request (invalid input)
- `404` - Not Found
- `405` - Method Not Allowed
- `415` - Unsupported Media Type
- `500` - Internal Server Error

## Rate Limiting

Currently, no explicit rate limiting is implemented, but the server includes:

- Session management with automatic cleanup
- Concurrent session limits (configurable)
- Request tracing and metrics collection

## Monitoring

The server provides metrics and logging:

- Request/response metrics via `/metrics` (if enabled)
- Structured logging with request IDs
- Session statistics and health monitoring

## Development vs Production

**Development:**

- CORS allows all origins
- Detailed error messages
- Debug logging enabled

**Production:**

- CORS restricted to allowed domains
- Minimal error messages for security
- Structured logging with appropriate levels

## Integration Examples

### Python Example

```python
import requests
import json

# Configuration
BASE_URL = "http://localhost:3001"  # Development
# BASE_URL = "https://doc-server.agent-platform.svc.cluster.local:3001"  # Production

# Start ingestion job
response = requests.post(
    f"{BASE_URL}/ingest/intelligent",
    json={
        "url": "https://github.com/example/repo",
        "doc_type": "github",
        "yes": True
    }
)

job_data = response.json()
job_id = job_data["job_id"]

# Check status
status_response = requests.get(f"{BASE_URL}/ingest/jobs/{job_id}")
status_data = status_response.json()
print(f"Job status: {status_data['status']}")
```

### JavaScript/Node.js Example

```javascript
const fetch = require('node-fetch');

// Configuration
const BASE_URL = "http://localhost:3001";  // Development
// const BASE_URL = "https://doc-server.agent-platform.svc.cluster.local:3001";  // Production

// Start ingestion
const startResponse = await fetch(`${BASE_URL}/ingest/intelligent`, {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    url: 'https://github.com/example/repo',
    doc_type: 'github',
    yes: true
  })
});

const { job_id } = await startResponse.json();

// Check status
const statusResponse = await fetch(`${BASE_URL}/ingest/jobs/${job_id}`);
const status = await statusResponse.json();
console.log(`Job status: ${status.status}`);
```

## Troubleshooting

### Common Issues

1. **Connection Refused**
   - **Development:** Ensure server is running on correct port (3001)
   - **Production:** Verify Twingate VPN connection is active
   - Check firewall/network configuration

2. **Invalid Protocol Version**
   - Use exact header: `MCP-Protocol-Version: 2025-06-18`
   - No other protocol versions are currently supported

3. **Job Not Found (404)**
   - Verify job ID is correct UUID format
   - Check if job has expired or been cleaned up (completed/failed jobs are retained for 30 days)
   - Ensure the server has applied the latest migrations (ingest job tracking is persisted in the database and available across replicas)

4. **CORS Errors**
   - Ensure proper headers are set for browser requests
   - Server allows all origins in development mode

5. **Production Access Issues**
   - **Twingate Required:** External access to production requires Twingate VPN
   - **Internal Services:** Other Kubernetes services can access via internal DNS
   - **SSL Certificate:** Production uses HTTPS with proper certificates

### Debug Information

Enable debug logging by setting:

```bash
export RUST_LOG=debug,doc_server=debug
```

Request logs include:

- Request ID for tracing
- Headers (redacted for security)
- Response status and timing
- Session information
