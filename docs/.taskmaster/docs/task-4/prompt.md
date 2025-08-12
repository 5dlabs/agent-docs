# Autonomous Agent Prompt: Session Management and Security Implementation

## Mission

Implement comprehensive session management with `Mcp-Session-Id` headers and robust security measures for the Doc Server MCP implementation. This task builds on the Streamable HTTP transport foundation to add essential security features and stateful session handling required for stable MCP client communication.

## Context

Following the implementation of Streamable HTTP transport in Task 2, you must now add secure session management that complies with MCP 2025-06-18 specification. This includes cryptographically secure session IDs, Origin header validation for DNS rebinding protection, and proper session lifecycle management.

## Primary Objectives

1. **Secure Session Management**: Implement UUID v4 session IDs with proper TTL and lifecycle management
2. **MCP Header Compliance**: Add bidirectional `Mcp-Session-Id` header handling per MCP specification
3. **Origin Validation**: Implement DNS rebinding protection with Origin header validation
4. **Localhost Security**: Ensure secure localhost binding for local deployments
5. **Session Lifecycle**: Add automatic expiry, cleanup, and explicit termination endpoints

## Step-by-Step Implementation

### Step 1: Session Module Foundation

1. Create `crates/mcp/src/session.rs` with core types:
   ```rust
   use chrono::{DateTime, Duration, Utc};
   use serde::{Deserialize, Serialize};
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
   ```

2. Implement Session methods:
   - `new()` with UUID v4 generation
   - `is_expired()` checking current time against TTL
   - `refresh()` updating last_accessed timestamp

3. Add comprehensive error handling:
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum SessionError {
       #[error("Session not found: {0}")]
       SessionNotFound(Uuid),
       #[error("Maximum sessions reached")]
       MaxSessionsReached,
       #[error("Lock acquisition failed")]
       LockError,
   }
   ```

### Step 2: Session Manager Implementation

1. Create `SessionManager` struct with thread-safe storage:
   ```rust
   pub struct SessionManager {
       sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
       default_ttl: Duration,
       max_sessions: usize,
   }
   ```

2. Implement core session operations:
   - `create_session()` with UUID v4 generation and session limits
   - `get_session()` with expiry checking
   - `update_last_accessed()` for activity tracking
   - `delete_session()` for explicit removal
   - `cleanup_expired_sessions()` for automatic cleanup

3. Add session metrics and monitoring:
   - `session_count()` for monitoring
   - Structured logging for all operations
   - Error tracking for failed operations

### Step 3: Security Validation Layer

1. Create `crates/mcp/src/security.rs` with security configuration:
   ```rust
   #[derive(Debug, Clone)]
   pub struct SecurityConfig {
       pub allowed_origins: HashSet<String>,
       pub strict_origin_validation: bool,
       pub localhost_only: bool,
       pub require_origin_header: bool,
   }
   ```

2. Implement Origin validation middleware:
   - `origin_validation_middleware` as Axum middleware
   - `validate_origin()` checking against allowed origins list
   - `is_localhost_origin()` for localhost pattern matching
   - DNS rebinding protection through Host header validation

3. Add secure server binding:
   - Force binding to 127.0.0.1 instead of 0.0.0.0
   - Configuration validation for security settings
   - Security headers in responses

### Step 4: Header Integration

1. Update `transport.rs` to handle `Mcp-Session-Id` headers:
   - Extract session ID from incoming requests
   - Create new sessions when ID is missing or invalid
   - Add session ID to all outgoing responses
   - Maintain session context throughout request lifecycle

2. Implement session extraction logic:
   ```rust
   async fn extract_or_create_session(
       headers: &HeaderMap,
       session_manager: &SessionManager,
   ) -> Result<Session, McpError> {
       // Try to extract existing session ID
       // Validate and retrieve session if found
       // Create new session if needed
   }
   ```

3. Add client info extraction from headers:
   - User-Agent for client identification
   - Origin for security validation
   - IP address for audit logging

### Step 5: Session Lifecycle Management

1. Implement background cleanup task:
   ```rust
   impl SessionManager {
       pub fn start_cleanup_task(&self, cleanup_interval: Duration) {
           let session_manager = Arc::clone(&Arc::new(self.clone()));
           tokio::spawn(async move {
               // Periodic cleanup of expired sessions
           });
       }
   }
   ```

2. Add DELETE endpoint for explicit session termination:
   - Handle DELETE requests to `/mcp` with session context
   - Return appropriate status codes (204 No Content, 404 Not Found)
   - Clean up associated resources

3. Implement session renewal on activity:
   - Update `last_accessed` on every valid request
   - Extend session lifetime for active connections
   - Log session activity for monitoring

### Step 6: Server Integration

1. Update `server.rs` to initialize session management:
   - Create SessionManager in server constructor
   - Start cleanup task during server initialization
   - Configure security settings

2. Integrate with transport layer:
   - Pass SessionManager through server state
   - Ensure thread-safe access across request handlers
   - Maintain backward compatibility with existing functionality

3. Add session-aware routing:
   - Include session context in all MCP handlers
   - Implement session validation middleware
   - Handle session-related errors appropriately

### Step 7: Error Handling and Recovery

1. Implement comprehensive error responses:
   - 404 for expired or non-existent sessions
   - 403 for security validation failures
   - 400 for malformed session IDs
   - 429 for session limit exceeded

2. Add graceful degradation:
   - Continue operation when sessions expire
   - Automatic session recreation for legitimate clients
   - Proper cleanup on server shutdown

3. Implement security logging:
   - Log all security validation failures
   - Track session creation and deletion
   - Monitor for suspicious activity patterns

## Required Tools

**Use these tools in this specific order:**

1. **read_file**: Examine existing transport implementation from Task 2
   - `crates/mcp/src/transport.rs` - Transport layer to integrate with
   - `crates/mcp/src/server.rs` - Server structure for integration
   - `crates/mcp/src/lib.rs` - Module organization

2. **write_file**: Create session management modules
   - `crates/mcp/src/session.rs` - Core session management
   - `crates/mcp/src/security.rs` - Security validation layer

3. **edit_file**: Update existing files for integration
   - Modify `crates/mcp/src/lib.rs` to include new modules
   - Update `crates/mcp/src/transport.rs` for session integration
   - Modify `crates/mcp/src/server.rs` for session manager initialization
   - Update `Cargo.toml` with new dependencies (chrono, uuid)

4. **write_file**: Create comprehensive test suite
   - `crates/mcp/tests/session_tests.rs` - Session management tests
   - `crates/mcp/tests/security_tests.rs` - Security validation tests
   - `crates/mcp/tests/integration_tests.rs` - End-to-end integration tests

5. **create_directory**: Ensure proper test structure
   - Verify `crates/mcp/tests/` directory exists
   - Create subdirectories if needed for organization

## Key Integration Points

1. **Transport Layer**: Seamless integration with Streamable HTTP transport from Task 2
2. **Session Headers**: Proper handling of `Mcp-Session-Id` in requests and responses
3. **Security Middleware**: Integration with Axum middleware stack
4. **Error Handling**: Consistent with existing MCP error handling patterns
5. **Logging**: Use existing tracing infrastructure for security and session events

## Success Criteria

### Functional Requirements
- [ ] Secure UUID v4 session ID generation with proper entropy
- [ ] Thread-safe session storage with configurable TTL
- [ ] Automatic cleanup of expired sessions with background task
- [ ] Bidirectional `Mcp-Session-Id` header handling
- [ ] Origin header validation with DNS rebinding protection
- [ ] Localhost binding enforcement for secure local deployment
- [ ] DELETE endpoint for explicit session termination

### Security Requirements
- [ ] DNS rebinding attack prevention through Origin validation
- [ ] Session fixation attack prevention (server-side ID generation only)
- [ ] Cryptographically secure session ID generation
- [ ] Proper CORS integration with security policies
- [ ] Security event logging for monitoring and audit
- [ ] Protection against session hijacking attempts

### Performance Requirements
- [ ] Session operations complete in < 10ms (95th percentile)
- [ ] Support for 1000+ concurrent sessions
- [ ] Memory usage linear with active session count
- [ ] Background cleanup completes in < 100ms for 1000+ sessions
- [ ] Zero memory leaks during 24-hour operation

## Testing Strategy

1. **Unit Tests**: Test each component in isolation
   - Session creation, validation, and expiry logic
   - Origin validation with various input combinations
   - Security configuration and validation
   - Error handling for all failure modes

2. **Integration Tests**: Test complete request/response cycles
   - Session creation during MCP request processing
   - Header propagation across multiple requests
   - Session cleanup and lifecycle management
   - Security validation with real HTTP requests

3. **Security Tests**: Validate protection mechanisms
   - DNS rebinding attack simulation
   - Session hijacking attempt prevention
   - Origin spoofing attack blocking
   - CORS policy enforcement

4. **Performance Tests**: Validate scalability and performance
   - Concurrent session handling (500+ simultaneous)
   - Session cleanup performance under load
   - Memory leak detection over extended periods
   - Response time benchmarking

## Critical Implementation Notes

1. **Thread Safety**: All session operations must be thread-safe using proper locking
2. **Security First**: Implement security validations before functional features
3. **Session Persistence**: Use in-memory storage with note for future Redis integration
4. **Error Recovery**: Graceful handling of all error conditions
5. **Monitoring**: Comprehensive logging for security and operational events

## Focus Areas

- **Security**: DNS rebinding protection and Origin validation are critical
- **Performance**: Session operations must not impact MCP request latency
- **Reliability**: Session management must be robust under high concurrency
- **Integration**: Seamless integration with existing transport and handler infrastructure

## Expected Deliverables

1. **Session Management Module**: Complete session.rs implementation with lifecycle management
2. **Security Module**: Comprehensive security.rs with Origin validation and DNS protection
3. **Transport Integration**: Updated transport.rs with session header handling
4. **Server Integration**: Modified server.rs with session manager initialization
5. **Test Suite**: Complete unit, integration, and security tests
6. **Documentation**: Security guide and session management API documentation

This implementation is critical for secure MCP communication and must be thoroughly tested before deployment to production environments.