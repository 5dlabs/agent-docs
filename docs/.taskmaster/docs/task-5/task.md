# Task 5: Protocol Version Negotiation and Headers

## Overview (MVP scope)

Implement strict MCP protocol header handling for the single supported protocol version `2025-06-18`. Drop negotiation and legacy support for MVP. Ensure headers are extracted/validated and echoed consistently.

## Background

For MVP, we support only the latest Streamable HTTP protocol (2025-06-18). Clients must send `MCP-Protocol-Version: 2025-06-18`; other versions are rejected with HTTP 400. Transport policy is JSON-only: SSE is disabled (no legacy HTTP+SSE); `GET /mcp` returns 405; server always responds with `Content-Type: application/json`.

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

### Phase 5: Response Management (JSON-only)
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

### Non-Negotiable Quality Gates (Blocking)
- Formatting must pass: `cargo fmt --all -- --check` must succeed locally and in CI. If formatting fails, fix formatting and re-run; no PRs may be opened/merged until this passes.
- Clippy must pass: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` must succeed locally and in CI. Do not add new `#[allow(...)]` without explicit review/approval.
- CI must gate deployment on both gates; any failure blocks deployment and PR creation.

## Detailed Requirements to Meet Acceptance Criteria

1) Protocol Version Enforcement
- Reject all requests lacking `MCP-Protocol-Version` or with any value other than `2025-06-18` with HTTP 400 and a structured JSON error body.
- Echo `MCP-Protocol-Version: 2025-06-18` on every response (success and error).

2) Header Validation (Requests)
- Content negotiation (JSON-only):
  - `Content-Type` for POST MUST be `application/json`; return 415 for any other value.
  - If `Accept` is present, it MUST include `application/json`; ignore `text/event-stream` even if present. Return 406 if `Accept` excludes `application/json`.
  - Provide dedicated extractor/types for protocol version, `Content-Type`, and `Accept` with clear error variants and IntoResponse implementations.

3) Initialize Response and Session
- `initialize` response must include `protocolVersion: "2025-06-18"`.
- Create a session on first valid request and return `Mcp-Session-Id` (UUID v4) in the response headers.
- Require `Mcp-Session-Id` on subsequent requests and maintain version consistency in the stored session record.
- Session TTL: default 30 minutes with background cleanup; refreshing on activity.

4) Response Header Management (All Responses)
- Always include `MCP-Protocol-Version`.
- Include `Mcp-Session-Id` when a session was created/found.
- Set `Content-Type: application/json` on all responses.
- Preserve CORS compatibility via `CorsLayer`.

5) Security Hooks
- Before business logic, run Origin and DNS-rebinding validations for DELETE/POST paths as implemented in security module.

6) Tests (Unit + Integration)
- Unit tests for header extractors (valid/invalid/missing) and error response formatting.
- Integration tests verifying:
  - 400 on missing or wrong `MCP-Protocol-Version`.
  - 415 on wrong `Content-Type` and 400 on missing `Content-Type` for POST.
  - 406 when `Accept` is present but lacks `application/json`.
  - 200 for valid POST with required headers; response contains both `MCP-Protocol-Version` and `Mcp-Session-Id`.
  - 405 for GET `/mcp` (MVP: SSE not yet implemented).

7) SDK/Crate Usage Policy (Spec Compliance)
- Do not rely on external MCP SDK crates for protocol semantics unless covered by tests verifying the 2025-06-18 spec. Current implementation should remain self-contained (extractors, transport, handler) to avoid drift.
- If external crates are used transitively, guard with integration tests that assert the wire behavior matches the spec (headers, error codes, fields).
