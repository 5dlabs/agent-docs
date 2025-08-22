# Task ID: 11
# Title: Implement Rust Crate Management Tools
# Status: pending
# Dependencies: 6
# Priority: medium
# Description: Create dynamic Rust crate management tools (add_rust_crate, remove_rust_crate, list_rust_crates, check_rust_status) for MCP-based crate administration.
# Details:
Implement add_rust_crate with automatic docs.rs fetching and parsing. Create remove_rust_crate with cascade deletion of documents and embeddings. Build list_rust_crates with pagination and status information. Implement check_rust_status for health monitoring and statistics. Add transaction support for atomic operations. Implement crate version management and update detection. Use cargo metadata parsing for dependency analysis. Add rate limiting for docs.rs API calls.

# Test Strategy:
Test adding popular crates (tokio, serde, axum), validate document extraction completeness, test removal with orphan cleanup, verify transaction rollback on errors, test concurrent management operations, and validate status reporting accuracy.

# Subtasks:
## 1. Create Rust Crate Management Tool Structures and Database Schema [pending]
### Dependencies: None
### Description: Design and implement the core data structures for crate management tools and extend the database schema to support crate-specific operations
### Details:
Create RustCrateManager struct in crates/mcp/src/tools.rs with fields for database pool, embedding client, and HTTP client for docs.rs API. Extend database schema with a 'crates' table containing fields: id, name, version, description, documentation_url, last_updated, status (active/inactive/updating), and metadata JSONB. Add indexes for name and version lookups. Create CrateInfo and CrateStatus structs in crates/database/src/models.rs. Implement transaction helper methods in DatabasePool for atomic operations.

## 2. Implement add_rust_crate Tool with docs.rs Integration [pending]
### Dependencies: 10.1
### Description: Build the add_rust_crate tool with automatic documentation fetching from docs.rs and parsing capabilities
### Details:
Implement AddRustCrateTool in crates/mcp/src/tools.rs following the Tool trait pattern. Add docs.rs API client in crates/doc-loader/src/loaders.rs with rate limiting (max 10 requests/minute) using tokio::time::interval. Parse HTML documentation using scraper crate to extract modules, structs, functions, and examples. Store parsed documentation in documents table with proper metadata (crate_name, version, module_path). Generate embeddings for documentation chunks. Implement version checking to detect updates. Add the tool to McpHandler registry in handlers.rs.

## 3. Implement remove_rust_crate Tool with Cascade Deletion [pending]
### Dependencies: 10.1, 10.2
### Description: Create the remove_rust_crate tool with proper cascade deletion of associated documents and embeddings
### Details:
Implement RemoveRustCrateTool with transaction support for atomic deletion. Query and delete all documents where metadata->>'crate_name' matches the target crate. Delete associated embeddings from embeddings table using document IDs. Remove crate entry from crates table. Implement orphan cleanup to identify and remove dangling embeddings. Add soft-delete option with status='inactive' for recoverable deletions. Log all deletion operations for audit trail.

## 4. Implement list_rust_crates Tool with Pagination [pending]
### Dependencies: 10.1, 10.2
### Description: Build the list_rust_crates tool with pagination support and comprehensive status information display
### Details:
Implement ListRustCratesTool with configurable pagination (default 20 items/page). Return crate information including name, version, document count, last updated timestamp, and status. Add filtering options by status (active/inactive/updating) and search by crate name pattern. Include statistics: total documents, total embeddings, average documents per crate. Implement sorting options (name, version, last_updated, document_count). Format output as structured JSON with metadata about pagination (current_page, total_pages, total_items).

## 5. Implement check_rust_status Tool and Dependency Analysis [pending]
### Dependencies: 10.1, 10.2, 10.3, 10.4
### Description: Create the check_rust_status tool for health monitoring and implement cargo metadata parsing for dependency analysis
### Details:
Implement CheckRustStatusTool to report overall system health including database connectivity, total crates/documents/embeddings count, storage usage statistics, and last update timestamps. Add cargo metadata parser using std::process::Command to extract crate dependencies from Cargo.toml files. Implement update detection by comparing local versions with docs.rs latest versions. Create health check endpoints for monitoring integration. Add metrics collection for tool usage patterns and query performance. Generate dependency graph visualization data in JSON format.

