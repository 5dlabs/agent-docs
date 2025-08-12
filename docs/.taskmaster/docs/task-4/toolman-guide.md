# Toolman Guide: Session Management and Security Implementation

## Tool Selection Rationale

For implementing Session Management and Security features, the selected tools focus on filesystem operations for creating security modules, updating transport integration, and establishing comprehensive testing. This task requires creating new security and session modules while maintaining integration with the existing transport layer from Task 2.

### Primary Tools

#### Filesystem Tools
- **read_file**: Essential for understanding existing transport and server architecture
- **write_file**: Required for creating session management and security modules
- **edit_file**: Needed for integrating session management with existing transport layer
- **list_directory**: Helpful for exploring project structure and test organization
- **create_directory**: May be needed for test directory structure
- **search_files**: Useful for finding security patterns and session-related code

## When to Use Each Tool

### Phase 1: Analysis and Architecture Understanding

**Use read_file for:**
- Understanding transport implementation (`crates/mcp/src/transport.rs`) from Task 2
- Examining server structure (`crates/mcp/src/server.rs`) for integration points
- Reviewing current error handling patterns (`crates/mcp/src/lib.rs`)
- Checking existing dependencies in `Cargo.toml`
- Understanding current header handling and middleware patterns

**Use list_directory for:**
- Exploring `crates/mcp/src/` directory structure
- Understanding test directory organization (`crates/mcp/tests/`)
- Identifying existing middleware or security patterns

**Use search_files for:**
- Finding existing UUID usage or generation patterns
- Locating current header handling code
- Identifying existing security or validation code
- Finding middleware integration examples
- Searching for existing session-related code

### Phase 2: Core Implementation

**Use write_file for:**
- Creating session management module (`crates/mcp/src/session.rs`)
- Creating security validation module (`crates/mcp/src/security.rs`)
- Writing comprehensive unit tests (`crates/mcp/tests/session_tests.rs`)
- Writing security tests (`crates/mcp/tests/security_tests.rs`)
- Creating integration tests (`crates/mcp/tests/session_integration_tests.rs`)

**Use create_directory for:**
- Ensuring test directory structure exists
- Creating subdirectories for organized testing

### Phase 3: Integration and Updates

**Use edit_file for:**
- Updating `crates/mcp/src/lib.rs` to include new modules
- Modifying `crates/mcp/src/transport.rs` to integrate session handling
- Updating `crates/mcp/src/server.rs` for session manager initialization
- Modifying `Cargo.toml` to add new dependencies (chrono, uuid)
- Updating existing error types to include session errors

**Use read_file for:**
- Verifying integration changes are correct
- Checking that existing functionality is preserved
- Validating test implementations

**Use search_files for:**
- Ensuring all session integration points are covered
- Verifying consistent error handling patterns
- Checking for any missed header handling locations

## Best Practices

### Security-First Implementation
1. **Start with security module**: Implement Origin validation and DNS rebinding protection first
2. **Validate early**: Test security measures before adding functional features
3. **Defense in depth**: Layer multiple security measures for robust protection
4. **Secure defaults**: Configure for maximum security by default

### Session Management Strategy
1. **Thread safety first**: Implement proper locking before functional operations
2. **Test concurrency**: Verify thread safety with concurrent operations
3. **Memory management**: Implement cleanup mechanisms early to prevent leaks
4. **Error resilience**: Handle all error conditions gracefully

### Integration Approach
1. **Minimal transport changes**: Preserve existing transport functionality
2. **Header compliance**: Follow MCP specification exactly for header handling
3. **Backward compatibility**: Ensure existing MCP tools continue working
4. **Incremental testing**: Test each integration point independently

## Tool Usage Patterns

### Security Implementation Pattern
```
1. read_file -> examine existing middleware patterns
2. write_file -> create security.rs module
3. edit_file -> integrate security middleware
4. write_file -> create security tests
5. read_file -> verify integration
```

### Session Management Pattern
```
1. read_file -> understand transport header handling
2. write_file -> create session.rs module  
3. write_file -> create comprehensive session tests
4. edit_file -> integrate with transport layer
5. search_files -> verify all integration points
```

### Server Integration Pattern
```
1. read_file -> examine current server initialization
2. edit_file -> add session manager to server state
3. edit_file -> update lib.rs with new modules
4. edit_file -> update Cargo.toml dependencies
5. read_file -> verify all changes are correct
```

## Common Implementation Pitfalls

### Security Issues to Avoid
- **Weak session IDs**: Always use cryptographically secure UUID v4 generation
- **Origin bypasses**: Don't trust client-provided origin information for security decisions
- **Session fixation**: Never accept client-provided session IDs for new sessions
- **Information disclosure**: Don't leak session details in error messages

### Performance Pitfalls
- **Lock contention**: Design session storage for minimal lock holding time
- **Memory leaks**: Implement proper session cleanup from the start
- **Blocking operations**: Keep background cleanup tasks non-blocking
- **Unbounded growth**: Always implement session limits and cleanup

### Integration Problems
- **Header case sensitivity**: Handle HTTP header names case-insensitively
- **Middleware ordering**: Place security validation before session handling
- **Error propagation**: Maintain consistent error handling patterns
- **State management**: Ensure thread-safe access to session state

## Specific Tool Usage Guidelines

### Creating Security Module
1. **read_file** existing middleware patterns first
2. **write_file** security.rs with Origin validation
3. Focus on DNS rebinding protection implementation
4. Test with various Origin header combinations

### Implementing Session Management
1. **write_file** session.rs with proper data structures
2. Implement thread-safe operations using Arc<RwLock<>>
3. Add comprehensive error handling
4. Include background cleanup task implementation

### Transport Integration
1. **read_file** transport.rs to understand current structure
2. **edit_file** to add session header extraction
3. Preserve all existing functionality
4. Add session context to request handling

### Testing Strategy
1. **write_file** unit tests for each module independently
2. Create security tests with attack simulation
3. Write integration tests for full request/response cycle
4. Include concurrent access tests for session management

## Troubleshooting Guide

### Compilation Issues
- Check `Cargo.toml` for correct dependency versions
- Verify all modules are properly declared in `lib.rs`
- Ensure proper imports for UUID and chrono types
- Validate async/await syntax in session operations

### Integration Problems
- Use `search_files` to find all header handling locations
- Verify middleware ordering in server setup
- Check session state is properly passed through request lifecycle
- Ensure error types are consistent with existing patterns

### Security Validation Issues
- Test Origin validation with localhost variants
- Verify DNS rebinding protection with various Host headers
- Check that security logging works for all validation failures
- Validate CORS integration doesn't conflict with Origin validation

### Session Management Issues
- Test concurrent session operations with multiple threads
- Verify session cleanup removes expired sessions only
- Check session limits are properly enforced
- Ensure background cleanup task starts correctly

## Success Indicators

### Security Implementation
- All Origin validation tests pass
- DNS rebinding attacks are properly blocked
- Security events are logged with appropriate detail
- Server binds only to localhost for security

### Session Management
- Sessions created with secure UUID v4 IDs
- Thread-safe operations work under concurrency
- Background cleanup removes expired sessions
- Memory usage remains stable over time

### Integration Success
- All existing MCP tools continue to function
- Headers handled correctly in both directions
- Transport layer enhanced without breaking changes
- Performance impact within acceptable limits

### Test Coverage
- Unit tests cover all session and security operations
- Integration tests verify end-to-end functionality
- Security tests simulate various attack scenarios
- Performance tests validate concurrency and cleanup

By following this comprehensive tool usage guide, you'll implement secure session management that integrates seamlessly with the existing transport infrastructure while providing robust protection against common web security vulnerabilities.