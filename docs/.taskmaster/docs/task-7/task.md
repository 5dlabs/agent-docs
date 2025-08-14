# Task 7: Database Migration and Schema Optimization (Extend Existing, Run on Live DB Safely)

## Overview
Optimize PostgreSQL database schema and implement migration system for production deployment with performance improvements.

## Implementation Guide
- Extend and validate the existing migration system with version tracking (do not recreate)
- Optimize schema for real-world query paths (metadata filters and non-vector indexes given pgvector dimension limits)
- Add appropriate indexing strategies for document retrieval
- Implement connection pooling optimization
- Add database health monitoring and metrics
- Document pgvector 2000-dimension index limitation; plan for either dimension reduction or future pgvector upgrade to enable vector index

## Live DB Execution Policy (Required)
- Use the live database via `DATABASE_URL` (production cluster) with the following safeguards:
  - Pre-migration backup: take and verify a restorable snapshot
  - Dry-run: validate migrations in staging against a fresh copy of production schema/data
  - Zero-downtime strategy: use online migration patterns (additive changes, backfill, dual-write if needed)
  - Maintenance window: schedule any potentially blocking operations
  - Roll-forward first: prefer forward fixes over rollbacks; keep rollback plans documented for emergencies
  - Post-migration verification: integrity checks, performance smoke tests, and monitoring

## Technical Requirements
- PostgreSQL with pgvector extension
- Migration versioning and rollback capability
- Optimized indexes for query performance
- Connection pool configuration
- Database monitoring integration

## Notes from Assessment
- Current embeddings are 3072-dim (OpenAI text-embedding-3-large); vector index cannot be created
- Queries must rely on metadata filters + full scan similarity; performance workarounds required

## Success Metrics
- Query performance improvements (< 2s response time)
- Successful schema migrations without data loss
- Optimized connection pooling for concurrent access
- Database health monitoring operational
- Backup and recovery procedures validated## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.
