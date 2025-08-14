# Task 10: Dynamic Tooling Extensions (Config-Driven)

## Overview
Generalize Task 9’s config-driven tool system: extend the JSON config and unified handler so new documentation categories (e.g., solana, birdeye, cilium) are added via config entries, not hardcoded tools. Rust docs remain the only hardcoded tool.

## Implementation Guide
- Extend config schema and loader (name, docType, title, description, enabled, optional metadata hints).
- Enhance unified query handler to accept optional generic filters and adapt output formatting based on metadata.
- Register tools strictly from config at startup; no new per-domain structs.
- Add tests for registration, routing, filters, and formatting.

## Technical Requirements
- Vector similarity or ranked search; include relevance score.
- JSONB metadata filters: `format`, `complexity`, `category`, `topic`, `api_version` (when present).
- Adaptive formatting for diagrams (bob/msc), PDFs (summary), markdown.
- Performance target: < 2 seconds per query; use pooling and efficient queries.

## Success Metrics
- Tools appear via `tools/list` solely from config.
- Queries filtered by `docType` and optional metadata filters; relevance and attribution included.
- p95 response < 2s across representative runs.## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.
