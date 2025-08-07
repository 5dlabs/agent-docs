# Task 2: SSE Keep-Alive Implementation - Tool Usage Guide

## Overview
This guide explains the optimal tool usage strategy for implementing Server-Sent Events (SSE) heartbeat mechanism to maintain stable connections with Toolman clients. The task focuses on infrastructure enhancement requiring code development, testing, and integration.

## Task Status: PENDING

## Tool Selection Strategy

### Why Local Filesystem Tools Are Primary
This task involves extensive code development and integration work:
- **Rust server-side implementation**: SSE endpoints, connection management
- **JavaScript client-side code**: Reconnection logic, message handling  
- **Configuration updates**: Environment setup, Docker configuration
- **Test suite development**: Unit tests, integration tests, load tests
- **Documentation creation**: API docs, troubleshooting guides

### Remote Tools Not Required
- **No external research needed**: SSE is a standard web technology
- **No Kubernetes operations**: This is MCP server enhancement
- **No documentation searches**: Implementation follows web standards
- **No infrastructure provisioning**: Uses existing Docker setup

## Phase-by-Phase Tool Usage

### Phase 1: Environment Analysis and Code Understanding
**Objective**: Understand current MCP server structure and SSE requirements

#### Primary Tools
- **`directory_tree`**: Map project structure and identify key files
- **`read_file`**: Examine current MCP server implementation
- **`search_files`**: Find existing HTTP/websocket handling code

#### Specific Actions
```bash
# Use directory_tree to understand structure
directory_tree("/workspace/agent-docs/crates/mcp")

# Use read_file for key server files:
- crates/mcp/src/main.rs (server startup)
- crates/mcp/src/handlers.rs (request handling)
- crates/mcp/Cargo.toml (dependencies)
- docker-compose.dev.yml (current setup)

# Use search_files to find relevant patterns:
- "axum" (current HTTP framework usage)
- "async fn" (async handler patterns)
- "router" (routing setup)
- "cors" (CORS configuration)
```

### Phase 2: Server-Side SSE Implementation
**Objective**: Implement SSE endpoint with heartbeat functionality

#### Primary Tools
- **`edit_file`**: Modify existing server files to add SSE support
- **`write_file`**: Create new modules for SSE functionality
- **`read_file`**: Reference existing code patterns for consistency

#### Implementation Workflow
```rust
// Step 1: Add SSE dependencies using edit_file
// File: crates/mcp/Cargo.toml
edit_file("crates/mcp/Cargo.toml", 
  old_string="axum = \"0.7\"",
  new_string="axum = \"0.7\"\ntokio-stream = \"0.1\"\nfutures = \"0.3\"")

// Step 2: Create SSE handler module using write_file
// File: crates/mcp/src/sse.rs
write_file("crates/mcp/src/sse.rs", sse_handler_code)

// Step 3: Integrate SSE route using edit_file  
// File: crates/mcp/src/main.rs
edit_file("crates/mcp/src/main.rs",
  old_string="let app = Router::new()",
  new_string="let app = Router::new().route(\"/sse\", get(sse::sse_handler))")
```

#### Key Components to Implement
```rust
// Create these files using write_file:
1. crates/mcp/src/sse/mod.rs - SSE module definition
2. crates/mcp/src/sse/handler.rs - SSE endpoint handler
3. crates/mcp/src/sse/connection.rs - Connection management
4. crates/mcp/src/sse/heartbeat.rs - Heartbeat mechanism
5. crates/mcp/src/sse/buffer.rs - Message buffering
```

### Phase 3: Connection Management System
**Objective**: Implement connection tracking and cleanup

#### Primary Tools
- **`write_file`**: Create connection management components
- **`edit_file`**: Add connection tracking to existing handlers
- **`read_file`**: Study existing connection patterns

#### Connection Management Implementation
```rust
// Create connection tracking using write_file
// File: crates/mcp/src/sse/connection_manager.rs

use std::collections::HashMap;
use uuid::Uuid;

pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<Uuid, Connection>>>,
    heartbeat_interval: Duration,
}

// Implementation includes:
- Connection lifecycle management
- Heartbeat scheduling with tokio::time
- Connection cleanup on disconnect
- Metrics collection for monitoring
```

### Phase 4: Client-Side Reconnection Logic
**Objective**: Create JavaScript wrapper for reliable SSE connections

#### Primary Tools
- **`write_file`**: Create JavaScript/TypeScript client code
- **`edit_file`**: Add client integration to existing frontend (if any)
- **`read_file`**: Reference existing client code patterns

#### Client Implementation
```javascript
// Create client wrapper using write_file
// File: client/sse-connection.js

class SSEConnection {
  constructor(url, options = {}) {
    this.url = url;
    this.options = {
      heartbeatInterval: 30000,
      initialRetry: 1000,
      maxRetry: 60000,
      jitterMax: 500,
      ...options
    };
    this.connect();
  }
  
  // Implementation includes:
  // - Exponential backoff reconnection
  // - Connection state management  
  // - Message buffering during disconnection
  // - Event handling and callbacks
}
```

### Phase 5: Message Buffering System
**Objective**: Implement message queuing during disconnections

#### Primary Tools
- **`write_file`**: Create buffering system components
- **`edit_file`**: Integrate with existing message handling
- **`read_file`**: Understand current message flow

#### Buffering Implementation Strategy
```rust
// Create message buffer using write_file
// File: crates/mcp/src/sse/message_buffer.rs

pub struct MessageBuffer {
    buffer: VecDeque<Message>,
    max_size: usize,
    retention_time: Duration,
}

// Key features to implement:
- FIFO message queuing
- Buffer size limits with overflow handling
- Message expiration based on timestamp
- Replay functionality on reconnection
```

### Phase 6: Testing and Validation
**Objective**: Create comprehensive test suites

#### Primary Tools
- **`write_file`**: Create test files for all components
- **`read_file`**: Study existing test patterns
- **`edit_file`**: Add test configurations to Cargo.toml

#### Test Suite Creation
```bash
# Create test files using write_file:
1. crates/mcp/tests/sse_tests.rs - Integration tests
2. crates/mcp/src/sse/tests.rs - Unit tests  
3. scripts/load_test_sse.js - Load testing script
4. client/test_reconnection.html - Manual testing page

# Test categories to implement:
- Unit tests for heartbeat timing
- Integration tests for client-server communication
- Load tests for concurrent connections
- Network failure simulation tests
```

### Phase 7: Configuration and Integration
**Objective**: Update system configuration and integrate with existing infrastructure

#### Primary Tools
- **`edit_file`**: Update configuration files
- **`search_files`**: Find configuration references
- **`read_file`**: Verify current configuration patterns

#### Configuration Updates
```yaml
# Edit docker-compose.dev.yml for SSE support
edit_file("docker-compose.dev.yml",
  old_string="ports:\n      - \"3001:3001\"",
  new_string="ports:\n      - \"3001:3001\"\n    environment:\n      - SSE_HEARTBEAT_INTERVAL=30\n      - SSE_CONNECTION_TIMEOUT=90")

# Edit .env.example for SSE configuration
edit_file(".env.example",
  old_string="PORT=3001",
  new_string="PORT=3001\nSSE_HEARTBEAT_INTERVAL=30\nSSE_CONNECTION_TIMEOUT=90\nSSE_BUFFER_SIZE=1000")
```

## Tool Usage Patterns

### File Reading Strategy
```bash
# Pattern 1: Single file examination
read_file("crates/mcp/src/main.rs")  # Understand server structure

# Pattern 2: Multi-file analysis for consistency
read_multiple_files([
  "crates/mcp/src/main.rs",
  "crates/mcp/src/handlers.rs", 
  "crates/mcp/src/lib.rs"
])
```

### Code Development Pattern
```bash
# Pattern 1: Create new functionality
write_file("new_module.rs", complete_implementation)

# Pattern 2: Extend existing functionality  
read_file("existing_file.rs")      # Study current implementation
edit_file("existing_file.rs",      # Add SSE integration
  old_string="existing_pattern",
  new_string="enhanced_pattern_with_sse")
```

### Testing and Validation Pattern
```bash
# Pattern 1: Create comprehensive tests
write_file("tests/sse_integration.rs", test_suite)

# Pattern 2: Validate implementations
read_file("test_results.log")      # Analyze test outcomes
edit_file("src/sse/handler.rs",    # Fix issues found in testing
  old_string="buggy_code",
  new_string="corrected_code")
```

## Critical Success Factors

### Code Quality Tools Usage
1. **Consistent Patterns**: Use `read_file` to understand existing patterns before implementing new code
2. **Incremental Development**: Use `edit_file` for careful, targeted modifications  
3. **Comprehensive Testing**: Use `write_file` to create thorough test suites

### Integration Safety
1. **Backup Before Changes**: Use `read_file` to capture current state
2. **Gradual Integration**: Use `edit_file` for step-by-step integration
3. **Validation at Each Step**: Use `read_file` to verify changes

### Performance Optimization
- Use `write_file` to create performance benchmark scripts
- Use `read_file` to analyze profiling results
- Use `edit_file` to implement optimizations based on findings

## Common Implementation Pitfalls

### File Management Issues
- **Don't**: Create files without understanding project structure
- **Do**: Use `directory_tree` and `read_file` to understand organization first

### Code Integration Problems  
- **Don't**: Modify existing code without understanding current patterns
- **Do**: Use `read_file` extensively to understand existing implementation

### Testing Inadequacy
- **Don't**: Skip comprehensive testing due to complexity
- **Do**: Use `write_file` to create thorough test suites for each component

## Tool Usage Checklist

### Pre-Implementation Analysis
- [ ] `directory_tree` - Map project structure
- [ ] `read_file` - Examine server architecture 
- [ ] `search_files` - Find relevant code patterns
- [ ] `read_multiple_files` - Study related components

### Server-Side Implementation
- [ ] `edit_file` - Add SSE dependencies to Cargo.toml
- [ ] `write_file` - Create SSE handler modules
- [ ] `write_file` - Implement connection management
- [ ] `edit_file` - Integrate SSE routes with main server

### Client-Side Implementation
- [ ] `write_file` - Create JavaScript reconnection wrapper
- [ ] `write_file` - Implement exponential backoff logic
- [ ] `write_file` - Add message buffering functionality
- [ ] `write_file` - Create client integration examples

### Testing and Validation
- [ ] `write_file` - Create unit test suites
- [ ] `write_file` - Develop integration test scenarios
- [ ] `write_file` - Build load testing scripts
- [ ] `read_file` - Analyze test results and logs

### Configuration and Deployment
- [ ] `edit_file` - Update Docker configuration
- [ ] `edit_file` - Add environment variables
- [ ] `search_files` - Find configuration references
- [ ] `read_file` - Verify configuration completeness

### Documentation and Handoff
- [ ] `write_file` - Create API documentation
- [ ] `write_file` - Develop troubleshooting guides
- [ ] `write_file` - Document configuration options
- [ ] `read_file` - Verify documentation accuracy

This systematic approach ensures successful SSE implementation while maintaining code quality, system stability, and comprehensive testing coverage throughout the development process.