# Acceptance Criteria: Task 11 - Rust Crate Management with Background Ingestion

## Functional Requirements

### 1. Tools and Job Queue

- [ ] `add_rust_crate` enqueues background ingestion and returns 202 + job id
- [ ] `check_rust_status` reports job state (queued, running, failed, complete) and counts
- [ ] `remove_rust_crate` performs cascade delete with soft-delete option
- [ ] `list_rust_crates` paginates with stats (doc/embedding counts, last updated)

### 2. Database and Storage

- [ ] New `crates` table present; atomic transactions used
- [ ] Documents and embeddings written with crate metadata
- [ ] Deletions remove related docs/embeddings without orphans
- [ ] Audit logs for add/remove/status
- [ ] New `crate_jobs` table persists job state and history (UUID id, crate_name, operation, status, progress, started_at, finished_at, error) so job IDs survive restarts

### 3. Performance and Reliability

- [ ] Ingestion completes within reasonable time bounds
- [ ] Rate limits respected for docs.rs (10 req/min) with retries
- [ ] Proper error handling and idempotency for repeated requests

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

- [ ] Four management tools implemented and registered
- [ ] Job queue/runner code with tests
- [ ] Database schema and queries for crate ops
- [ ] Integration tests for add/remove/list/status

## Validation

### Automated

```bash
cargo test --package mcp --test crate_management
cargo test --package database --test crate_operations
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

### Manual

1. Tools visible via MCP tools list (add/remove/list/check)
2. Add crate returns 202 and job id; status shows progress; docs appear on completion
3. Remove crate deletes docs/embeddings; list reflects changes

## Deployment Validation (4-step)

1. Push to GitHub → CI build/tests
2. Image built and published
3. Helm deploy to cluster
4. Real-world queries via MCP client confirm functionality### NFR-0: Code Quality and Automation

- [ ] After adding any new function, run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and fix all warnings before continuing
- [ ] Prior to submission, ensure `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` all pass locally
- [ ] All changes pushed to a feature branch; GitHub Actions must complete successfully (including deployment) before opening a PR
- [ ] PR creation is gated on a green CI pipeline and successful deployment of the server artifact
