# Autonomous Agent Prompt: System Assessment and Migration Planning

You are tasked with performing a comprehensive evaluation of the Doc Server system to prepare for migration from deprecated HTTP+SSE transport to Streamable HTTP (MCP 2025-06-18).

## Your Mission

Conduct a thorough system assessment to identify all gaps between the current implementation and production requirements, focusing particularly on MCP transport migration needs.

## Execution Steps

### Step 1: Codebase Analysis
- Navigate to the project root and examine `Cargo.toml` for workspace structure
- Review each crate directory under `crates/`: database, mcp, embeddings, doc-loader, llm
- Identify and document all dependencies with versions
- Check for deprecated packages using `cargo outdated`

### Step 2: Transport Assessment
- Open `crates/mcp/src/server.rs` and analyze the current HTTP/SSE implementation
- Review `crates/mcp/src/transport.rs` to understand placeholder structure
- Compare current implementation against MCP 2025-06-18 specification
- Document all required changes for compliance

### Step 3: Database Validation
- Test PostgreSQL connection using environment variables
- Verify pgvector extension with dimension support (3072)
- Check schema: documents and document_sources tables
- Execute test vector operations to validate functionality

### Step 4: Test Execution
Run the following commands and document results:
```bash
cargo test --all --verbose
cargo clippy --all-targets --all-features
cargo fmt --all -- --check
```

### Step 5: Infrastructure Review
- Examine `.github/workflows/deploy-doc-server.yml`
- Check for Helm charts in `helm/` directory
- Review Dockerfile for container configuration
- Validate Kubernetes deployment manifests

### Step 6: Functionality Testing
- Start the MCP server locally if possible
- Test `/health` endpoint for server status
- Verify `rust_query` tool operation
- Test SSE connections at `/sse` endpoint

## Required Outputs

Generate the following artifacts:

1. **assessment_report.md** containing:
   - Current architecture overview
   - Component dependency analysis
   - Database schema validation results
   - Test execution summary

2. **migration_plan.md** containing:
   - Streamable HTTP migration steps
   - Risk assessment matrix
   - Timeline estimates
   - Rollback procedures

3. **gaps_analysis.md** containing:
   - Feature comparison table
   - Security findings
   - Performance baselines
   - Compliance requirements

## Key Areas of Focus

1. **Transport Migration**: Document exact changes needed for Streamable HTTP
2. **Session Management**: Identify requirements for Mcp-Session-Id implementation
3. **Security**: Note any security vulnerabilities or improvements needed
4. **Performance**: Baseline current performance metrics
5. **Database**: Verify pgvector functionality and schema integrity

## Tools at Your Disposal

- File system access for code review
- Command execution for testing
- Database query capabilities
- Documentation generation tools
- Git for version control analysis

## Success Criteria

Your assessment is complete when:
- All codebase components have been analyzed
- Database connectivity and schema are verified
- Test suite has been executed with results documented
- Migration requirements are clearly defined
- All deliverables are generated and saved

## Important Notes

- Document any blocking issues immediately
- Include code snippets for critical findings
- Provide specific file paths and line numbers for issues
- Create actionable recommendations for each gap identified
- Ensure all findings are reproducible with clear steps

Begin your assessment now and maintain detailed logs of all findings.