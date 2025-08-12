# Task 1: Comprehensive System Assessment and Migration Planning

## Overview

This task involves performing a thorough evaluation of the existing Doc Server codebase, infrastructure, and database state to identify gaps between current implementation and production requirements, with particular focus on MCP transport migration needs.

## Background

The Doc Server is transitioning from a deprecated HTTP+SSE transport to the new Streamable HTTP transport (MCP 2025-06-18). A comprehensive assessment is critical to understand the current state, identify migration requirements, and plan the upgrade path.

## Implementation Guide

### Phase 1: Codebase Analysis

1. **Rust Crate Structure Review**
   - Examine the workspace structure in `Cargo.toml`
   - Review each crate: `database`, `mcp`, `embeddings`, `doc-loader`, `llm`
   - Document dependencies and version compatibility
   - Identify deprecated packages requiring updates

2. **Transport Implementation Assessment**
   - Analyze current HTTP/SSE implementation in `crates/mcp/src/server.rs`
   - Review `transport.rs` placeholder for migration readiness
   - Document differences between current implementation and Streamable HTTP spec
   - Identify required changes for MCP 2025-06-18 compliance

### Phase 2: Database and Infrastructure

1. **Database Migration Status**
   - Verify PostgreSQL cluster connectivity
   - Check pgvector extension (3072 dimensions) availability
   - Validate `documents` and `document_sources` tables
   - Review migration scripts in `crates/database/src/migrations.rs`
   - Test vector operations with sample queries

2. **Kubernetes Deployment Review**
   - Examine `.github/workflows/deploy-doc-server.yml`
   - Validate Helm chart configuration if present
   - Check container registry settings
   - Review resource allocations and scaling parameters

### Phase 3: Functionality Testing

1. **Existing Tool Validation**
   - Test `rust_query` tool functionality
   - Verify MCP server health checks at `/health`
   - Test SSE endpoint at `/sse`
   - Validate JSON-RPC message handling

2. **Test Suite Execution**
   ```bash
   cargo test --all --verbose
   cargo clippy --all-targets --all-features
   cargo fmt --all -- --check
   ```

### Phase 4: Gap Analysis

1. **Transport Migration Requirements**
   - Document required changes for Streamable HTTP
   - Identify session management implementation needs
   - List security enhancements required
   - Plan backward compatibility strategy

2. **Production Readiness Gaps**
   - Database connection pooling improvements
   - Monitoring and observability additions
   - Security hardening requirements
   - Performance optimization opportunities

## Technical Requirements

### Tools and Environment
- Rust toolchain (stable)
- PostgreSQL client tools
- kubectl for Kubernetes validation
- GitHub Actions CLI for workflow testing

### Access Requirements
- Read access to all source code
- Database connection credentials
- Kubernetes cluster access
- GitHub repository permissions

### Environment Configuration
The project uses `requirements.yaml` file to define environment variables and secrets for Kubernetes deployment. Key environment variables that must be configured include:
- **VECTOR_DATABASE_URL**: Connection string for the PostgreSQL database with pgvector extension
- **DATABASE_URL**: Connection string for the main PostgreSQL database 
- **OPENAI_API_KEY**: API key for OpenAI embedding service
 - **DOC_SERVER_CONFIG_PATH**: Absolute path inside the pod to the config file controlling tools and ingestion (see below)

Additional configuration is defined in `requirements.yaml` including rate limiting, batch processing settings, and monitoring parameters. Review this file during the assessment to understand the complete configuration requirements.

### Config-Driven Tools and Ingestion

- Primary config file example for this task: `docs/.taskmaster/docs/task-1/doc-server.config.yaml`
- At runtime, set `DOC_SERVER_CONFIG_PATH` to the absolute path where the server can read this file inside the container.
- See `config.md` in this directory for a schema summary and usage notes.

## Expected Deliverables

1. **System Assessment Report**
   - Current architecture documentation
   - Component dependency map
   - Database schema validation results
   - Test suite execution report

2. **Migration Plan Document**
   - Detailed migration steps for Streamable HTTP
   - Risk assessment and mitigation strategies
   - Timeline and resource requirements
   - Rollback procedures

3. **Gap Analysis Matrix**
   - Feature comparison (current vs. required)
   - Security assessment findings
   - Performance baseline metrics
   - Compliance requirements checklist

## Code Examples

### Database Connectivity Test
```rust
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;

async fn test_database_connection() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")?;
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    // Test pgvector extension
    let result = sqlx::query("SELECT version()")
        .fetch_one(&pool)
        .await?;
    
    println!("PostgreSQL version: {}", result.get::<String, _>(0));
    
    // Test vector operations
    let vector_test = sqlx::query(
        "SELECT '[1,2,3]'::vector(3) <=> '[4,5,6]'::vector(3) as distance"
    )
    .fetch_one(&pool)
    .await?;
    
    println!("Vector distance: {}", vector_test.get::<f32, _>("distance"));
    
    Ok(())
}
```

### MCP Health Check Test
```rust
use reqwest;

async fn test_mcp_health() -> Result<(), Box<dyn std::error::Error>> {
    let health_url = "http://localhost:3001/health";
    let response = reqwest::get(health_url).await?;
    
    assert_eq!(response.status(), 200);
    
    let body = response.text().await?;
    println!("Health check response: {}", body);
    
    Ok(())
}
```

## Dependencies

This task has no dependencies and serves as the foundation for all subsequent tasks.

## Risk Considerations

1. **Discovery Risk**: Unknown technical debt or architectural issues
2. **Access Risk**: Insufficient permissions for comprehensive testing
3. **Data Risk**: Production data sensitivity during assessment
4. **Time Risk**: Assessment scope larger than anticipated

## Success Metrics

- Complete codebase analysis documented
- All tests executed with results recorded
- Migration requirements clearly defined
- Production readiness gaps identified
- Stakeholder alignment on findings

## Timeline Estimate

- Codebase Analysis: 2 days
- Database/Infrastructure Review: 1 day
- Functionality Testing: 1 day
- Documentation and Reporting: 1 day
- **Total: 5 days**