# Autonomous Agent Prompt: Protocol Version Negotiation and Headers

## Mission (MVP)

Implement strict MCP protocol header handling for the single supported version `2025-06-18` and comprehensive header management. Drop negotiation/backward compatibility for MVP.

## Context

Building on the Streamable HTTP transport and session management from Tasks 2-3, support only protocol version `2025-06-18`. Reject other versions with HTTP 400.

## Primary Objectives

1. **Protocol Header Utilities**: Create constants and simple validators for the fixed version
2. **Header Management**: Implement comprehensive MCP and HTTP header handling
3. **Initialize Handler**: Ensure the initialize response includes the fixed version
4. **Session Integration**: Store protocol version in session state for consistency
5. **Response Formatting**: Ensure version-specific response formatting and headers

## Step-by-Step Implementation

### Step 1: Create Protocol Header Utilities
1. Create `crates/mcp/src/headers.rs` with constants/extractors
2. Implement fixed-version validation (only `2025-06-18`)
3. Provide helpers to inject headers into responses

### Step 2: Header Extraction and Validation
1. Create `crates/mcp/src/headers.rs` with Axum extractors
2. Implement MCP-Protocol-Version header extraction
3. Add Accept header validation for content types
4. Create header validation middleware

### Step 3: Initialize Handler
1. Ensure `handle_initialize` sets `protocolVersion: "2025-06-18"` in the result
2. Store fixed version in session state

### Step 4: Session State Integration
1. Update session management to store protocol version
2. Create version-aware session validation
3. Implement session consistency checks
4. Add version tracking across requests

### Step 5: Response Management
1. Implement proper Content-Type header management
2. Add MCP-specific headers to all responses
3. Ensure CORS compatibility

## Required Tools

1. **read_file**: Examine existing handlers and transport code
2. **write_file**: Create new protocol and header modules
3. **edit_file**: Update existing handlers and session management
4. **write_file**: Create comprehensive test suite

## Success Criteria

### Functional Requirements
- [ ] Fixed protocol version validation (2025-06-18)
- [ ] Header extraction and validation
- [ ] Session state with protocol version tracking (fixed)
- [ ] Response headers set consistently

### Integration Requirements
- [ ] Seamless integration with existing transport layer
- [ ] Compatibility with session management from Task 3
- [ ] Proper error handling for version mismatches
- [ ] CORS header compatibility

## Testing Strategy

1. **Header Validation Tests**: Verify proper header parsing and validation
2. **Session Integration Tests**: Test fixed version consistency across requests
3. **Error Handling Tests**: Validate responses for unsupported versions

## Expected Deliverables

1. **Protocol Version Module**: Complete version management implementation
2. **Header Management Module**: Comprehensive header handling
3. **Updated Initialize Handler**: Enhanced with version negotiation
4. **Session Integration**: Protocol version tracking in sessions
5. **Test Suite**: Complete coverage of version and header handling## Quality Gates and CI/CD Process

- Run static analysis after every new function is written:
  - Command: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
  - Fix all warnings before proceeding to write the next function.
- Before submission, ensure the workspace is clean:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
  - `cargo test --all-features`
- Feature branch workflow and CI gating:
  - Do all work on a feature branch (e.g., `feature/<task-id>-<short-name>`).
  - Push to the remote feature branch and monitor the GitHub Actions workflow (`.github/workflows/build-server.yml`) until it is green.
  - Require the deployment stage to complete successfully before creating a pull request.
  - Only create the PR after the workflow is green and deployment has succeeded; otherwise fix issues and re-run.
