# Task 3: Implement Streamable HTTP Transport Foundation

## Overview

This task implements the critical transition from the deprecated HTTP+SSE transport (protocol version 2024-11-05) to the new Streamable HTTP transport (protocol version 2025-06-18) following the MCP specification. This forms the foundation for reliable MCP server communication and is essential for maintaining compatibility with modern MCP clients like Toolman and Cursor.

## Background

The current Doc Server implementation uses the deprecated HTTP+SSE transport that is no longer supported in the latest MCP specification. This creates stability and compatibility issues with modern MCP clients. The new Streamable HTTP transport provides:

- **Unified Endpoint Architecture (MVP)**: Single `/mcp` endpoint supporting POST (JSON) only; GET returns 405
- **Improved Session Management**: Proper session tracking with `Mcp-Session-Id` headers
- **Enhanced Reliability**: Better connection handling and message delivery guarantees
- **Protocol Compliance**: Full compliance with MCP 2025-06-18 specification
- **No Legacy Compatibility (MVP)**: Require latest protocol header; reject others

## Implementation Guide

### Phase 1: Core Transport Module Structure

#### 1.1 Create Base Transport Module

Create `crates/mcp/src/transport.rs` with the foundational structure:

```rust
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::post,
    Json, Router
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

// Core transport configuration
#[derive(Clone, Debug)]
pub struct TransportConfig {
    pub protocol_version: String,
    pub session_timeout: Duration,
    pub heartbeat_interval: Duration,
}

// Session management
pub type SessionId = Uuid;

#[derive(Debug, Clone)]
pub struct McpSession {
    pub id: SessionId,
    pub created_at: Instant,
    pub last_activity: Arc<RwLock<Instant>>,
    pub message_sender: broadcast::Sender<SseMessage>,
}

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, McpSession>>>,
    config: TransportConfig,
}
```

#### 1.2 Define Transport Constants

```rust
// MCP Protocol Headers
pub const MCP_PROTOCOL_VERSION: &str = "MCP-Protocol-Version";
pub const MCP_SESSION_ID: &str = "Mcp-Session-Id";
pub const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";
// Content Types
pub const APPLICATION_JSON: &str = "application/json";
```

### Phase 2: Unified MCP Endpoint Handler

#### 2.1 Implement Main Handler

```rust
pub async fn unified_mcp_handler(
    State(state): State<Arc<McpServerState>>,
    headers: HeaderMap,
    request: Request<Body>,
) -> Result<Response<Body>, McpError> {
    // Extract protocol version and session ID
    let protocol_version = extract_protocol_version(&headers)?;
    let session_id = extract_or_create_session_id(&headers, &state.transport).await?;
    
    // Route based on HTTP method and content negotiation
    match request.method() {
        &Method::POST => handle_json_rpc_request(state, session_id, request).await,
        &Method::GET => Err(McpError::MethodNotAllowed),
        _ => Err(McpError::MethodNotAllowed),
    }
}
```

#### 2.2 Implement Request Processing

```rust
async fn handle_json_rpc_request(
    state: Arc<McpServerState>,
    session_id: SessionId,
    mut request: Request<Body>,
) -> Result<Response<Body>, McpError> {
    let body = hyper::body::to_bytes(request.into_body()).await?;
    let json_rpc: JsonRpcMessage = serde_json::from_slice(&body)?;
    
    // Process through existing MCP handler
    let response = state.handler.handle_message(json_rpc).await?;
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", APPLICATION_JSON)
        .header(MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSION)
        .header(MCP_SESSION_ID, session_id.to_string())
        .body(serde_json::to_vec(&response)?.into())?)
}
```

### (Removed) Phase 3: SSE Streaming Infrastructure (out of scope for MVP)

```rust
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;

pub async fn handle_sse_stream_request(
    state: Arc<McpServerState>,
    session_id: SessionId,
    headers: HeaderMap,
) -> Result<Response<Body>, McpError> {
    // Validate Accept header for SSE
    if !accepts_event_stream(&headers) {
        return Err(McpError::NotAcceptable);
    }
    
    // Create or get existing session
    let session = state.transport.get_or_create_session(session_id).await?;
    
    // Create SSE stream
    let stream = create_sse_stream(session.message_sender.subscribe())
        .map_err(|e| McpError::StreamError(e.to_string()))?;
    
    let sse = Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("heartbeat"));
    
    Ok(sse.into_response())
}

fn create_sse_stream(
    mut receiver: broadcast::Receiver<SseMessage>,
) -> impl Stream<Item = Result<Event, Box<dyn Error + Send + Sync>>> {
    async_stream::stream! {
        while let Ok(message) = receiver.recv().await {
            let event = Event::default()
                .id(message.id.to_string())
                .data(message.data);
            
            match message.event_type.as_deref() {
                Some(event_type) => yield Ok(event.event(event_type)),
                None => yield Ok(event),
            }
        }
    }
}
```

### (Removed) Phase 4: Backward Compatibility Detection (out of scope for MVP)

```rust
pub fn detect_legacy_transport(headers: &HeaderMap) -> bool {
    // Check for missing protocol version (legacy indicator)
    if headers.get(MCP_PROTOCOL_VERSION).is_none() {
        return true;
    }
    
    // Check for legacy protocol version
    if let Some(version) = headers.get(MCP_PROTOCOL_VERSION) {
        if let Ok(version_str) = version.to_str() {
            return version_str == LEGACY_PROTOCOL_VERSION;
        }
    }
    
    false
}

pub async fn handle_legacy_transport(
    headers: &HeaderMap,
    method: &Method,
) -> Result<Response<Body>, McpError> {
    tracing::warn!("Legacy transport detected, returning compatibility response");
    
    // Return appropriate error for legacy clients
    Ok(Response::builder()
        .status(StatusCode::UPGRADE_REQUIRED)
        .header("Content-Type", APPLICATION_JSON)
        .body(serde_json::to_vec(&json!({
            "error": "Transport upgrade required",
            "supported_version": SUPPORTED_PROTOCOL_VERSION,
            "deprecated_version": LEGACY_PROTOCOL_VERSION
        }))?.into())?)
}
```

### Phase 5: Server Integration

#### 5.1 Update Main Server

```rust
// In crates/mcp/src/server.rs
impl McpServer {
    pub fn new() -> Self {
        let transport = Arc::new(TransportManager::new(TransportConfig {
            protocol_version: SUPPORTED_PROTOCOL_VERSION.to_string(),
            session_timeout: Duration::from_secs(300),
            heartbeat_interval: Duration::from_secs(30),
        }));
        
        let state = Arc::new(McpServerState {
            handler: Arc::new(McpHandler::new()),
            transport,
        });
        
        Self { state }
    }
    
    pub fn router(&self) -> Router {
        Router::new()
            .route("/mcp", post(unified_mcp_handler))
            .route("/health", get(health_check))
            .layer(CorsLayer::permissive())
            .with_state(Arc::clone(&self.state))
    }
}
```

## Technical Requirements

### Dependencies

```toml
[dependencies]
axum = { version = "0.7", features = ["ws", "headers"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
async-stream = "0.3"
tokio-stream = { version = "0.1", features = ["sync"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4"] }
tracing = "0.1"
```

### Configuration

```rust
pub struct TransportConfig {
    pub protocol_version: String,
    pub session_timeout: Duration,
    pub heartbeat_interval: Duration,
    pub max_sessions: usize,
    pub message_buffer_size: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            protocol_version: "2025-06-18".to_string(),
            session_timeout: Duration::from_secs(300),
            heartbeat_interval: Duration::from_secs(30),
            max_sessions: 1000,
            message_buffer_size: 100,
        }
    }
}
```

## Code Examples

### Session Management

```rust
impl SessionManager {
    pub async fn create_session(&self) -> Result<SessionId, McpError> {
        let session_id = Uuid::new_v4();
        let (sender, _) = broadcast::channel(self.config.message_buffer_size);
        
        let session = McpSession {
            id: session_id,
            created_at: Instant::now(),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            message_sender: sender,
        };
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session);
        
        Ok(session_id)
    }
    
    pub async fn cleanup_expired_sessions(&self) -> Result<usize, McpError> {
        let mut sessions = self.sessions.write().await;
        let now = Instant::now();
        let expired_count = sessions.len();
        
        sessions.retain(|_, session| {
            let last_activity = *session.last_activity.read().unwrap();
            now.duration_since(last_activity) < self.config.session_timeout
        });
        
        Ok(expired_count - sessions.len())
    }
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum McpTransportError {
    #[error("Protocol version not supported: {0}")]
    UnsupportedProtocolVersion(String),
    
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),
    
    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),
    
    #[error("Stream error: {0}")]
    StreamError(String),
    
    #[error("Transport configuration error: {0}")]
    ConfigError(String),
}
```

## Dependencies

- **Task 1**: Basic MCP server infrastructure must be in place
- **rmcp ≥ 0.5**: Upgrade workspace dependency for MCP 2025-06-18 support
- **Axum 0.7**: Web framework for HTTP handling
- **Tower-HTTP**: Middleware for CORS and tracing
- **Tokio**: Async runtime and streaming utilities
- **UUID**: Session ID generation

## Risk Considerations

### Technical Risks

1. **Protocol Compatibility**: Ensuring full compliance with MCP 2025-06-18
   - **Mitigation**: Comprehensive testing against official MCP specification
   - **Validation**: Integration tests with multiple MCP clients

2. **Session Management Complexity**: Memory leaks from uncleaned sessions
   - **Mitigation**: Automatic session cleanup with configurable timeouts
   - **Monitoring**: Session count metrics and cleanup logging

3. **SSE Connection Stability**: Network interruptions causing stream failures
   - **Mitigation**: Proper error handling and reconnection logic
   - **Testing**: Network simulation tests with connection drops

4. **Backward Compatibility**: Breaking existing client integrations
   - **Mitigation**: Graceful legacy transport detection and helpful error messages
   - **Rollback**: Configuration flag to temporarily re-enable old transport

### Performance Risks

1. **Memory Usage**: Unbounded session growth
   - **Solution**: Session limits and automatic cleanup
   - **Monitoring**: Memory usage tracking per session

2. **Connection Overhead**: High SSE connection count
   - **Solution**: Connection pooling and efficient stream management
   - **Benchmarking**: Load testing with 100+ concurrent connections

## Success Metrics

### Functional Metrics

- **Protocol Compliance**: 100% compliance with MCP 2025-06-18 specification
- **Transport Reliability**: 99.9% successful message delivery rate
- **Session Management**: Zero memory leaks in 24-hour stress tests
- **Backward Compatibility**: Graceful handling of legacy transport attempts

### Performance Metrics

- **Response Time**: < 100ms for JSON-RPC request processing
- **Connection Setup**: < 50ms for SSE stream initialization
- **Memory Usage**: < 10MB overhead for 100 concurrent sessions
- **CPU Usage**: < 5% CPU for normal operation load

### Integration Metrics

- **Client Compatibility**: Successful integration with Cursor and Toolman
- **Connection Stability**: 99% uptime for long-running sessions
- **Error Recovery**: < 1s reconnection time after network interruption

## Timeline

### Week 1: Foundation (Days 1-2)
- Core transport module structure
- Session management implementation
- Basic error handling

### Week 1: Handler Implementation (Days 3-4)
- Unified MCP endpoint handler
- JSON-RPC request processing
- Content negotiation logic

### Week 2: Streaming (Days 5-7)
- SSE streaming infrastructure
- Event formatting and delivery
- Connection management

### Week 2: Integration (Days 8-10)
- Server integration
- Backward compatibility
- Testing and validation

## Validation Criteria

### Unit Tests

- Session creation and cleanup
- Protocol version detection
- Message serialization/deserialization
- Error handling scenarios

### Integration Tests

- End-to-end JSON-RPC communication
- SSE stream functionality
- Multiple concurrent sessions
- Legacy transport detection

### Production Validation

- Kubernetes deployment with new transport
- Real-world testing with Cursor integration
- Performance benchmarking under load
- Monitoring and alerting verification

## Documentation

- **Transport Architecture**: Complete documentation of the new transport layer
- **Migration Guide**: Step-by-step guide for upgrading from legacy transport
- **API Reference**: Detailed documentation of all transport endpoints
- **Troubleshooting Guide**: Common issues and resolution procedures

## Notes from Assessment
- Current transport is a placeholder; implement full Streamable HTTP
- SSE module references break compilation today; treat SSE only as stream response
- Protocol version must be negotiated and echoed; add headers consistently
