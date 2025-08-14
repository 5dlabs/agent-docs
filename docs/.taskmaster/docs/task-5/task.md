# Task 5: Protocol Version Negotiation and Headers

## Overview (MVP scope)

Implement strict MCP protocol header handling for the single supported protocol version `2025-06-18`. Drop negotiation and legacy support for MVP. Ensure headers are extracted/validated and echoed consistently.

## Background

For MVP, we support only the latest Streamable HTTP protocol (2025-06-18). Clients must send `MCP-Protocol-Version: 2025-06-18`; other versions are rejected with HTTP 400.

## Implementation Guide

### Phase 1: Protocol Header Utilities (Simplified)
- Create `headers.rs` with constants and simple validators for `MCP-Protocol-Version` and `Mcp-Session-Id`
- Validate only `2025-06-18`; return 400 otherwise

### Phase 2: Header Management
- Implement header extraction middleware
- Add MCP-Protocol-Version and Accept header validation
- Create content type validation for responses

### Phase 3: Initialize Handler
- Ensure initialize response includes `protocolVersion: "2025-06-18"`
- Store fixed version in session state for consistency

### Phase 4: Session State Integration
- Create session state management with protocol version tracking
- Implement session storage and retrieval
- Add version consistency enforcement

### Phase 5: Response Management
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

### Core Types (Simplified)
```rust
pub struct SessionState {
    pub session_id: Uuid,
    pub protocol_version: &'static str, // always "2025-06-18" in MVP
    pub created_at: DateTime<Utc>,
}
```

## Success Metrics
- Proper header handling in requests and responses
- Session state maintains version consistency (fixed 2025-06-18)
- Error handling for unsupported versions (400)

## Notes from Assessment
- Echo version `2025-06-18` in all responses; include `Mcp-Session-Id`

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
- Error handling verification## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.
