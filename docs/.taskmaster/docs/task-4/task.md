# Task 4: Session Management and Security Implementation

## Overview

This task implements comprehensive session management with `Mcp-Session-Id` headers and robust security measures for the Doc Server MCP implementation. Building on the Streamable HTTP transport foundation from Task 2, this adds essential security features including Origin header validation, DNS rebinding protection, and secure session lifecycle management.

## Background

Session management is critical for maintaining stateful MCP connections and ensuring secure communication between clients and the Doc Server. The MCP 2025-06-18 specification requires proper session handling with unique identifiers, while security best practices demand protection against common web vulnerabilities like DNS rebinding attacks and session hijacking.

### Key Requirements

- **Secure Session IDs**: Cryptographically secure UUID v4 generation
- **Header Compliance**: Proper `Mcp-Session-Id` header handling per MCP specification
- **Origin Validation**: Protection against DNS rebinding attacks
- **Session Lifecycle**: Automatic expiry and cleanup mechanisms
- **Localhost Binding**: Secure local deployment configuration
- **Concurrency Safety**: Thread-safe session management for multiple clients

## Implementation Guide

### Phase 1: Session Module and Data Structures

#### 1.1 Create Session Data Model

Create `crates/mcp/src/session.rs` with core session structures:

```rust
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub ttl: Duration,
    pub client_info: Option<ClientInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub user_agent: Option<String>,
    pub origin: Option<String>,
    pub ip_address: Option<String>,
}

impl Session {
    pub fn new(ttl: Option<Duration>) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4(),
            created_at: now,
            last_accessed: now,
            ttl: ttl.unwrap_or_else(|| Duration::minutes(30)),
            client_info: None,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        now - self.last_accessed > self.ttl
    }
    
    pub fn refresh(&mut self) {
        self.last_accessed = Utc::now();
    }
}
```

#### 1.2 Implement Session Manager

```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    default_ttl: Duration,
    max_sessions: usize,
}

impl SessionManager {
    pub fn new(default_ttl: Duration, max_sessions: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
            max_sessions,
        }
    }
    
    pub fn create_session(&self, client_info: Option<ClientInfo>) -> Result<Session, SessionError> {
        let mut sessions = self.sessions.write().map_err(|_| SessionError::LockError)?;
        
        if sessions.len() >= self.max_sessions {
            return Err(SessionError::MaxSessionsReached);
        }
        
        let mut session = Session::new(Some(self.default_ttl));
        session.client_info = client_info;
        
        sessions.insert(session.session_id, session.clone());
        
        tracing::info!("Created new session: {}", session.session_id);
        Ok(session)
    }
    
    pub fn get_session(&self, session_id: Uuid) -> Result<Option<Session>, SessionError> {
        let sessions = self.sessions.read().map_err(|_| SessionError::LockError)?;
        
        if let Some(session) = sessions.get(&session_id) {
            if session.is_expired() {
                drop(sessions);
                self.delete_session(session_id)?;
                return Ok(None);
            }
            Ok(Some(session.clone()))
        } else {
            Ok(None)
        }
    }
    
    pub fn update_last_accessed(&self, session_id: Uuid) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().map_err(|_| SessionError::LockError)?;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.refresh();
            tracing::debug!("Updated last_accessed for session: {}", session_id);
        }
        
        Ok(())
    }
    
    pub fn delete_session(&self, session_id: Uuid) -> Result<bool, SessionError> {
        let mut sessions = self.sessions.write().map_err(|_| SessionError::LockError)?;
        
        let removed = sessions.remove(&session_id).is_some();
        if removed {
            tracing::info!("Deleted session: {}", session_id);
        }
        
        Ok(removed)
    }
    
    pub fn cleanup_expired_sessions(&self) -> Result<usize, SessionError> {
        let mut sessions = self.sessions.write().map_err(|_| SessionError::LockError)?;
        
        let initial_count = sessions.len();
        sessions.retain(|_, session| !session.is_expired());
        let removed_count = initial_count - sessions.len();
        
        if removed_count > 0 {
            tracing::info!("Cleaned up {} expired sessions", removed_count);
        }
        
        Ok(removed_count)
    }
    
    pub fn session_count(&self) -> Result<usize, SessionError> {
        let sessions = self.sessions.read().map_err(|_| SessionError::LockError)?;
        Ok(sessions.len())
    }
}
```

### Phase 2: Security Validation Layer

#### 2.1 Create Security Module

Create `crates/mcp/src/security.rs` with Origin validation:

```rust
use axum::{
    extract::{ConnectInfo, Request},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashSet;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub allowed_origins: HashSet<String>,
    pub strict_origin_validation: bool,
    pub localhost_only: bool,
    pub require_origin_header: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        let mut allowed_origins = HashSet::new();
        allowed_origins.insert("http://localhost".to_string());
        allowed_origins.insert("http://127.0.0.1".to_string());
        allowed_origins.insert("https://localhost".to_string());
        allowed_origins.insert("https://127.0.0.1".to_string());
        
        Self {
            allowed_origins,
            strict_origin_validation: true,
            localhost_only: true,
            require_origin_header: false,
        }
    }
}

pub async fn origin_validation_middleware(
    security_config: axum::extract::State<SecurityConfig>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = request.headers();
    
    // Check Origin header if present or required
    if let Some(origin) = headers.get("origin") {
        if let Ok(origin_str) = origin.to_str() {
            if !validate_origin(origin_str, &security_config) {
                tracing::warn!("Blocked request with invalid origin: {}", origin_str);
                return Err(StatusCode::FORBIDDEN);
            }
        } else {
            tracing::warn!("Invalid Origin header format");
            return Err(StatusCode::BAD_REQUEST);
        }
    } else if security_config.require_origin_header {
        tracing::warn!("Missing required Origin header");
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Validate Host header for DNS rebinding protection
    if let Some(host) = headers.get("host") {
        if let Ok(host_str) = host.to_str() {
            if security_config.localhost_only && !is_localhost_host(host_str) {
                tracing::warn!("Blocked request with non-localhost host: {}", host_str);
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }
    
    Ok(next.run(request).await)
}

fn validate_origin(origin: &str, config: &SecurityConfig) -> bool {
    if !config.strict_origin_validation {
        return true;
    }
    
    // Check against allowed origins list
    if config.allowed_origins.contains(origin) {
        return true;
    }
    
    // Check for localhost patterns
    if config.localhost_only {
        return is_localhost_origin(origin);
    }
    
    false
}

fn is_localhost_origin(origin: &str) -> bool {
    origin.starts_with("http://localhost")
        || origin.starts_with("https://localhost")
        || origin.starts_with("http://127.0.0.1")
        || origin.starts_with("https://127.0.0.1")
        || origin.starts_with("http://[::1]")
        || origin.starts_with("https://[::1]")
}

fn is_localhost_host(host: &str) -> bool {
    host.starts_with("localhost")
        || host.starts_with("127.0.0.1")
        || host.starts_with("[::1]")
}
```

#### 2.2 Implement Server Binding Security

```rust
// In server.rs
impl McpServer {
    pub async fn bind_secure(&self, port: u16) -> Result<(), ServerError> {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        
        tracing::info!("Binding server to secure localhost address: {}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| ServerError::BindError(e.to_string()))?;
        
        let app = self.create_router_with_security();
        
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>()
        ).await
            .map_err(|e| ServerError::ServerError(e.to_string()))?;
        
        Ok(())
    }
    
    fn create_router_with_security(&self) -> Router {
        let security_config = SecurityConfig::default();
        
        self.router()
            .layer(axum::middleware::from_fn_with_state(
                security_config,
                origin_validation_middleware
            ))
    }
}
```

### Phase 3: Mcp-Session-Id Header Handling

#### 3.1 Update Transport Layer

Modify `transport.rs` to handle session headers:

```rust
// In transport.rs
use crate::session::{Session, SessionManager};

const MCP_SESSION_ID_HEADER: &str = "Mcp-Session-Id";

pub async fn unified_mcp_handler(
    State(state): State<Arc<McpServerState>>,
    headers: HeaderMap,
    request: Request<Body>,
) -> Result<Response<Body>, McpError> {
    // Extract or create session
    let session = extract_or_create_session(&headers, &state.session_manager).await?;
    
    // Update session activity
    state.session_manager.update_last_accessed(session.session_id)
        .map_err(|e| McpError::SessionError(e.to_string()))?;
    
    // Process request with session context
    let mut response = match request.method() {
        &Method::POST => handle_json_rpc_request(state, session.clone(), request).await?,
        &Method::GET => handle_sse_stream_request(state, session.clone(), headers).await?,
        &Method::DELETE => handle_session_delete_request(state, session.clone()).await?,
        _ => return Err(McpError::MethodNotAllowed),
    }?;
    
    // Add session header to response
    response.headers_mut().insert(
        MCP_SESSION_ID_HEADER,
        HeaderValue::from_str(&session.session_id.to_string())
            .map_err(|_| McpError::HeaderError)?
    );
    
    Ok(response)
}

async fn extract_or_create_session(
    headers: &HeaderMap,
    session_manager: &SessionManager,
) -> Result<Session, McpError> {
    // Try to extract session ID from headers
    if let Some(session_id_header) = headers.get(MCP_SESSION_ID_HEADER) {
        if let Ok(session_id_str) = session_id_header.to_str() {
            if let Ok(session_id) = Uuid::parse_str(session_id_str) {
                // Try to get existing session
                if let Some(session) = session_manager.get_session(session_id)
                    .map_err(|e| McpError::SessionError(e.to_string()))? {
                    return Ok(session);
                }
                // Session not found or expired, create new one
            }
        }
    }
    
    // Create new session
    let client_info = extract_client_info(headers);
    session_manager.create_session(client_info)
        .map_err(|e| McpError::SessionError(e.to_string()))
}

fn extract_client_info(headers: &HeaderMap) -> Option<ClientInfo> {
    Some(ClientInfo {
        user_agent: headers.get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string()),
        origin: headers.get("origin")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string()),
        ip_address: None, // Can be extracted from ConnectInfo if needed
    })
}
```

### Phase 4: Session Lifecycle Management

#### 4.1 Implement Session Cleanup Task

```rust
// In session.rs
impl SessionManager {
    pub fn start_cleanup_task(&self, cleanup_interval: Duration) {
        let session_manager = Arc::clone(&Arc::new(self.clone()));
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            
            loop {
                interval.tick().await;
                
                match session_manager.cleanup_expired_sessions() {
                    Ok(removed_count) => {
                        if removed_count > 0 {
                            tracing::debug!("Cleanup task removed {} expired sessions", removed_count);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Session cleanup failed: {:?}", e);
                    }
                }
            }
        });
        
        tracing::info!("Started session cleanup task with interval: {:?}", cleanup_interval);
    }
}
```

#### 4.2 Add DELETE Endpoint for Session Termination

```rust
// In transport.rs
async fn handle_session_delete_request(
    state: Arc<McpServerState>,
    session: Session,
) -> Result<Response<Body>, McpError> {
    // Delete the session
    let deleted = state.session_manager.delete_session(session.session_id)
        .map_err(|e| McpError::SessionError(e.to_string()))?;
    
    if deleted {
        Ok(Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("Content-Type", "application/json")
            .body(Body::empty())?)
    } else {
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&json!({
                "error": "Session not found"
            }))?.into())?)
    }
}
```

### Phase 5: Error Handling and Types

#### 5.1 Define Session Error Types

```rust
// In session.rs
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),
    
    #[error("Session expired: {0}")]
    SessionExpired(Uuid),
    
    #[error("Maximum number of sessions reached")]
    MaxSessionsReached,
    
    #[error("Lock acquisition failed")]
    LockError,
    
    #[error("Invalid session ID format: {0}")]
    InvalidSessionId(String),
    
    #[error("Session configuration error: {0}")]
    ConfigError(String),
}
```

## Technical Requirements

### Dependencies

Update `Cargo.toml` with required dependencies:

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
tokio = { version = "1.0", features = ["time", "sync"] }
tracing = "0.1"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Configuration

```rust
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub default_ttl: Duration,
    pub max_sessions: usize,
    pub cleanup_interval: Duration,
    pub strict_expiry: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::minutes(30),
            max_sessions: 1000,
            cleanup_interval: Duration::minutes(5),
            strict_expiry: true,
        }
    }
}
```

## Code Examples

### Session Creation and Management

```rust
// Example usage in server initialization
let session_config = SessionConfig::default();
let session_manager = SessionManager::new(
    session_config.default_ttl,
    session_config.max_sessions,
);

// Start background cleanup task
session_manager.start_cleanup_task(session_config.cleanup_interval);

// Create session during request handling
let client_info = ClientInfo {
    user_agent: Some("MCP Client 1.0".to_string()),
    origin: Some("http://localhost:3000".to_string()),
    ip_address: Some("127.0.0.1".to_string()),
};

let session = session_manager.create_session(Some(client_info))?;
tracing::info!("Created session: {}", session.session_id);
```

### Origin Validation

```rust
// Security configuration
let mut security_config = SecurityConfig::default();
security_config.allowed_origins.insert("https://app.example.com".to_string());
security_config.strict_origin_validation = true;

// Apply as middleware
let app = Router::new()
    .route("/mcp", post(unified_mcp_handler))
    .layer(axum::middleware::from_fn_with_state(
        security_config,
        origin_validation_middleware
    ));
```

## Dependencies

- **Task 2**: Streamable HTTP Transport Foundation must be implemented
- **rmcp ≥ 0.5**: Ensure protocol objects align with updated MCP libraries
- **Chrono**: Date/time handling for session timestamps and TTL
- **UUID**: Cryptographically secure session ID generation
- **Tokio**: Async runtime for cleanup tasks and concurrency
- **Axum**: Web framework integration for middleware and headers

## Risk Considerations

### Security Risks

1. **Session Hijacking**: Weak session ID generation or transmission
   - **Mitigation**: Use cryptographically secure UUID v4 generation
   - **Validation**: Test with security scanning tools

2. **DNS Rebinding Attacks**: Malicious websites accessing local server
   - **Mitigation**: Strict Origin header validation and localhost binding
   - **Testing**: Simulate rebinding attacks in test environment

3. **Session Fixation**: Attacker sets known session ID
   - **Mitigation**: Always generate new session IDs server-side
   - **Validation**: Never accept client-provided session IDs for creation

4. **Cross-Origin Request Forgery**: Unauthorized cross-origin requests
   - **Mitigation**: Proper CORS configuration and Origin validation
   - **Testing**: Verify CORS policies with various origin combinations

### Performance Risks

1. **Memory Leaks**: Unbounded session growth
   - **Solution**: Configurable session limits and automatic cleanup
   - **Monitoring**: Track session count and memory usage metrics

2. **Lock Contention**: High contention on session storage
   - **Solution**: Consider using more granular locking or concurrent maps
   - **Benchmarking**: Test with high concurrent session operations

### Operational Risks

1. **Session Loss**: Server restart causes all sessions to expire
   - **Future Solution**: Redis or persistent storage for production
   - **Mitigation**: Document session persistence requirements

## Success Metrics

### Security Metrics

- **Origin Validation**: 100% blocking of non-allowed origins
- **Session Security**: Zero successful session hijacking attempts in security tests
- **DNS Rebinding Protection**: Complete blocking of rebinding attack vectors
- **Header Compliance**: Proper `Mcp-Session-Id` handling in all requests/responses

### Performance Metrics

- **Session Creation**: < 10ms per session creation operation
- **Session Lookup**: < 1ms per session retrieval operation
- **Cleanup Performance**: < 100ms to clean 1000+ expired sessions
- **Memory Usage**: Linear scaling with active session count
- **Concurrent Handling**: Support for 500+ concurrent session operations

### Functional Metrics

- **Session Lifecycle**: 100% proper session creation, access, and expiry
- **Error Handling**: Appropriate HTTP status codes for all error conditions
- **Header Processing**: Correct bidirectional header handling
- **Integration**: Seamless integration with existing MCP transport layer

## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/task-4-session-security`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.

## Timeline

### Week 1: Foundation (Days 1-3)
- Session module and data structures
- Security validation layer
- Basic Origin header validation

### Week 2: Integration (Days 4-6)
- Mcp-Session-Id header handling
- Transport layer integration
- Session lifecycle management

### Week 2: Testing and Security (Days 7-10)
- Comprehensive testing suite
- Security audit and penetration testing
- Performance benchmarking
- Documentation and deployment guides

## Validation Criteria

### Unit Tests

- Session creation, retrieval, and expiry logic
- Origin validation with various input combinations
- UUID generation and validation
- Error handling for all edge cases

### Integration Tests

- End-to-end session lifecycle with MCP client
- Security validation with simulated attacks
- Concurrent session handling under load
- Header propagation across request/response cycles

### Security Tests

- DNS rebinding attack prevention
- Session hijacking attempt blocking
- Origin validation with malicious inputs
- CORS policy enforcement

### Production Validation

- Kubernetes deployment with secure session management
- Real-world testing with Cursor and Toolman clients
- Performance monitoring under production load
- Security scanning and vulnerability assessment

## Documentation

- **Session Management API**: Complete documentation of session operations
- **Security Guide**: Best practices for secure deployment
- **Configuration Reference**: All session and security configuration options
- **Troubleshooting Guide**: Common issues and resolution procedures

## Notes from Assessment
- Implement `Mcp-Session-Id` end-to-end; add DELETE to terminate sessions
- Enforce 30s heartbeat/90s timeout behavior on streams
- Origin validation and localhost binding recommended for single-user setup