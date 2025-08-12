# Toolman Guide: Protocol Version Negotiation and Headers Implementation

## Tool Selection Rationale

For implementing Protocol Version Negotiation and Headers, the selected tools focus on filesystem operations for creating protocol management modules, updating existing handlers, and establishing comprehensive header validation. This task requires careful integration with existing transport and session management while maintaining backward compatibility.

### Primary Tools

#### Filesystem Tools
- **read_file**: Essential for understanding existing handler and transport implementations
- **write_file**: Required for creating protocol version and header management modules
- **edit_file**: Needed for updating initialize handlers and session management integration
- **list_directory**: Helpful for exploring handler patterns and test structures
- **create_directory**: May be needed for organized test structure
- **search_files**: Useful for finding existing version handling and header patterns

## When to Use Each Tool

### Phase 1: Understanding Current Architecture

**Use read_file for:**
- Examining current initialize handler (`crates/mcp/src/handlers.rs`)
- Understanding transport layer header handling (`crates/mcp/src/transport.rs`)
- Reviewing session management implementation (`crates/mcp/src/session.rs`)
- Checking existing error handling patterns
- Understanding current MCP request/response structures

**Use search_files for:**
- Finding existing header handling code
- Locating version-related constants or enums
- Identifying initialization request processing
- Finding error handling patterns for HTTP status codes

### Phase 2: Core Implementation

**Use write_file for:**
- Creating protocol version module (`crates/mcp/src/protocol_version.rs`)
- Creating header management module (`crates/mcp/src/headers.rs`)
- Writing comprehensive unit tests (`crates/mcp/tests/protocol_tests.rs`)
- Writing header validation tests (`crates/mcp/tests/header_tests.rs`)

**Use create_directory for:**
- Ensuring proper test directory structure
- Creating organized subdirectories for different test categories

### Phase 3: Integration and Updates

**Use edit_file for:**
- Updating `crates/mcp/src/handlers.rs` to add version negotiation
- Modifying `crates/mcp/src/lib.rs` to include new modules
- Updating session management to store protocol versions
- Modifying transport layer for enhanced header handling
- Adding new dependencies to `Cargo.toml` if needed

**Use read_file for:**
- Verifying integration changes maintain existing functionality
- Checking updated handler implementations
- Validating test implementations

## Best Practices

### Protocol Version Management
1. **Start with enum definition**: Create clear ProtocolVersion enum first
2. **Implement parsing carefully**: Handle malformed version strings gracefully
3. **Version compatibility matrix**: Define clear compatibility rules
4. **Future-proof design**: Consider extensibility for new versions

### Header Management Strategy
1. **Use Axum extractors**: Leverage framework capabilities for header extraction
2. **Validate early**: Perform header validation at middleware level
3. **Consistent error responses**: Use standard HTTP status codes
4. **Content negotiation**: Properly handle Accept headers

### Initialize Handler Integration
1. **Preserve existing functionality**: Don't break current initialize handling
2. **Clear negotiation logic**: Implement transparent version selection
3. **Proper error responses**: Return helpful error messages for version issues
4. **Session integration**: Store negotiated version in session state

## Tool Usage Patterns

### Protocol Version Implementation Pattern
```
1. read_file -> examine existing handler patterns
2. write_file -> create protocol_version.rs module
3. write_file -> create comprehensive version tests
4. edit_file -> integrate with handlers
```

### Header Management Pattern
```
1. read_file -> understand current header handling
2. write_file -> create headers.rs module
3. edit_file -> integrate header extractors with transport
4. write_file -> create header validation tests
```

### Initialize Handler Enhancement Pattern
```
1. read_file -> examine current initialize implementation
2. edit_file -> add version negotiation logic
3. edit_file -> update response structure
4. write_file -> create integration tests
```

## Common Implementation Pitfalls

### Version Handling Issues
- **Case sensitivity**: Handle protocol version strings consistently
- **Partial matches**: Don't accept partial or malformed version strings
- **Default fallback**: Always have a clear default version strategy
- **Future versions**: Handle unknown versions gracefully

### Header Processing Problems  
- **Case insensitive**: HTTP headers should be handled case-insensitively
- **Missing headers**: Graceful handling of optional vs required headers
- **Content negotiation**: Proper Accept header processing
- **CORS compatibility**: Ensure new headers don't break CORS

### Integration Challenges
- **Session state**: Properly store and retrieve protocol version from sessions
- **Error propagation**: Consistent error handling across version operations
- **Backward compatibility**: Don't break existing clients during upgrades
- **Performance impact**: Minimize overhead of version checking

## Specific Implementation Guidelines

### Creating Protocol Version Module
1. **Define enum first**: Start with clear ProtocolVersion variants
2. **Implement FromStr**: Handle string parsing with proper error handling
3. **Add comparison methods**: Implement version ordering and compatibility
4. **Create registry**: Centralized management of supported versions

### Header Management Implementation
1. **Custom extractors**: Use Axum's extractor system for type safety
2. **Validation middleware**: Early validation at middleware level
3. **Error responses**: Standard HTTP error codes for header issues
4. **Content type handling**: Proper negotiation for JSON and SSE

### Initialize Handler Updates
1. **Extract client version**: Parse version from initialize request
2. **Negotiate version**: Implement server-side version selection logic
3. **Update response**: Include negotiated version in InitializeResult
4. **Session storage**: Store version in session for consistency

## Troubleshooting Guide

### Version Parsing Issues
- Check string format matches expected pattern
- Verify FromStr implementation handles edge cases
- Test with various malformed input strings
- Ensure proper error types for different failure modes

### Header Validation Problems
- Use case-insensitive header name matching
- Test with various Accept header combinations  
- Verify Content-Type setting in responses
- Check CORS header compatibility

### Integration Issues
- Ensure new modules are declared in lib.rs
- Verify session state properly stores version information
- Test that existing functionality remains unaffected
- Check error handling maintains consistent patterns

### Performance Concerns
- Profile version parsing performance with benchmarks
- Monitor header validation overhead
- Test with high request volumes
- Verify no memory leaks in version handling

## Success Indicators

### Protocol Version Management
- All supported versions parse correctly
- Version comparison logic works as expected
- Registry properly manages supported versions
- Error handling covers all edge cases

### Header Processing
- All required headers extracted properly
- Content negotiation works with various clients
- Error responses appropriate for header failures
- Integration with existing middleware successful

### Initialize Handler
- Version negotiation follows MCP specification
- Session state properly tracks negotiated version
- Response format includes correct version information
- Backward compatibility maintained for legacy clients

### Overall Integration
- No breaking changes to existing functionality
- Performance impact within acceptable limits
- Comprehensive test coverage for all scenarios
- Documentation clear and complete

By following this guide, you'll implement robust protocol version negotiation that ensures compatibility across different MCP client versions while maintaining the integrity of the existing system architecture.