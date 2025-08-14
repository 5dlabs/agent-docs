# Task 9: Config-Driven Documentation Query Tools (Dynamic Registration)

## Overview

Implement dynamic registration of documentation query tools from a JSON config. Expose multiple tool names (one per doc category) that all use a single unified query implementation. Keep Rust docs tools hardcoded (due to add-crate flow); all other categories are config-driven.

## Implementation Guide

- Define a JSON config shape for tool definitions (name, docType, title, description, enabled)
- Load and validate config at server startup
- Dynamically register tools in the MCP server based on the config
- Route all tool calls to a unified query handler that filters by `docType`
- Provide response formatting with source attribution and relevance scoring

## Technical Requirements

- JSON config (committed) for tool definitions (excluding Rust docs)
- Unified query handler with pgvector similarity search
- JSONB metadata filtering when available per category (e.g., API endpoints)
- Dynamic tool registration in MCP handler
- Response formatting with relevance scores

## Parameter Constraints and Errors

- `query`: non-empty UTF-8 string, trimmed length 1–512
- `limit`: integer 1–20 (default 10)
- Invalid parameters must produce JSON-RPC error objects (code -32602) returned via JSON-only Streamable HTTP (MCP 2025-06-18)

## Success Metrics

- Tools appear via `tools/list` and invoke successfully
- Query response time < 2 seconds for typical inputs
- Accurate results filtered by `docType`
- Source attribution in responses
- Dynamic registration from config confirmed

## MCP Transport Policy

- Streamable HTTP (2025-06-18), JSON-only mode, GET returns 405; no SSE for MVP. JSON-RPC 2.0 error formatting for invalid requests.

## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded. If a PR already exists, push updates to the same PR branch (do not create a new PR).
