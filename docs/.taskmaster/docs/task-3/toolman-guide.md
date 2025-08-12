# Toolman Guide: Streamable HTTP Transport Foundation Implementation

## Tool Selection Rationale

For implementing the Streamable HTTP Transport Foundation, the selected tools focus on filesystem operations for code implementation and file management. This task involves creating new transport modules, updating existing server code, and establishing proper integration patterns.

### Primary Tools

#### Filesystem Tools
- **read_file**: Essential for understanding existing MCP server architecture
- **write_file**: Required for creating new transport module and test files
- **edit_file**: Needed for updating existing server integration points
- **list_directory**: Helpful for exploring project structure and dependencies
- **create_directory**: May be needed for test directory structure
- **search_files**: Useful for finding related code patterns and dependencies

## When to Use Each Tool

### Phase 1: Analysis and Planning

**Use read_file for:**
- Understanding current server architecture (`crates/mcp/src/server.rs`)
- Examining existing handler implementations (`crates/mcp/src/handlers.rs`)
- Reviewing module structure (`crates/mcp/src/lib.rs`)
- Checking current Cargo.toml dependencies
- Understanding existing error handling patterns

**Use list_directory for:**
- Exploring the `crates/mcp/src/` directory structure
- Identifying existing test directories and patterns
- Understanding overall project layout

**Use search_files for:**
- Finding references to current transport implementation
- Locating existing SSE or HTTP handling code
- Identifying error types and handling patterns
- Finding configuration management examples

### Phase 2: Implementation

**Use write_file for:**
- Creating the main transport module (`crates/mcp/src/transport.rs`)
- Writing comprehensive unit tests (`crates/mcp/tests/transport_tests.rs`)
- Creating integration test files
- Adding documentation files

**Use create_directory for:**
- Ensuring test directory structure exists (`crates/mcp/tests/`)
- Creating any needed subdirectories for organization

**Use edit_file for:**
- Updating `crates/mcp/src/server.rs` to integrate new transport
- Modifying `crates/mcp/src/lib.rs` to include new transport module
- Updating `Cargo.toml` if new dependencies are required
- Modifying existing error handling to include transport errors

### Phase 3: Integration and Testing

**Use read_file for:**
- Verifying integration changes are correct
- Checking test file implementation
- Reviewing error handling integration

**Use edit_file for:**
- Fine-tuning integration points
- Adjusting error handling and logging
- Updating configuration management
- Refining test implementations

**Use search_files for:**
- Ensuring all references to old transport are updated
- Verifying consistent error handling patterns
- Checking for any missed integration points

## Best Practices

### File Reading Strategy
1. **Start with architecture**: Read `server.rs` first to understand current structure
2. **Understand handlers**: Review existing handler patterns before implementing transport
3. **Check dependencies**: Examine `Cargo.toml` for existing async/HTTP dependencies
4. **Review tests**: Look at existing test patterns before writing new tests

### Implementation Approach
1. **Incremental development**: Create transport module structure first, then add functionality
2. **Test-driven development**: Write tests alongside implementation
3. **Integration points**: Update server integration only after transport module is complete
4. **Dependency management**: Use edit_file for Cargo.toml only if absolutely necessary

### Code Organization
1. **Modular structure**: Keep transport logic separate from server logic
2. **Error handling**: Maintain consistency with existing error patterns
3. **Configuration**: Follow existing configuration management approaches
4. **Testing**: Place tests in appropriate directories following project conventions

## Tool Usage Patterns

### Discovery Pattern
```
1. list_directory -> understand structure
2. read_file -> examine key files
3. search_files -> find related patterns
4. read_file -> deep dive into specific implementations
```

### Implementation Pattern
```
1. write_file -> create new module skeleton
2. edit_file -> add module to lib.rs
3. write_file -> implement core functionality
4. write_file -> create comprehensive tests
```

### Integration Pattern
```
1. read_file -> verify current server implementation
2. edit_file -> update server to use new transport
3. read_file -> confirm changes are correct
4. search_files -> ensure no references to old transport remain
```

## Common Pitfalls to Avoid

### File Management
- **Don't overwrite important files**: Always read existing files before editing
- **Maintain backup approach**: Understand existing functionality before replacing
- **Preserve existing patterns**: Follow project conventions for error handling and structure

### Dependency Management
- **Minimal dependency changes**: Only add new dependencies if absolutely required
- **Version compatibility**: Ensure any new dependencies work with existing versions
- **Feature flags**: Use existing dependency features rather than adding new ones

### Integration Complexity
- **Incremental integration**: Don't update server integration until transport is complete
- **Preserve functionality**: Ensure all existing MCP tools continue to work
- **Test thoroughly**: Create comprehensive tests before integration

## Troubleshooting Guide

### File Not Found Issues
- Use `list_directory` to verify path structure
- Check for typos in file paths
- Ensure you're in the correct working directory context

### Integration Problems
- Use `search_files` to find all references to components being updated
- Read existing error handling patterns before implementing new ones
- Verify module declarations in lib.rs after adding new modules

### Test Failures
- Read existing test patterns to understand project testing conventions
- Use `list_directory` to find appropriate test directories
- Check Cargo.toml for existing test dependencies

### Compilation Errors
- Read dependency versions in Cargo.toml
- Check existing import patterns in similar modules
- Verify feature flags are consistent with project standards

## Success Indicators

### Code Quality
- All new code follows existing project patterns
- Error handling is consistent with project conventions
- Module structure matches project organization
- Tests cover all critical functionality paths

### Integration Success
- Server starts successfully with new transport
- All existing MCP tools continue to function
- Health checks work properly
- No compilation warnings or errors

### Functionality Verification
- New transport handles POST and GET requests properly
- Session management works correctly
- SSE streaming functions as expected
- Backward compatibility detection works

By following this tool usage guide, you'll be able to implement the Streamable HTTP Transport Foundation efficiently while maintaining code quality and integration consistency.