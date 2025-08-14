# Autonomous Agent Prompt: Solana Query Tool Implementation

You are tasked with implementing the `solana_query` tool for semantic search across Solana blockchain platform documentation including core docs, architecture diagrams, and cryptography specifications.

## Your Mission

Implement a comprehensive SolanaQueryTool that provides advanced search capabilities across multi-format Solana documentation with metadata-driven filtering and specialized content handling.

## Execution Steps

### Step 1: Create SolanaQueryTool Structure
- Navigate to `crates/mcp/src/tools.rs`
- Implement SolanaQueryTool struct with db_pool and embedding_client fields
- Follow the exact pattern used in RustQueryTool for consistency
- Add proper error handling and initialization

### Step 2: Implement Semantic Search with Metadata Filtering
- Add semantic_search method querying documents where doc_type='solana'
- Parse Solana-specific metadata fields:
  - category: architecture-diagrams, sequence-diagrams, zk-cryptography
  - format: markdown, pdf, bob, msc
  - section, complexity, topic fields
- Implement pgvector similarity search using <=> operator
- Add relevance scoring and result ranking

### Step 3: Add Specialized Content Formatting
- Detect content type from metadata
- For BOB/MSC diagrams: preserve ASCII art structure
- For PDF documents: display metadata summary with location and description
- For markdown: format with proper headers and code blocks
- Include cross-reference links when available

### Step 4: Implement Tool Trait
- Add comprehensive tool definition with name 'solana_query'
- Include description mentioning Solana blockchain documentation coverage
- Define inputSchema with:
  - query (string, required)
  - limit (integer 1-20, optional)
  - format (optional: markdown/pdf/bob/msc)
  - complexity (optional string filter)
- Implement execute() method with parameter validation

### Step 5: Register Tool in MCP Handler
- Navigate to `crates/mcp/src/handlers.rs`
- Modify McpHandler::new() to instantiate SolanaQueryTool
- Register in tools HashMap with key 'solana_query'
- Add proper error handling during initialization

## Required Outputs

Generate these implementation artifacts:

1. **SolanaQueryTool struct** in tools.rs with proper initialization
2. **Metadata parsing logic** for all Solana-specific fields
3. **Content formatting handlers** for different document types
4. **Tool trait implementation** with comprehensive definition
5. **MCP handler integration** with proper registration

## Key Technical Requirements

1. **Performance**: Query response time < 2 seconds
2. **Compatibility**: Follow existing RustQueryTool patterns exactly
3. **Metadata Support**: Handle all documented Solana metadata fields
4. **Format Handling**: Support markdown, PDF, BOB diagrams, MSC charts
5. **Error Handling**: Graceful degradation for missing metadata

## Tools at Your Disposal

- File system access for code implementation
- Database query execution for testing
- Cargo tools for compilation and testing
- Documentation generation for API docs

## Success Criteria

Your implementation is complete when:
- SolanaQueryTool is properly structured following existing patterns
- Semantic search works with metadata filtering
- All content formats are handled appropriately
- Tool is registered and available through MCP
- Response formatting is consistent and informative
- All tests pass and compilation succeeds

## Important Implementation Notes

- Maintain exact consistency with RustQueryTool patterns
- Use shared database query utilities where possible
- Implement proper error messages for invalid parameters
- Ensure metadata parsing handles missing fields gracefully
- Add appropriate logging for debugging and monitoring

## Validation Commands

Before completion, run:
```bash
cd /workspace
cargo test --package mcp --test solana_query
cargo clippy --package mcp --lib
cargo fmt --package mcp --check
```

Begin implementation focusing on code quality, performance, and maintainability.## Quality Gates and CI/CD Process

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
