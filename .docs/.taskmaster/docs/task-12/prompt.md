# Autonomous Agent Prompt: Additional Query Tools Suite Implementation

You are tasked with implementing the remaining query tools for jupyter, cilium, talos, meteora, raydium, ebpf, and rust_best_practices documentation types following the established QueryTool pattern.

## Your Mission

Create seven specialized query tools with type-specific metadata parsing, consistent response formatting, shared utility functions, and comprehensive caching strategies for optimal performance.

## Execution Steps

### Step 1: Create Shared Utility Module

- Create `crates/mcp/src/query_utils.rs` module
- Implement shared utility functions:
  - `parse_metadata_field()`: Generic JSONB field extraction with type conversion
  - `format_document_response()`: Consistent markdown formatting across all tools
  - `calculate_relevance_score()`: Similarity-based ranking algorithm
  - `create_performance_monitor()`: Query execution time tracking
  - `validate_query_params()`: Standard parameter validation
- Export utilities from lib.rs for use by all QueryTool implementations

### Step 2: Implement Jupyter and Cilium Query Tools

- Create `JupyterQueryTool` in `crates/mcp/src/tools.rs`:
  - Handle notebook-specific metadata (kernel, language, cell_types)
  - Parse cell execution results and outputs
  - Format notebook content with proper code highlighting
- Create `CiliumQueryTool`:
  - Process network policy metadata (policy_type, namespace, endpoints)
  - Handle Kubernetes networking configuration
  - Format policy rules and security contexts
- Add `jupyter_vector_search` and `cilium_vector_search` methods to `crates/database/src/queries.rs`
- Register both tools in McpHandler with comprehensive tool definitions

### Step 3: Implement Talos and Meteora Query Tools

- Create `TalosQueryTool`:
  - Parse Kubernetes-specific metadata (resource_type, api_version, namespace)
  - Handle Talos node configuration and cluster setup
  - Format system configuration and boot sequences
- Create `MeteoraQueryTool`:
  - Process DeFi protocol metadata (pool_type, liquidity_params, reward_structure)
  - Handle liquidity pool configurations and AMM parameters
  - Format financial calculations and yield strategies
- Add corresponding database vector search methods
- Implement custom response formatting for technical specifications

### Step 4: Implement Raydium and eBPF Query Tools

- Create `RaydiumQueryTool`:
  - Handle AMM and liquidity pool metadata (amm_version, pool_address, fee_structure)
  - Process Solana-based DeFi protocol parameters
  - Format swap mechanics and liquidity provision details
- Create `EbpfQueryTool`:
  - Parse kernel programming metadata (program_type, kernel_version, hook_points)
  - Handle BPF program lifecycle and attachment points
  - Format code examples and kernel integration patterns
- Add caching strategy using `tokio::sync::RwLock` for frequently accessed technical docs
- Register tools in MCP handler with proper error handling

### Step 5: Implement RustBestPractices Tool and Integration Testing

- Create `RustBestPracticesQueryTool`:
  - Handle pattern/anti-pattern metadata (practice_category, rust_version, complexity_level)
  - Parse best practice recommendations and code patterns
  - Format examples with before/after code comparisons
- Add `rust_best_practices_vector_search` to queries.rs
- Create comprehensive integration tests in `crates/mcp/tests/`:
  - Test query accuracy for all seven tools
  - Validate metadata filtering functionality
  - Test response formatting consistency
  - Verify error handling and edge cases
  - Performance benchmarks (< 2 seconds per query)
- Verify MCP protocol compliance for all tools

## Required Outputs

Generate these implementation artifacts:

1. **Shared Utilities Module** with common functions for all query tools
2. **Seven Specialized Query Tools** following consistent patterns
3. **Database Query Methods** for each documentation type
4. **Caching Infrastructure** for performance optimization
5. **Comprehensive Test Suite** covering all functionality and edge cases

## Key Technical Requirements

1. **Consistency**: All tools follow identical patterns from RustQueryTool
2. **Performance**: Each tool responds within 2 seconds
3. **Metadata Handling**: Type-specific metadata parsing for each domain
4. **Caching**: Shared caching strategy for frequently accessed content
5. **Testing**: 100% coverage of core functionality with integration tests

## Documentation Type Specifications

### Jupyter Notebooks

- **Metadata Fields**: kernel, language, cell_types, execution_count
- **Content Types**: Code cells, markdown cells, output results
- **Formatting**: Syntax highlighting, execution results display

### Cilium Networking

- **Metadata Fields**: policy_type, namespace, endpoints, rules
- **Content Types**: Network policies, security contexts, traffic rules
- **Formatting**: YAML configuration blocks, network diagrams

### Talos Kubernetes

- **Metadata Fields**: resource_type, api_version, namespace, node_config
- **Content Types**: System configurations, boot sequences, cluster setup
- **Formatting**: Configuration files, system commands

### Meteora DeFi

- **Metadata Fields**: pool_type, liquidity_params, reward_structure, apy
- **Content Types**: Liquidity strategies, yield calculations, protocol mechanics
- **Formatting**: Financial formulas, pool configurations

### Raydium AMM

- **Metadata Fields**: amm_version, pool_address, fee_structure, trading_pairs
- **Content Types**: Swap mechanics, liquidity provision, price calculations
- **Formatting**: Trading parameters, pool statistics

### eBPF Programming

- **Metadata Fields**: program_type, kernel_version, hook_points, verifier_info
- **Content Types**: BPF programs, kernel integration, performance metrics
- **Formatting**: C code examples, assembly output

### Rust Best Practices

- **Metadata Fields**: practice_category, rust_version, complexity_level, pattern_type
- **Content Types**: Code patterns, anti-patterns, recommendations
- **Formatting**: Before/after examples, best practice explanations

## Tools at Your Disposal

- File system access for implementation and testing
- Database access for query development and optimization
- Existing query tool patterns for reference
- Performance monitoring tools for optimization

## Success Criteria

Your implementation is complete when:

- All seven query tools are implemented following consistent patterns
- Shared utility module provides reusable functionality
- Database queries are optimized with proper indexing
- Caching strategy improves performance for repeated queries
- All tools are registered and available through MCP
- Comprehensive test suite validates all functionality
- Performance targets are consistently met

## Important Implementation Notes

- Reuse existing RustQueryTool patterns exactly
- Implement proper error handling for each domain-specific case
- Use shared utilities to maintain consistency
- Add appropriate logging for debugging and monitoring
- Ensure thread safety for concurrent operations

## Validation Commands

Before completion, run:

```bash
cd /workspace
cargo test --package mcp --test query_tools_integration
cargo test --package mcp query_tools
cargo clippy --package mcp --lib
cargo fmt --package mcp --check
cargo doc --package mcp --no-deps
```

Begin implementation with focus on consistency, performance, and comprehensive testing across all seven query tools.## Quality Gates and CI/CD Process

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
