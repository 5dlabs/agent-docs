# Acceptance Criteria: Task 11 - Additional Query Tools Implementation

## Functional Requirements

### 1. Shared Utility Module
- [ ] `crates/mcp/src/query_utils.rs` with reusable helpers:
  - [ ] `parse_metadata_field()` safe JSONB extraction
  - [ ] `format_document_response()` standardized formatting
  - [ ] `calculate_relevance_score()` ranking
  - [ ] `create_performance_monitor()` timing
  - [ ] `validate_query_params()` validation

### 2. Implement Remaining Query Tools
- [ ] `jupyter_query` with notebook metadata and cell formatting
- [ ] `cilium_query` with network policy metadata
- [ ] `talos_query` with Kubernetes/Talos specifics
- [ ] `meteora_query` with DeFi pool params
- [ ] `raydium_query` with AMM details
- [ ] `ebpf_query` with kernel/BPF metadata
- [ ] `rust_best_practices_query` with pattern/anti-patterns
- [ ] Database query methods for each tool type
- [ ] MCP tool registrations with full JSON schema

### 3. Consistency and Performance
- [ ] Consistent formatting across all tools
- [ ] Response time < 2 seconds per tool query
- [ ] Proper error handling and parameter validation

## Non-Functional Requirements
- [ ] Shared utilities used across implementations
- [ ] Thread-safe operations; no data races
- [ ] Minimal duplication; utilities reduce copy-paste

## Test Cases
- [ ] Each tool returns relevant results with correct metadata
- [ ] Bad inputs handled gracefully with clear errors
- [ ] Response formatting consistent across tools
- [ ] Performance under light concurrency (5–6 agents) within target

## Deliverables
- [ ] Seven query tools implemented and registered
- [ ] Utility module with tests
- [ ] Database query methods with tests
- [ ] Integration tests covering all tools

## Validation
### Automated
```bash
cargo test --package mcp query_tools
cargo test --package mcp --test query_tools_integration
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

### Manual
1. Tools visible via MCP tools list
2. End-to-end queries in dev environment
3. Formatting and metadata verified for sample queries

## Deployment Validation (4-step)
1. Push to GitHub → CI build/tests
2. Image built and published
3. Helm deploy to cluster
4. Real-world queries via MCP client confirm functionality### NFR-0: Code Quality and Automation
- [ ] After adding any new function, run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and fix all warnings before continuing
- [ ] Prior to submission, ensure `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` all pass locally
- [ ] All changes pushed to a feature branch; GitHub Actions must complete successfully (including deployment) before opening a PR
- [ ] PR creation is gated on a green CI pipeline and successful deployment of the server artifact
