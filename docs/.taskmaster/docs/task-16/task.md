# Task 16: Basic Performance Validation for Small User Base

## Overview
Validate performance for a single-user environment (5–6 agents). Focus on correctness and baseline latency without heavy load or complex scaling.

## Implementation Guide
- Establish baseline latency for key operations (query, embedding, transport)
- Verify in-memory session/caching suffices; no Redis required at this stage
- Ensure connection keep-alive and timeouts behave under light concurrency
- Validate DB pool sizing for light use (e.g., 5–10 connections)
- Profile slow paths and capture actionable follow-ups

## Success Metrics
- P50/P95 query latency within targets (< 500ms / < 2s)
- Stable behavior with 5–6 concurrent agents
- No connection leaks or timeouts during 30-minute sessions
- DB pool saturation does not occur under expected usage## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.
