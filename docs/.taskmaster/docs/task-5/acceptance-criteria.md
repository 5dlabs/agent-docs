# Acceptance Criteria: Protocol Version Negotiation and Headers

## Functional Requirements

### FR-1: Protocol Header Utilities (MVP)
- [ ] Constants for `MCP-Protocol-Version` (value must be `2025-06-18`)
- [ ] Validator returning HTTP 400 for unsupported versions
- [ ] Utilities for setting required headers on responses

### FR-2: Header Extraction and Validation
- [ ] Custom Axum extractor for MCP-Protocol-Version header
- [ ] AcceptHeader extractor for content type validation
- [ ] Header validation returning HTTP 400 for invalid headers
- [ ] Support for application/json and text/event-stream content types
- [ ] Header constants and manipulation utilities
- [ ] Integration with tracing for header processing logs

### FR-3: Initialize Handler (Fixed Version)
- [ ] InitializeResult includes `protocolVersion: "2025-06-18"`
- [ ] Session state created with fixed version
- [ ] Proper error responses for missing/invalid headers

- [ ] SessionState struct with `protocol_version: "2025-06-18"`
- [ ] Thread-safe session storage with version tracking
- [ ] Session validation ensuring version consistency
- [ ] Session expiry with configurable TTL
- [ ] Middleware for session extraction and validation
- [ ] Version-aware request processing

### FR-5: Response Header Management
- [ ] Automatic Content-Type header setting for responses
- [ ] MCP-Protocol-Version header in all responses
- [ ] Mcp-Session-Id header management
- [ ] CORS header compatibility
- [ ] Error response header consistency

## Non-Functional Requirements

### NFR-1: Performance
- [ ] Version parsing time < 1ms per request
- [ ] Header validation overhead < 5ms per request
- [ ] Session lookup time < 1ms
- [ ] Memory usage linear with session count
- [ ] No performance degradation with version negotiation

### NFR-2: Compatibility (MVP)
- [ ] Support for 2025-06-18 only; reject others with clear errors

### NFR-3: Reliability
- [ ] Robust header parsing with malformed input handling
- [ ] Session state consistency across requests
- [ ] Proper error recovery for version negotiation failures
- [ ] No crashes with invalid protocol version strings
- [ ] Consistent behavior across different client types

## Test Cases (MVP)

### TC-1: Protocol Version Parsing
**Given**: Various protocol version strings
**When**: Parsing with FromStr implementation  
**Then**: Correct ProtocolVersion enum variants returned
**And**: Invalid versions return appropriate errors

### TC-2: Fixed Version Echo
**Given**: Any initialize request
**Then**: Response includes `protocolVersion: "2025-06-18"`
**And**: Session state stores same fixed version

### TC-3: Unsupported Version Handling
**Given**: Client sends any version other than `2025-06-18`
**Then**: HTTP 400 returned with clear error

### TC-4: Header Validation
**Given**: Requests with various header combinations
**When**: Header extraction middleware processes request
**Then**: Valid headers extracted correctly
**And**: Invalid headers return HTTP 400
**And**: Missing headers handled appropriately

### TC-5: Session Version Consistency
**Given**: Established session with negotiated version
**When**: Subsequent requests processed
**Then**: Same protocol version used throughout
**And**: Version mismatch detected and handled
**And**: Session state remains consistent

## Deliverables

### D-1: Core Protocol Management (Simplified)
- [ ] `crates/mcp/src/headers.rs` - Header constants, extraction, validation
- [ ] Error handling for invalid/missing protocol version

### D-2: Handler Integration
- [ ] Updated initialize handler with version negotiation
- [ ] Session state management with version tracking
- [ ] Response formatting based on negotiated version
- [ ] Version-aware error handling

### D-3: Test Suite
- [ ] Unit tests for version parsing and comparison
- [ ] Header validation tests with edge cases
- [ ] Integration tests for version negotiation
- [ ] Session state consistency tests
- [ ] Backward compatibility tests

### D-4: Documentation
- [ ] Protocol version handling documentation
- [ ] Header management API reference
- [ ] Version negotiation flow diagrams
- [ ] Troubleshooting guide for version issues

## Validation Criteria

### V-1: Functional Validation
- [ ] Header extraction functions with valid inputs
- [ ] Fixed protocol version echoed consistently
- [ ] Session state maintains version consistency (fixed)
- [ ] Error handling covers invalid/missing headers

### V-2: Integration Validation
- [ ] Seamless integration with transport layer from Task 2
- [ ] Compatibility with session management from Task 3
- [ ] Proper CORS header handling
- [ ] No breaking changes to existing functionality

### V-3: Compatibility Validation
- [ ] Verify rejection of non-2025-06-18 versions

## Definition of Done

**This task is considered complete when:**

1. **All Functional Requirements** (FR-1 through FR-5) are implemented and verified
2. **All Non-Functional Requirements** (NFR-1 through NFR-3) meet specified criteria  
3. **All Test Cases** (TC-1 through TC-5) pass in automated test suite
4. **All Deliverables** (D-1 through D-4) are completed and reviewed
5. **All Validation Criteria** (V-1 through V-3) are satisfied
6. **Integration Testing** successful with multiple MCP clients
7. **Protocol Compliance** verified against MCP specification
8. **Performance Benchmarks** meet all specified requirements
9. **Documentation** complete and accessible
10. **Code Review** approved with focus on protocol compliance