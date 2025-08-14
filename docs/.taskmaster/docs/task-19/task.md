# Task 19: Basic Security for Single-User Environment

## Overview
Implement practical, lightweight security measures appropriate for a single-user environment (5–6 agents). Emphasize sane defaults without heavy compliance overhead.

## Implementation Guide
- Origin validation and localhost binding for MCP server
- Input size limits and basic request validation
- Rate limiting suitable for single-user (low thresholds)
- Secrets via environment variables; avoid logging sensitive data
- Minimal audit logs for tool invocations and errors

## Technical Requirements
- CORS/Origin checks; DNS rebinding protections
- Request size limits (e.g., 1–2 MB)
- Simple token-bucket rate limiter
- Structured logging with redaction

## Success Metrics
- No P0/P1 vulnerabilities in basic scan
- Blocks cross-origin and oversized request attempts
- No sensitive data in logs
- Stable under expected agent usage## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.
