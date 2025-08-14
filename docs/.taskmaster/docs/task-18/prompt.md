# Autonomous Agent Prompt: Documentation and API Reference Generation

You are tasked with creating comprehensive documentation including API reference, deployment guides, integration examples, and architecture diagrams using cargo doc, OpenAPI/Swagger, and mermaid diagrams.

## Your Mission

Generate complete documentation ecosystem covering Rust API documentation, OpenAPI specification, Kubernetes deployment guides, MCP tool usage examples, and client integration guides for Cursor and Toolman.

## Execution Steps

### Step 1: Generate Rust API Documentation with cargo doc
- Use `cargo doc --all-features --no-deps` for comprehensive API documentation
- Configure rustdoc settings for external links and search indexes
- Generate documentation for all workspace crates (database, mcp, embeddings, doc-loader, llm)
- Include examples from code comments and ensure all public APIs documented
- Prepare documentation for hosting and integration

### Step 2: Create OpenAPI/Swagger Specification
- Create `openapi.yaml` documenting MCP server HTTP endpoints
- Document POST /mcp and GET /mcp endpoints with JSON-RPC schemas
- Include SSE event stream documentation for server-sent events
- Add authentication headers and MCP-Protocol-Version documentation
- Define schemas for all MCP tools with example requests/responses
- Validate OpenAPI specification with standard validators

### Step 3: Write Kubernetes Deployment Guide
- Document Helm chart creation in `helm/doc-server/` directory
- Write comprehensive `values.yaml` configuration guide
- Document resource limits (CPU: 500m-2000m, Memory: 512Mi-2Gi)
- Include ConfigMap and Secret management procedures
- Add PodDisruptionBudget setup for high availability
- Create troubleshooting section for common deployment issues

### Step 4: Document MCP Tool Usage with Client Integration
- Document rust_query tool usage with detailed examples
- Create Cursor IDE integration guide with MCP configuration
- Write Toolman client setup and usage documentation
- Include examples for all query tools (jupyter, cilium, talos, meteora, etc.)
- Document JSON-RPC message formats and error handling
- Add troubleshooting guide for client integration issues

### Step 5: Create Architecture Diagrams and Performance Guide
- Generate mermaid diagrams for system architecture
- Document data flow from ingestion through vector search
- Write performance tuning guide with optimization parameters
- Include benchmarks for vector search with different index types
- Document monitoring setup with metrics and alerting
- Create operational runbooks for production deployment

## Required Outputs

1. **Complete API Documentation** generated with cargo doc
2. **OpenAPI Specification** with comprehensive endpoint documentation
3. **Kubernetes Deployment Guide** with Helm charts and procedures
4. **Client Integration Guides** for Cursor and Toolman
5. **Architecture Documentation** with diagrams and performance guide

## Key Technical Requirements

1. **Completeness**: All public APIs and endpoints documented
2. **Accuracy**: Examples tested and validated for correctness
3. **Usability**: Clear step-by-step procedures and troubleshooting
4. **Standards Compliance**: OpenAPI 3.0 and documentation best practices
5. **Maintainability**: Documentation integrated with code generation

## Tools at Your Disposal

- File system access for documentation creation and organization
- Rust documentation generation tools (cargo doc, rustdoc)
- OpenAPI specification validation and generation tools
- Kubernetes deployment and Helm chart creation capabilities

## Success Criteria

Your documentation is complete when:
- Rust API documentation covers all public interfaces comprehensively
- OpenAPI specification enables client generation and integration
- Kubernetes deployment guide enables successful production deployment
- Client integration guides enable seamless MCP tool usage
- Architecture documentation provides complete system understanding
- All examples and procedures tested and validated

## Documentation Structure

```
docs/
├── api/                 # Generated API documentation
├── openapi.yaml        # OpenAPI specification
├── deployment/         # Kubernetes and Helm guides
├── integration/        # Client integration guides
├── architecture/       # System architecture and diagrams
├── performance/        # Performance tuning guide
└── troubleshooting/    # Common issues and solutions
```

## Validation Commands

```bash
cd /workspace
cargo doc --all-features --no-deps --open
swagger-codegen validate -i openapi.yaml
helm template helm/doc-server --validate
markdown-link-check docs/**/*.md
```

Begin documentation creation focusing on completeness, accuracy, and usability for end users and developers.## Quality Gates and CI/CD Process

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
## Worktree and Parallel Branching (Required for parallel tasks)

- Use Git worktrees to isolate this task's working directory and feature branch to avoid conflicts with other tasks running in parallel.

### Steps
1. Create a dedicated worktree and feature branch for this task:

2. Enter the worktree and do all work from there:

3. Run your development session here (e.g., Claude Code) and follow the Quality Gates section (Clippy pedantic after each new function; fmt/clippy/tests before pushing).

4. Push from this worktree and monitor GitHub Actions; create a PR only after CI is green and deployment succeeds.

5. Manage worktrees when finished:
/Users/jonathonfritz/code/work-projects/5dlabs/agent-docs  610a801 [main]
