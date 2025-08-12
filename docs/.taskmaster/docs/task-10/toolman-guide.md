# Toolman Guide: Task 10 - Rust Crate Management Tools Implementation

## Overview

This task implements comprehensive Rust crate management tools for dynamic crate administration through MCP. The tools provide full lifecycle management including automatic docs.rs integration, atomic operations, and health monitoring.

## Core Tools

### Filesystem Server Tools

Essential for implementing the crate management system with proper database integration and testing.

#### read_file
**Purpose**: Analyze existing patterns and examine current database schema
**When to Use**: 
- Study existing tool implementations for patterns
- Examine database models and migration files
- Review docs.rs API integration examples
- Analyze transaction handling patterns

**Usage Example**:
```
read_file("/workspace/crates/database/src/models.rs")
read_file("/workspace/crates/mcp/src/tools.rs")
```

#### write_file
**Purpose**: Create new implementation files and database migrations
**When to Use**:
- Implement RustCrateManager and tool structures
- Create database migration scripts
- Write comprehensive test suites
- Add docs.rs API client implementation

**Usage Example**:
```
write_file("/workspace/migrations/add_crates_table.sql", "CREATE TABLE crates...")
write_file("/workspace/crates/doc-loader/src/docs_rs_client.rs", "impl DocsRsClient...")
```

#### edit_file
**Purpose**: Modify existing files to integrate new crate management functionality
**When to Use**:
- Register new tools in MCP handler
- Update database connection pool configuration
- Modify existing models to support crate metadata
- Add new tool exports to module files

**Usage Example**:
```
edit_file("/workspace/crates/mcp/src/handlers.rs", register_crate_tools)
```

### Remote Crate Management Tools

#### add_rust_crate
**Purpose**: Add new Rust crates with automatic documentation fetching
**When to Use**:
- Adding popular crates like tokio, serde, axum
- Testing documentation parsing and storage
- Validating embedding generation
- Testing rate limiting behavior

**Parameters**:
- `name`: Crate name (required)
- `version`: Specific version (optional, defaults to latest)

**Usage Example**:
```
add_rust_crate(name="tokio", version="1.0")
```

#### remove_rust_crate
**Purpose**: Remove crates with cascade deletion of associated data
**When to Use**:
- Cleaning up test crates
- Removing outdated or deprecated crates
- Testing cascade deletion logic
- Validating orphan cleanup

**Parameters**:
- `name`: Crate name to remove (required)
- `soft_delete`: Boolean to preserve data with inactive status (optional)

**Usage Example**:
```
remove_rust_crate(name="test-crate", soft_delete=false)
```

#### list_rust_crates
**Purpose**: List all managed crates with pagination and filtering
**When to Use**:
- Viewing all available crates
- Testing pagination functionality
- Filtering by status or name patterns
- Monitoring crate statistics

**Parameters**:
- `page`: Page number (default 1)
- `limit`: Items per page (default 20, max 100)
- `status_filter`: active/inactive/updating (optional)
- `name_pattern`: Search pattern (optional)

**Usage Example**:
```
list_rust_crates(page=2, limit=10, status_filter="active")
```

#### check_rust_status
**Purpose**: Monitor system health and crate management statistics
**When to Use**:
- Checking system health before operations
- Monitoring storage usage and performance
- Validating update detection logic
- Generating dependency analysis

**Parameters**:
- `include_dependencies`: Include cargo metadata analysis (optional)

**Usage Example**:
```
check_rust_status(include_dependencies=true)
```

## Implementation Flow

### Phase 1: Database Schema Setup
1. Use `read_file` to examine existing database migration patterns
2. Use `write_file` to create crates table migration script
3. Add proper indexes for performance optimization
4. Create database model structures

### Phase 2: Core Infrastructure Implementation
1. Implement RustCrateManager struct with required clients
2. Create docs.rs API client with rate limiting
3. Add HTML parsing utilities using scraper crate
4. Implement transaction helper methods

### Phase 3: Tool Implementation
1. **AddRustCrateTool**: Documentation fetching and parsing
2. **RemoveRustCrateTool**: Cascade deletion with transaction support
3. **ListRustCratesTool**: Pagination and filtering logic
4. **CheckRustStatusTool**: Health monitoring and statistics

### Phase 4: Integration and Testing
1. Register all tools in MCP handler
2. Create comprehensive test suites
3. Validate rate limiting and error handling
4. Test concurrent operations and data integrity

## Best Practices

### Database Operations
- Always use transactions for multi-step operations
- Implement proper rollback on any failure
- Use prepared statements to prevent SQL injection
- Add comprehensive error logging for debugging

### docs.rs Integration
- Respect rate limits (10 requests per minute)
- Implement retry logic with exponential backoff
- Handle network failures gracefully
- Cache parsed documentation appropriately

### Performance Optimization
- Use database indexes for frequently queried fields
- Batch embedding generation operations
- Implement connection pooling for HTTP clients
- Monitor memory usage during large operations

### Error Handling
- Provide clear, actionable error messages
- Log all errors with sufficient context
- Implement graceful degradation for partial failures
- Validate all input parameters before processing

## Task-Specific Implementation Guidelines

### 1. Database Schema Design
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
```

### 2. Rate Limiting Implementation
```rust
use tokio::time::{interval, Duration};

struct DocsRsClient {
    rate_limiter: interval,
    http_client: reqwest::Client,
}

impl DocsRsClient {
    async fn fetch_docs(&mut self, crate_name: &str) -> Result<String> {
        self.rate_limiter.tick().await; // Wait for rate limit
        // Fetch documentation...
    }
}
```

### 3. Transaction Pattern
```rust
async fn add_crate_with_docs(&self, name: &str) -> Result<()> {
    let mut tx = self.db_pool.begin().await?;
    
    // Insert crate record
    let crate_id = self.insert_crate(&mut tx, name).await?;
    
    // Parse and store documentation
    match self.store_documentation(&mut tx, crate_id).await {
        Ok(_) => tx.commit().await?,
        Err(e) => {
            tx.rollback().await?;
            return Err(e);
        }
    }
    
    Ok(())
}
```

### 4. Cascade Deletion Logic
```rust
async fn remove_crate(&self, name: &str) -> Result<()> {
    let mut tx = self.db_pool.begin().await?;
    
    // Find all documents for this crate
    let doc_ids = self.find_crate_documents(&mut tx, name).await?;
    
    // Delete embeddings for these documents
    self.delete_embeddings(&mut tx, &doc_ids).await?;
    
    // Delete the documents
    self.delete_documents(&mut tx, &doc_ids).await?;
    
    // Delete the crate record
    self.delete_crate(&mut tx, name).await?;
    
    tx.commit().await?;
    Ok(())
}
```

## Troubleshooting

### Common Implementation Issues

#### Database Connection Failures
- Check database URL configuration
- Verify connection pool settings
- Test database connectivity before operations
- Monitor connection pool exhaustion

#### docs.rs API Issues
- Handle rate limiting responses (429 status)
- Implement proper User-Agent headers
- Check for API endpoint changes
- Monitor for temporary service unavailability

#### HTML Parsing Problems
- Validate HTML structure before parsing
- Handle missing or malformed documentation
- Provide fallback for parsing failures
- Log parsing errors with document context

#### Memory Usage During Large Operations
- Stream large documentation parsing
- Batch embedding generation
- Monitor memory usage during operations
- Implement backpressure for concurrent requests

### Performance Issues

#### Slow Documentation Fetching
- Use connection pooling for HTTP requests
- Implement parallel fetching with rate limiting
- Cache frequently accessed documentation
- Optimize HTML parsing with targeted selectors

#### Database Query Performance
- Add indexes for metadata JSONB queries
- Use EXPLAIN ANALYZE for query optimization
- Implement query result caching
- Monitor slow query logs

## Validation Steps

### Development Testing
1. **Unit Tests**: All manager methods and utilities
2. **Integration Tests**: Complete tool workflows
3. **Performance Tests**: Rate limiting and response times
4. **Error Handling**: Network failures and edge cases

### System Testing
1. **Load Testing**: Concurrent crate operations
2. **Data Integrity**: Transaction rollback scenarios
3. **Resource Usage**: Memory and database connections
4. **Rate Limiting**: docs.rs API compliance

### Quality Assurance
```bash
# Run these validation commands
cargo test --package mcp crate_management
cargo test --package database crate_operations
cargo test --package doc-loader docs_rs_client
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Success Indicators

- All four crate management tools operational
- docs.rs integration working with proper rate limiting
- Database transactions maintain data integrity
- Cascade deletion prevents orphaned data
- Health monitoring provides accurate system status
- Performance targets consistently met
- Error handling graceful and informative
- Concurrent operations safe and reliable