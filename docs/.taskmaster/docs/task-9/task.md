# Task 9: BirdEye Query Tool Implementation

## Overview
Implement `birdeye_query` tool for semantic search across BirdEye API documentation with metadata filtering and response formatting.

## Implementation Guide
- Create BirdEyeQueryTool with semantic search capabilities
- Implement metadata filtering for API endpoints and methods
- Add response formatting with source attribution
- Integrate with existing MCP tool registration system
- Optimize query performance for API documentation

## Technical Requirements
- Vector similarity search using OpenAI embeddings
- JSONB metadata filtering for API categories
- Tool registration in MCP handler
- Response formatting with relevance scoring
- Query performance optimization

## Success Metrics
- Query response time < 2 seconds
- Accurate search results with metadata filtering
- Proper tool registration and MCP integration
- Source attribution in all responses
- High relevance scoring for API documentation## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.
