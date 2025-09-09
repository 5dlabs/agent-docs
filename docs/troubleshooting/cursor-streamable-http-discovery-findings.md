# MCP Cursor/Streamable HTTP Connection - Code Discovery Findings

## Executive Summary

This document contains my findings from a comprehensive analysis of the MCP server implementation, specifically focusing on the Cursor/Streamable HTTP connection troubleshooting. The analysis reveals several implementation details that can enhance the troubleshooting guide and provide deeper insights into connection issues.

## 1. Transport Layer Implementation Analysis

### Streamable HTTP Handler Architecture

The MCP server implements a unified endpoint handler (`unified_mcp_handler`) in `mcp/src/transport.rs` that supports both POST (JSON-RPC) and GET (SSE) methods:

```rust
pub async fn unified_mcp_handler(
    State(state): State<McpServerState>,
    headers: HeaderMap,
    request: Request<Body>,
) -> Result<Response, TransportError> {
    // ...
    match *request.method() {
        Method::POST => handle_json_rpc_request(state, headers, request, request_id).await,
        Method::DELETE => handle_delete_session_request(&state, &headers, request_id),
        Method::GET => {
            // Gate SSE behind an env flag for compatibility with acceptance tests
            if std::env::var("MCP_ENABLE_SSE")
                .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            {
                handle_sse_request(&state, &headers, request_id)
            } else {
                warn!(request_id = %request_id, "GET /mcp not enabled (MCP_ENABLE_SSE not set) - returning 405");
                Err(TransportError::MethodNotAllowed)
            }
        }
        _ => {
            metrics().increment_method_not_allowed();
            warn!(request_id = %request_id, method = %request.method(), "Unsupported HTTP method");
            Err(TransportError::MethodNotAllowed)
        }
    }
}
```

### Key Implementation Details

1. **SSE Gating Logic**: SSE is explicitly gated behind the `MCP_ENABLE_SSE` environment variable
2. **Protocol Version Validation**: All requests require `MCP-Protocol-Version: 2025-06-18` header
3. **Accept Header Validation**: GET requests must have `Accept: text/event-stream` for SSE
4. **POST requests require `Accept` compatible with `application/json`
5. **Session Management**: Both transport methods support MCP session management
6. **Security Validation**: Origin and DNS rebinding protection on all requests

## 2. Security Implementation Details

### Origin and DNS Rebinding Protection

The security module (`mcp/src/security.rs`) implements comprehensive protection:

```rust
/// Validate origin header against security configuration
pub fn validate_origin(headers: &HeaderMap, config: &SecurityConfig) -> Result<(), SecurityError> {
    let origin = extract_origin(headers);

    // Check if Origin header is required
    if config.require_origin_header && origin.is_none() {
        return Err(SecurityError::MissingOriginHeader);
    }

    // If origin is present, validate it
    if let Some(origin_value) = origin {
        // Basic origin format validation
        if !origin_value.starts_with("http://") && !origin_value.starts_with("https://") {
            return Err(SecurityError::InvalidOriginFormat(origin_value));
        }

        // Check if origin is in allowed list (if strict validation enabled)
        if config.strict_origin_validation && !config.is_origin_allowed(&origin_value) {
            warn!("Origin not allowed: {}", origin_value);
            return Err(SecurityError::OriginNotAllowed(origin_value));
        }
    }

    Ok(())
}
```

### Default Security Configuration

```rust
impl Default for SecurityConfig {
    fn default() -> Self {
        let mut allowed_origins = HashSet::new();
        allowed_origins.insert("http://localhost:3001".to_string());
        allowed_origins.insert("https://localhost:3001".to_string());
        // ... more localhost variants

        Self {
            allowed_origins,
            strict_origin_validation: true,
            localhost_only: true,
            require_origin_header: false, // Keep flexible for MVP
        }
    }
}
```

### DNS Rebinding Detection

```rust
pub fn validate_dns_rebinding(
    headers: &HeaderMap,
    config: &SecurityConfig,
) -> Result<(), SecurityError> {
    let host = extract_host(headers);
    let origin = extract_origin(headers);

    // If both headers are present, validate they match for security
    if let (Some(host_value), Some(origin_value)) = (host, origin) {
        // Parse origin to extract host part
        let origin_host = url::Url::parse(&origin_value).map_or(None, |url| {
            url.host_str().map(|h| {
                url.port()
                    .map_or_else(|| h.to_string(), |port| format!("{h}:{port}"))
            })
        });

        // Check for DNS rebinding attack
        if let Some(origin_host_value) = origin_host {
            // Allow localhost variants
            let is_safe = config.is_localhost_origin(&host_value)
                && config.is_localhost_origin(&origin_host_value);

            if !is_safe && host_value != origin_host_value {
                error!(
                    "DNS rebinding attack detected - Host: {}, Origin: {}",
                    host_value, origin_value
                );
                return Err(SecurityError::DnsRebindingDetected {
                    host: host_value,
                    origin: origin_value,
                });
            }
        }
    }

    Ok(())
}
```

## 3. Header Validation and Processing

### Accept Header Validation Logic

```rust
fn validate_accept_header(headers: &HeaderMap, method: &Method) -> Result<(), TransportError> {
    if let Some(value) = headers.get("accept") {
        if let Ok(accept_header) = value.to_str() {
            debug!("Validating Accept header: {accept_header}");

            // For POST (JSON-RPC) requests, Accept should be compatible with application/json
            match *method {
                Method::POST => {
                    if accept_header.contains("application/json")
                        || accept_header.contains("application/*")
                        || accept_header.contains("*/*")
                    {
                        Ok(())
                    } else {
                        warn!("Unacceptable Accept header for POST: {accept_header}");
                        Err(TransportError::UnacceptableAcceptHeader(
                            accept_header.to_string(),
                        ))
                    }
                }
                Method::GET => {
                    // For SSE requests, Accept should be compatible with text/event-stream
                    if accept_header.contains("text/event-stream")
                        || accept_header.contains("*/*")
                        || accept_header.contains("text/*")
                    {
                        Ok(())
                    } else {
                        warn!("Unacceptable Accept header for SSE: {accept_header}");
                        Err(TransportError::UnacceptableAcceptHeader(
                            accept_header.to_string(),
                        ))
                    }
                }
                _ => Ok(()), // Other methods don't have specific Accept requirements
            }
        } else {
            warn!("Invalid Accept header value");
            Err(TransportError::InvalidAcceptHeader(
                "invalid header value".to_string(),
            ))
        }
    } else {
        // Missing Accept header is acceptable (defaults based on method)
        debug!("No Accept header provided - defaulting based on method");
        Ok(())
    }
}
```

### Comprehensive Header Logging

The server implements detailed header logging for debugging:

```rust
fn log_request_headers(request_id: Uuid, headers: &HeaderMap) {
    // Helper to fetch header values as UTF-8 strings
    fn header_str(headers: &HeaderMap, name: &str) -> String {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map_or_else(|| "<missing>".to_string(), ToString::to_string)
    }

    // Short summary at info level for quick visibility
    let proto = header_str(headers, "MCP-Protocol-Version");
    let accept = header_str(headers, "accept");
    let content_type = header_str(headers, "content-type");
    let session_id = header_str(headers, MCP_SESSION_ID);
    let user_agent = header_str(headers, "user-agent");
    let origin = header_str(headers, "origin");

    info!(
        request_id = %request_id,
        protocol = %proto,
        accept = %accept,
        content_type = %content_type,
        session_id = %session_id,
        user_agent = %user_agent,
        origin = %origin,
        "Incoming request headers (summary)"
    );

    // Detailed header log at debug level with redaction and truncation
    let detailed: Vec<(String, String)> = headers
        .iter()
        .map(|(name, value)| {
            let name_str = name.as_str().to_string();
            let lower = name_str.to_ascii_lowercase();
            let mut val = match value.to_str() {
                Ok(s) => s.to_string(),
                Err(_) => "<non-utf8>".to_string(),
            };
            if SENSITIVE_HEADERS.contains(&lower.as_str()) {
                val = "<redacted>".to_string();
            } else if val.len() > 256 {
                val = format!("{}…", &val[..256]);
            }
            (name_str, val)
        })
        .collect();

    debug!(request_id = %request_id, headers = ?detailed, "Incoming request headers (detailed)");
}
```

## 4. Configuration Management

### Environment Variable Control

The server uses several environment variables to control behavior:

- `MCP_ENABLE_SSE`: Controls SSE availability on GET /mcp (defaults to disabled)
- `MCP_HOST`: Server binding address (defaults to "0.0.0.0")
- `PORT`: Server port (defaults to "3001")
- `RUST_LOG`: Logging level configuration

### Helm Chart Configuration

The Helm chart (`docs/charts/agent-docs/`) provides structured configuration:

**Default values (SSE disabled):**
```yaml
env:
  MCP_ENABLE_SSE: "0"  # SSE disabled by default
```

**SSE-enabled variant:**
```yaml
# values-sse.yaml
env:
  MCP_ENABLE_SSE: "true"  # Enable SSE for IDE clients
```

### Deployment Template

```yaml
# docs/charts/agent-docs/templates/deployment.yaml
env:
- name: MCP_ENABLE_SSE
  value: {{ .Values.env.MCP_ENABLE_SSE | quote }}
```

## 5. Protocol Version Management

### Supported Versions

The server supports protocol version `2025-06-18` with backwards compatibility for `2025-03-26`:

```rust
/// The only supported MCP protocol version (fixed for MVP)
pub const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolVersion {
    /// MCP Protocol Version 2025-03-26 (backwards compatibility)
    V2025_03_26,
    /// MCP Protocol Version 2025-06-18 (current supported version)
    V2025_06_18,
}
```

### Validation Logic

```rust
pub fn validate_protocol_version(headers: &HeaderMap) -> Result<(), StatusCode> {
    let registry = ProtocolRegistry::new();

    if let Some(value) = headers.get(MCP_PROTOCOL_VERSION) {
        value.to_str().map_or_else(
            |_| {
                warn!("Invalid protocol version header value");
                Err(StatusCode::BAD_REQUEST)
            },
            |version_str| {
                debug!(
                    "Validating protocol version: '{}' against supported versions",
                    version_str
                );
                if registry.is_version_string_supported(version_str) {
                    debug!("Protocol version '{}' is supported", version_str);
                    Ok(())
                } else {
                    warn!(
                        "Unsupported protocol version: '{}' - supported versions: 2025-03-26, {}",
                        version_str, SUPPORTED_PROTOCOL_VERSION
                    );
                    Err(StatusCode::BAD_REQUEST)
                }
            },
        )
    } else {
        // Backwards compatibility: assume 2025-03-26 when header is missing
        // as specified in MCP Streamable HTTP specification
        debug!("Missing MCP-Protocol-Version header - defaulting to 2025-03-26 for backwards compatibility");
        Ok(())
    }
}
```

## 6. Session Management

### Session State Tracking

The server implements comprehensive session management:

```rust
/// MCP session state
#[derive(Debug, Clone)]
pub struct McpSession {
    pub id: SessionId,
    pub created_at: Instant,
    pub last_activity: Arc<RwLock<Instant>>,
    pub message_sender: broadcast::Sender<SseMessage>,
}
```

### Session Lifecycle

1. **Creation**: New sessions created on first request or when existing session expires
2. **Activity Tracking**: Session activity updated on each request
3. **Expiration**: Sessions expire after configurable timeout (default 5 minutes)
4. **Cleanup**: Background task periodically removes expired sessions

## 7. Error Handling and Response Codes

### Transport Error Types

```rust
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Method not allowed")]
    MethodNotAllowed,

    #[error("Unsupported protocol version: {0}")]
    UnsupportedProtocolVersion(String),

    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),

    #[error("Session lock error")]
    SessionLockError,

    #[error("Missing content type")]
    MissingContentType,

    #[error("Invalid content type: {0}")]
    InvalidContentType(String),

    #[error("JSON parsing error: {0}")]
    JsonParseError(String),

    #[error("Payload too large")]
    PayloadTooLarge,

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Security validation failed: {0}")]
    SecurityValidationFailed(String),

    #[error("Invalid Accept header: {0}")]
    InvalidAcceptHeader(String),

    #[error("Unacceptable Accept header: {0}")]
    UnacceptableAcceptHeader(String),
}
```

### HTTP Status Code Mapping

```rust
impl IntoResponse for TransportError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed"),
            Self::UnsupportedProtocolVersion(_) => {
                (StatusCode::BAD_REQUEST, "Unsupported Protocol Version")
            }
            Self::SessionNotFound(_) => (StatusCode::BAD_REQUEST, "Session Not Found"),
            Self::InvalidSessionId(_) => (StatusCode::BAD_REQUEST, "Invalid Session ID"),
            Self::SessionLockError => (StatusCode::INTERNAL_SERVER_ERROR, "Session Lock Error"),
            Self::MissingContentType => (StatusCode::BAD_REQUEST, "Missing Content-Type"),
            Self::InvalidContentType(_) => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported Media Type")
            }
            Self::JsonParseError(_) => (StatusCode::BAD_REQUEST, "Invalid JSON"),
            Self::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "Payload Too Large"),
            Self::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
            Self::SecurityValidationFailed(_) => {
                (StatusCode::FORBIDDEN, "Security Validation Failed")
            }
            Self::InvalidAcceptHeader(_) => (StatusCode::BAD_REQUEST, "Invalid Accept Header"),
            Self::UnacceptableAcceptHeader(_) => (StatusCode::NOT_ACCEPTABLE, "Not Acceptable"),
        };
        // ... error response construction
    }
}
```

## 8. Metrics and Monitoring

### Request Metrics Tracking

```rust
pub struct McpMetrics {
    pub requests_total: AtomicU64,
    pub post_requests_success: AtomicU64,
    pub method_not_allowed_total: AtomicU64,
    pub protocol_version_errors: AtomicU64,
    pub json_parse_errors: AtomicU64,
    pub security_validation_errors: AtomicU64,
    pub internal_errors: AtomicU64,
    pub sessions_created: AtomicU64,
    pub sessions_deleted: AtomicU64,
}
```

### Health Check Endpoints

The server provides comprehensive health monitoring:

- `/health`: Basic health check
- `/health/ready`: Kubernetes readiness probe
- `/health/live`: Kubernetes liveness probe
- `/health/detailed`: Comprehensive health status with component breakdown

## 9. Acceptance Testing Integration

### Test Expectations

The acceptance tests in `scripts/acceptance-tests.sh` expect specific behavior:

```bash
# Test HTTP transport - GET method (should return 405)
test_http_transport_get() {
    log_info "Testing HTTP transport GET method (should return 405)..."
    
    local response
    local http_code
    
    response=$(curl -s -w "HTTPSTATUS:%{http_code}" \
        -X GET "$BASE_URL/mcp" \
        -H "MCP-Protocol-Version: 2025-06-18" 2>/dev/null)
    
    http_code=$(echo "$response" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')
    
    if [[ "$http_code" == "405" ]]; then
        log_success "GET method correctly returns 405 Method Not Allowed"
    else
        log_error "GET method should return 405, got: $http_code"
    fi
}
```

### SSE Enablement Impact

When SSE is enabled (`MCP_ENABLE_SSE=true`), the acceptance tests will fail because GET /mcp returns 200 instead of 405. This is expected behavior for environments where IDE clients are used.

## 10. Recommendations for Troubleshooting Guide Enhancement

### Additional Diagnostic Steps

1. **Session State Verification**: Check if sessions are being created and maintained properly
2. **Protocol Version Header Validation**: Verify exact header format and values
3. **Security Configuration Review**: Confirm allowed origins match client expectations
4. **DNS Rebinding Detection**: Check for host/origin mismatches in logs
5. **Session Timeout Issues**: Verify session expiration settings

### Enhanced Logging Recommendations

1. **Request Tracing**: Enable detailed request ID tracking through logs
2. **Header Redaction**: Ensure sensitive headers are properly redacted in logs
3. **Session Lifecycle Logging**: Track session creation, updates, and cleanup
4. **Security Event Logging**: Monitor security validation failures

### Configuration Troubleshooting

1. **Environment Variable Validation**: Verify MCP_ENABLE_SSE is set correctly
2. **Helm Chart Overrides**: Ensure proper values are applied
3. **Container Environment**: Check runtime environment variables
4. **Configuration Reload**: Verify configuration changes take effect

## 11. Common Misconfigurations

### 1. SSE Configuration Issues
- `MCP_ENABLE_SSE` not set or set to incorrect value
- Helm chart values not applied correctly
- Environment variable case sensitivity issues

### 2. Origin Validation Problems
- Client origin not in allowed origins list
- Missing Origin header when required
- DNS rebinding detection false positives

### 3. Protocol Version Issues
- Incorrect protocol version header format
- Missing protocol version header
- Unsupported protocol version usage

### 4. Accept Header Problems
- GET requests without `Accept: text/event-stream`
- POST requests with incompatible Accept headers
- Header case sensitivity issues

## 12. Advanced Debugging Techniques

### 1. Detailed Request Logging
```bash
# Enable detailed transport and security logging
RUST_LOG=info,mcp::transport=debug,mcp::security=debug,mcp::headers=debug
```

### 2. Request Tracing
```bash
# Follow specific request through logs using request_id
kubectl logs -l app=doc-server -f | grep "request_id=12345678-1234-1234-1234-123456789abc"
```

### 3. Session State Inspection
```bash
# Check session creation and management logs
kubectl logs -l app=doc-server -f | grep "session"
```

### 4. Security Validation Debugging
```bash
# Monitor security validation events
kubectl logs -l app=doc-server -f | grep "Security validation"
```

## Conclusion

The MCP server implementation provides robust support for both JSON-RPC (POST) and Streamable HTTP (GET with SSE) transports. The current troubleshooting guide covers the essential diagnostic steps, but could be enhanced with the additional implementation details and diagnostic techniques identified in this analysis.

Key areas for enhancement include:
- More detailed session management troubleshooting
- Enhanced security validation diagnostics
- Advanced logging and tracing techniques
- Configuration validation procedures

This companion document provides the technical foundation needed to improve the troubleshooting guide and help users more effectively diagnose connection issues.




