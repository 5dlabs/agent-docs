# Task 8: OpenAI Embedding Client with Batch Processing

## Overview
Implement optimized OpenAI embedding generation using text-embedding-3-large model with batch processing targeting ~70% cost reduction per assessment.

## Implementation Guide
- Implement OpenAI Batch API integration with JSONL format
- Configure text-embedding-3-large with 3072 or 1024 dimensions
- Create batch queue system for 20,000 line chunks
- Implement rate limiting (3000 RPM / 1M TPM) with token bucket
- Add retry logic with exponential backoff
- Implement cost tracking and reporting system

## Technical Requirements
- OpenAI Batch API integration
- JSONL format batch processing
- Rate limiting with token bucket algorithm
- Exponential backoff retry mechanism
- Cost tracking and 50% reduction validation
- Secure API key management

## Success Metrics
- ~70% cost reduction through batch processing (baseline vs. batched)
- Rate limiting compliance (3000 RPM / 1M TPM)
- Processing time < 20 minutes for 20k embeddings
- Retry mechanism handles failures gracefully
- Cost tracking accurate and comprehensive## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.
