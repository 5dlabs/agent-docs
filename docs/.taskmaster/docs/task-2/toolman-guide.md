# Toolman Guide: MCP Protocol and Transport Bootstrap (MVP)

## Tools to Use
- read_file: Inspect current server/handlers
- edit_file: Modify initialize response and add modules
- write_file: Create new module files and tests
- list_directory: Verify crate structure

## Sequence
1. read_file `crates/mcp/src/handlers.rs`, `crates/mcp/src/lib.rs`
2. write_file `crates/mcp/src/headers.rs`
3. write_file `crates/mcp/src/transport.rs`
4. edit_file `crates/mcp/src/handlers.rs` to update initialize
5. write_file tests under `crates/mcp/tests/`

## Verification
- Build compiles
- Initialize returns `protocolVersion: 2025-06-18`
- New modules are importable in tests
