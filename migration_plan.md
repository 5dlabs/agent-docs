# Doc Server Migration Plan: HTTP+SSE to Streamable HTTP

**Date**: 2024-12-28  
**Target**: MCP 2025-06-18 Streamable HTTP Compliance  
**Timeline**: 3-4 weeks development effort  

## Migration Overview

This plan outlines the step-by-step migration from the current broken HTTP+SSE implementation to full MCP 2025-06-18 Streamable HTTP compliance. The migration addresses critical implementation gaps while maintaining the existing database schema and deployment infrastructure.

## Current State vs. Target State

| Component | Current State | Target State | Migration Required |
|-----------|---------------|--------------|-------------------|
| Transport Layer | 3-line placeholder | Full Streamable HTTP | Complete rewrite |
| SSE Module | Missing (breaks compilation) | Working SSE with keep-alive | New implementation |
| Session Management | None | Mcp-Session-Id support | New feature |
| Protocol Version | 2024-11-05 | 2025-06-18 | Library upgrade |
| rmcp Library | v0.1.5 | v0.5.0+ | Dependency update |

## Migration Phases

### Phase 1: Foundation Repair (Week 1)
*Priority: P0 - Critical (Blocks compilation)*

#### 1.1 Create Missing SSE Module
**Files to create**: `crates/mcp/src/sse.rs`

```rust
// Required SSE functionality
pub struct SSEConfig {
    pub heartbeat_interval: Duration,
    pub connection_timeout: Duration,
    pub message_buffer_size: usize,
}

pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<Uuid, Connection>>>,
    config: SSEConfig,
}

pub struct HeartbeatService {
    connection_manager: Arc<ConnectionManager>,
    config: SSEConfig,
}

pub async fn sse_handler(
    State(state): State<McpServerState>,
) -> Result<Response<Body>, StatusCode> {
    // SSE endpoint implementation
}
```

**Implementation Requirements**:
- Connection lifecycle management
- Message buffering system
- Heartbeat scheduling
- Connection timeout handling
- Thread-safe concurrent access

**Acceptance Criteria**:
- [ ] Module compiles successfully
- [ ] Basic SSE endpoint responds with proper headers
- [ ] Connection manager can add/remove connections
- [ ] Heartbeat service sends periodic messages

#### 1.2 Implement Transport Layer
**Files to modify**: `crates/mcp/src/transport.rs`

**Current**:
```rust
//! MCP transport layer

// TODO: Implement HTTP/SSE transport
```

**Target**:
```rust
//! MCP transport layer implementation for Streamable HTTP

pub trait Transport {
    async fn send_message(&self, message: Value) -> Result<()>;
    async fn receive_message(&self) -> Result<Option<Value>>;
    fn connection_id(&self) -> Uuid;
}

pub struct StreamableHttpTransport {
    connection_id: Uuid,
    sender: Option<SseSender>,
    receiver: Option<Receiver<Value>>,
    session_id: Option<String>,
}
```

**Implementation Steps**:
1. Define transport trait interface
2. Implement Streamable HTTP transport
3. Add session management support
4. Integrate with existing server

**Risk Mitigation**:
- Incremental implementation with feature flags
- Comprehensive unit tests for each component
- Backward compatibility during transition

#### 1.3 Fix Server Compilation
**Files to modify**: `crates/mcp/src/server.rs`

**Issues to resolve**:
- Remove broken SSE import: `use crate::sse::sse_handler;`
- Implement proper SSE handler registration
- Add session header handling
- Update CORS configuration for new headers

**Expected Outcome**: Project compiles successfully

### Phase 2: MCP Protocol Upgrade (Week 2)
*Priority: P0 - Critical (Protocol compliance)*

#### 2.1 Upgrade rmcp Dependency
**Files to modify**: `Cargo.toml`, all crate `Cargo.toml` files

**Current**: `rmcp = "0.1"`  
**Target**: `rmcp = "0.5"`

**Migration Steps**:
1. Update workspace dependency
2. Review breaking changes in rmcp changelog
3. Update MCP message handling code
4. Update protocol version declarations

**Breaking Changes Expected**:
- Message structure changes
- New capability declarations
- Enhanced error handling
- Session management APIs

#### 2.2 Implement Streamable HTTP Features

**Session Management**:
```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

pub struct Session {
    id: String,
    created_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    capabilities: SessionCapabilities,
    transport: Box<dyn Transport>,
}
```

**Headers to support**:
- `Mcp-Protocol-Version: 2025-06-18`
- `Mcp-Session-Id: <uuid>`
- `Accept: application/json,text/event-stream`

**Keep-Alive Implementation**:
- 30-second heartbeat intervals
- 90-second connection timeout
- Automatic reconnection support
- Message buffering during disconnections

#### 2.3 Update Server Capabilities
**Files to modify**: `crates/mcp/src/handlers.rs`

**Current capabilities**:
```json
{
  "protocolVersion": "2024-11-05",
  "capabilities": {
    "tools": {},
    "sse": true
  }
}
```

**Target capabilities**:
```json
{
  "protocolVersion": "2025-06-18", 
  "capabilities": {
    "tools": {},
    "transport": {
      "streamableHttp": true,
      "keepAlive": true,
      "sessionManagement": true
    }
  }
}
```

### Phase 3: Advanced Features (Week 3)
*Priority: P1 - High (Production readiness)*

#### 3.1 Connection Resilience
**Features to implement**:
- Exponential backoff for reconnections
- Message deduplication
- Connection state monitoring
- Health check improvements

**File**: `crates/mcp/src/connection.rs` (new)

#### 3.2 Performance Optimizations
**Database Layer**:
- Consider dimension reduction for vector indexing
- Implement connection pooling optimizations
- Add query caching layer

**Embeddings**:
- Implement batch processing (100 embeddings per request)
- Add rate limiting (3,000 RPM / 1M TPM)
- Queue management with retry logic

**File**: `crates/embeddings/src/batch.rs`

#### 3.3 Monitoring and Observability
**Metrics to add**:
- Connection count and duration
- Message throughput rates
- SSE connection health
- Embedding API usage

**Tools**: Prometheus metrics, structured logging

### Phase 4: Testing and Validation (Week 4)
*Priority: P1 - High (Quality assurance)*

#### 4.1 Fix Test Environment
**Issue**: Linker errors preventing test execution

**Resolution**:
1. Install build dependencies in CI/CD
2. Configure proper linker in container
3. Add development container with all tools

**Expected outcome**: `cargo test --all` executes successfully

#### 4.2 Implement Integration Tests
**Test scenarios**:
- SSE connection lifecycle
- Message buffering and replay
- Session management
- Protocol version negotiation
- Keep-alive functionality
- Connection resilience

**File**: `crates/mcp/tests/integration_tests.rs` (rename from sse_integration_tests.rs)

#### 4.3 Load Testing
**Scenarios**:
- Multiple concurrent SSE connections
- High-frequency message sending
- Connection timeout and recovery
- Memory usage under load

**Tools**: Custom load testing scripts, memory profiling

## Technical Implementation Details

### SSE Implementation Architecture

```rust
// Connection flow
Client -> HTTP Upgrade -> SSE Stream -> Keep-Alive -> Session Management

// Component relationships
ConnectionManager 
    ↓ manages
Connections (1:N)
    ↓ each has
MessageBuffer + HeartbeatTimer + SessionContext
    ↓ coordinated by  
HeartbeatService (background task)
```

### Message Flow

```
1. Client connects to /sse endpoint
2. Server upgrades to SSE stream
3. Server assigns session ID and UUID
4. Server starts heartbeat timer (30s interval)
5. Client sends MCP messages via HTTP POST to /mcp
6. Server responses via SSE stream
7. Keep-alive heartbeats maintain connection
8. Timeout after 90s of inactivity
```

### Session Management

```rust
// Session lifecycle
Create -> Active -> Idle -> Timeout -> Cleanup

// Session storage
SessionId -> {
    transport: StreamableHttpTransport,
    created: DateTime,
    last_activity: DateTime,
    message_buffer: CircularBuffer<Message>,
    capabilities: ClientCapabilities,
}
```

## Rollback Procedures

### Phase 1 Rollback (SSE Module Issues)
1. Remove SSE module from `lib.rs`
2. Comment out SSE routes in server
3. Return server to health-check only mode
4. Deploy previous working version

### Phase 2 Rollback (rmcp Upgrade Issues)
1. Revert rmcp dependency to v0.1.5
2. Restore old protocol version declarations
3. Remove new session management code
4. Test with original MCP client

### Phase 3 Rollback (Performance Issues)
1. Disable batch processing features
2. Remove connection resilience code
3. Reduce connection limits
4. Monitor performance metrics

### Emergency Rollback
**Database**: No schema changes planned - no database rollback needed
**Docker**: Tagged images allow instant rollback to previous version
**Kubernetes**: Helm rollback commands ready

## Risk Assessment and Mitigation

### High Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| SSE implementation breaks existing functionality | Medium | High | Feature flags, gradual rollout |
| rmcp upgrade introduces breaking changes | High | High | Thorough testing, compatibility layer |
| Performance degradation | Medium | Medium | Load testing, monitoring |

### Medium Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Development timeline overrun | Medium | Medium | Parallel development tracks |
| Test environment issues persist | Low | Medium | Container-based testing |
| pgvector limitations cause problems | Low | Medium | Dimension reduction options |

## Resource Requirements

### Development Team
- **Lead Rust Developer**: 4 weeks full-time
  - SSE implementation
  - Transport layer design
  - Protocol upgrade
  
- **Backend Developer**: 2 weeks part-time
  - Testing implementation
  - Performance optimizations
  - Documentation
  
- **DevOps Engineer**: 1 week part-time
  - CI/CD pipeline fixes
  - Container optimization
  - Production deployment

### Infrastructure
- Development PostgreSQL instance with pgvector
- CI/CD pipeline with proper Rust build environment
- Container registry for testing images
- Load testing environment

### External Dependencies
- OpenAI API access for embeddings testing
- Kubernetes cluster access for deployment testing

## Success Metrics

### Technical Metrics
- [ ] 100% test suite passes
- [ ] Zero compilation warnings
- [ ] Sub-100ms SSE connection establishment
- [ ] 99.9% message delivery rate
- [ ] Memory usage < 100MB under normal load

### Protocol Compliance Metrics  
- [ ] MCP 2025-06-18 specification compliance
- [ ] Session management working correctly
- [ ] Keep-alive intervals maintain connections
- [ ] Proper error handling and recovery

### Performance Metrics
- [ ] Support 100+ concurrent SSE connections
- [ ] < 2 second response time for tool queries
- [ ] 70% cost reduction through batch embeddings
- [ ] Zero downtime deployments

## Timeline Summary

| Phase | Duration | Key Deliverables | Success Criteria |
|-------|----------|------------------|------------------|
| Phase 1 | Week 1 | SSE module, Transport layer | Project compiles |
| Phase 2 | Week 2 | Protocol upgrade, Session mgmt | MCP 2025-06-18 compliance |
| Phase 3 | Week 3 | Performance, Resilience | Production ready |
| Phase 4 | Week 4 | Testing, Validation | Full test coverage |

**Total Estimated Effort**: 3-4 weeks with 1 senior developer + supporting team

## Next Steps

1. **Immediate**: Begin Phase 1 implementation with SSE module creation
2. **Week 1**: Daily check-ins on compilation status and basic functionality
3. **Week 2**: Protocol compliance testing with real MCP clients
4. **Week 3**: Performance benchmarking and optimization
5. **Week 4**: Production deployment preparation and validation

The migration plan provides a clear path forward while maintaining system stability and allowing for rollback at each phase. The phased approach minimizes risk while ensuring the critical SSE functionality is implemented first to unblock compilation and basic testing.