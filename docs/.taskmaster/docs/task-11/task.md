# Task 11: Rust Crate Management (MCP add/remove/list/status with Background Ingestion)

## Overview

Implement Rust crate management via MCP tools with non-blocking ingestion: `add_rust_crate`, `remove_rust_crate`, `list_rust_crates`, `check_rust_status`. Calling `add_rust_crate` MUST enqueue a background job that fetches from docs.rs, parses, chunks, embeds, and stores docs. The request returns 202 Accepted with a job ID; progress is available via `check_rust_status`. Persist job state in a `crate_jobs` table so job IDs survive restarts.

## Implementation Guide

- Create crate management tools (MCP) and background ingestion pipeline
- `add_rust_crate` enqueues job; returns 202 + job id; job performs fetch→parse→chunk→embed→store
- `check_rust_status` reports job states (queued, running, failed, complete) and counts
- `remove_rust_crate` supports cascade delete (docs + embeddings) with soft-delete option
- `list_rust_crates` paginates with stats (doc/embedding counts, last updated)

## Technical Requirements

- Background job runner (tokio task/queue) with retry and rate limiting (docs.rs)
- Persisted jobs table with fields: id (UUID), crate_name, operation (ingest/remove), status (queued|running|failed|complete), progress (0-100), started_at, finished_at, error (TEXT)
- Atomic DB operations with transactions for writes/deletes
- Consistent error handling and audit logs
- Performance limits (end-to-end ingestion within reasonable time)

## Success Metrics

- `add_rust_crate` returns 202 + job id and does not block
- `check_rust_status` reports real-time progress and final counts
- Ingestion produces documents + embeddings linked to crate metadata## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.
