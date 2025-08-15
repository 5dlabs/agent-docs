# Autonomous Agent Prompt: Rust Crate Management with Background Ingestion

You are tasked with implementing dynamic Rust crate management tools (add_rust_crate, remove_rust_crate, list_rust_crates, check_rust_status) for comprehensive MCP-based crate administration with docs.rs integration.

## Your Mission

Create a complete Rust crate management system with automatic documentation fetching, atomic operations, health monitoring, and dependency analysis capabilities. The `add_rust_crate` tool must enqueue a background job and immediately return 202 + job id; the ingestion runs asynchronously.

## Execution Steps

### Step 1: Create Core Data Structures and Database Schema

- Extend database schema with 'crates' table containing:
  - id, name, version, description, documentation_url
  - last_updated, status (active/inactive/updating)
  - metadata JSONB field
- Create RustCrateManager struct in `crates/mcp/src/tools.rs`
- Add database pool, embedding client, and HTTP client fields
- Create CrateInfo and CrateStatus models in `crates/database/src/models.rs`
- Implement transaction helper methods for atomic operations

### Step 2: Implement add_rust_crate Tool with docs.rs Integration (Async)

- Build AddRustCrateTool following Tool trait pattern that enqueues a background job and returns 202 Accepted
- Add docs.rs API client in `crates/doc-loader/src/loaders.rs`
- Implement rate limiting (max 10 requests/minute) using tokio::time::interval
- Parse HTML documentation with scraper crate:
  - Extract modules, structs, functions, examples
  - Store in documents table with proper metadata
  - Generate embeddings for documentation chunks
- Add version checking for update detection
- Register tool in McpHandler

### Step 3: Implement remove_rust_crate Tool with Cascade Deletion

- Create RemoveRustCrateTool with transaction support
- Implement cascade deletion logic:
  - Query documents where metadata->>'crate_name' matches target
  - Delete associated embeddings using document IDs
  - Remove crate entry from crates table
  - Clean up orphaned embeddings
- Add soft-delete option with status='inactive'
- Implement comprehensive audit logging

### Step 4: Implement list_rust_crates Tool with Pagination

- Build ListRustCratesTool with configurable pagination
- Default 20 items per page with customizable limits
- Include comprehensive information:
  - Name, version, document count, last updated, status
  - Filtering by status and name pattern search
  - Statistics: total documents, embeddings, averages
- Add sorting options (name, version, last_updated, document_count)
- Format output as structured JSON with pagination metadata

### Step 5: Implement check_rust_status Tool and Dependency Analysis

- Create CheckRustStatusTool for comprehensive health monitoring
- Report system health metrics:
  - Database connectivity and performance
  - Storage usage statistics
  - Total counts for crates/documents/embeddings
- Implement cargo metadata parser using std::process::Command
- Add update detection comparing local vs docs.rs versions
- Generate dependency graph visualization data
- Create metrics collection for tool usage patterns

## Required Outputs

Generate these implementation artifacts:

1. **Database Schema Extensions** with crates table and transaction helpers
2. **RustCrateManager Core Structure** with all required clients
3. **Four Management Tools** (add, remove, list, check) fully implemented
4. **docs.rs Integration Client** with rate limiting and error handling
5. **Comprehensive Testing Suite** covering all operations and edge cases

## Key Technical Requirements

1. **Performance**: All operations complete within 30 seconds
2. **Reliability**: Atomic operations with proper rollback handling
3. **Rate Limiting**: Respect docs.rs API limits (10 req/min)
4. **Data Integrity**: Cascade deletions and orphan cleanup
5. **Monitoring**: Health checks and usage metrics

## Tools at Your Disposal

- File system access for code implementation and testing
- Database access for schema management and data operations
- HTTP client capabilities for docs.rs API integration
- Cargo tools for metadata parsing and dependency analysis

## Success Criteria

Your implementation is complete when:

- All four management tools are implemented and registered
- add_rust_crate enqueues a background job and returns 202 + job id
- check_rust_status reports real-time job status and final counts
- docs.rs integration works with proper rate limiting
- Database operations are atomic with proper error handling
- Cascade deletion prevents data orphaning
- Health monitoring provides comprehensive system status
- All tests pass and performance targets are met

## Important Implementation Notes

- Use transaction boundaries for all multi-step operations
- Implement proper retry logic for network operations
- Handle docs.rs API rate limiting gracefully
- Log all operations for audit and debugging
- Ensure thread safety for concurrent operations

## Database Schema Requirements

```sql
CREATE TABLE crates (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    version VARCHAR(50) NOT NULL,
    description TEXT,
    documentation_url TEXT,
    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(20) DEFAULT 'active',
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_crates_name ON crates(name);
CREATE INDEX idx_crates_status ON crates(status);
CREATE INDEX idx_crates_last_updated ON crates(last_updated);
```

## Tool Definitions Required

1. **add_rust_crate**: Parameters: name (required), version (optional)
2. **remove_rust_crate**: Parameters: name (required), soft_delete (boolean)
3. **list_rust_crates**: Parameters: page, limit, status_filter, name_pattern
4. **check_rust_status**: Parameters: include_dependencies (boolean)

## Validation Commands

Before completion, run:

```bash
cd /workspace
cargo test --package mcp --test crate_management
cargo test --package database --test crate_operations
cargo clippy --package mcp --lib
cargo fmt --all --check
```

Begin implementation with focus on data integrity, performance, and comprehensive error handling.## Quality Gates and CI/CD Process

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

## Final Deliverable: Pull Request Submission

**CRITICAL REQUIREMENT**: At the end of this task, you MUST submit a pull request to merge your feature branch into the main branch. This is a mandatory final step.

### Pull Request Requirements:
1. **Create the PR** after all CI/CD checks pass successfully
2. **Use descriptive title** following the pattern: `feat: Implement Rust Crate Management Tools (Task #11)`
3. **Write comprehensive description** including:
   - Summary of implemented features
   - List of all four management tools added
   - Database schema changes made
   - Testing coverage summary
   - Any breaking changes or migration notes
4. **Link to task** by mentioning "Closes #11" or "Implements Task #11"
5. **Request review** from appropriate team members
6. **Ensure all CI checks pass** before requesting review

### Success Criteria Update:
Your implementation is complete when:
- All four management tools are implemented and registered
- add_rust_crate enqueues a background job and returns 202 + job id
- check_rust_status reports real-time job status and final counts
- docs.rs integration works with proper rate limiting
- Database operations are atomic with proper error handling
- Cascade deletion prevents data orphaning
- Health monitoring provides comprehensive system status
- All tests pass and performance targets are met
- **A pull request is successfully submitted and ready for review**
