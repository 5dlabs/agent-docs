# Task 4: Protocol Version Negotiation and Headers

## Overview

Implement MCP protocol version negotiation and header compliance for proper client-server communication and backward compatibility. This task ensures the Doc Server can handle multiple protocol versions and negotiate appropriate communication parameters with diverse MCP clients.

## Background

The MCP specification requires proper protocol version negotiation during the initialize phase. The server must support multiple protocol versions (2025-06-18, 2025-03-26, 2024-11-05) and negotiate the best compatible version with clients.

## Implementation Guide

### Phase 1: Protocol Version Registry
- Create `protocol_version.rs` module with ProtocolVersion enum
- Implement version parsing and comparison logic
- Define supported versions and compatibility matrix

### Phase 2: Header Management
- Implement header extraction middleware
- Add MCP-Protocol-Version and Accept header validation
- Create content type validation for responses

### Phase 3: Initialize Handler Enhancement
- Modify initialize request handler for version negotiation
- Store negotiated version in session state
- Return appropriate InitializeResult with protocol version

### Phase 4: Session State Integration
- Create session state management with protocol version tracking
- Implement session storage and retrieval
- Add version consistency enforcement

### Phase 5: Response Management
- Implement version-specific response formatting
- Add proper Content-Type and MCP headers
- Ensure CORS compatibility

## Technical Requirements

### Dependencies
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

### Core Types
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolVersion {
    V2025_06_18,
    V2025_03_26, 
    V2024_11_05,
}

pub struct SessionState {
    pub session_id: Uuid,
    pub negotiated_version: ProtocolVersion,
    pub created_at: DateTime<Utc>,
}
```

## Success Metrics
- Protocol version negotiation works with all supported versions
- Proper header handling in requests and responses
- Session state maintains version consistency
- Backward compatibility with legacy clients
- Error handling for unsupported versions

## Dependencies
- Task 2: Streamable HTTP Transport Foundation
- Axum framework for header extraction
- UUID for session management
- Chrono for session timestamps

## Risk Considerations
- Version compatibility issues with clients
- Session state management complexity
- Header parsing edge cases
- Performance impact of version checks

## Validation Criteria
- Unit tests for version negotiation logic
- Integration tests with various client versions
- Header validation tests
- Session state management tests
- Error handling verification