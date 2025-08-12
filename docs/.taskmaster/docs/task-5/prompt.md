# Autonomous Agent Prompt: Protocol Version Negotiation and Headers

## Mission

Implement MCP protocol version negotiation and comprehensive header management to ensure proper client-server communication and backward compatibility across multiple MCP protocol versions.

## Context

Building on the Streamable HTTP transport and session management from Tasks 2-3, you must now implement proper MCP protocol version handling to support clients using different protocol versions (2025-06-18, 2025-03-26, 2024-11-05).

## Primary Objectives

1. **Protocol Version Registry**: Create centralized version management with parsing and validation
2. **Header Management**: Implement comprehensive MCP and HTTP header handling
3. **Version Negotiation**: Add initialize handler with proper version negotiation
4. **Session Integration**: Store protocol version in session state for consistency
5. **Response Formatting**: Ensure version-specific response formatting and headers

## Step-by-Step Implementation

### Step 1: Create Protocol Version Foundation
1. Create `crates/mcp/src/protocol_version.rs` with ProtocolVersion enum
2. Implement FromStr trait for version parsing
3. Add version comparison and compatibility methods
4. Create ProtocolRegistry for version management

### Step 2: Header Extraction and Validation
1. Create `crates/mcp/src/headers.rs` with Axum extractors
2. Implement MCP-Protocol-Version header extraction
3. Add Accept header validation for content types
4. Create header validation middleware

### Step 3: Initialize Handler Enhancement
1. Modify `handle_initialize` in handlers.rs for version negotiation
2. Extract client protocol version from initialize params
3. Implement negotiation logic and version selection
4. Update InitializeResult with negotiated version

### Step 4: Session State Integration
1. Update session management to store protocol version
2. Create version-aware session validation
3. Implement session consistency checks
4. Add version tracking across requests

### Step 5: Response Management
1. Create version-specific response formatting
2. Implement proper Content-Type header management
3. Add MCP-specific headers to all responses
4. Ensure CORS compatibility

## Required Tools

1. **read_file**: Examine existing handlers and transport code
2. **write_file**: Create new protocol and header modules
3. **edit_file**: Update existing handlers and session management
4. **write_file**: Create comprehensive test suite

## Success Criteria

### Functional Requirements
- [ ] Protocol version parsing and validation
- [ ] Version negotiation during initialize
- [ ] Header extraction and validation
- [ ] Session state with protocol version tracking
- [ ] Version-specific response formatting

### Integration Requirements
- [ ] Seamless integration with existing transport layer
- [ ] Compatibility with session management from Task 3
- [ ] Proper error handling for version mismatches
- [ ] CORS header compatibility

## Testing Strategy

1. **Version Negotiation Tests**: Test with all supported protocol versions
2. **Header Validation Tests**: Verify proper header parsing and validation
3. **Session Integration Tests**: Test version consistency across requests
4. **Error Handling Tests**: Validate responses for unsupported versions
5. **Backward Compatibility Tests**: Ensure legacy client support

## Expected Deliverables

1. **Protocol Version Module**: Complete version management implementation
2. **Header Management Module**: Comprehensive header handling
3. **Updated Initialize Handler**: Enhanced with version negotiation
4. **Session Integration**: Protocol version tracking in sessions
5. **Test Suite**: Complete coverage of version and header handling