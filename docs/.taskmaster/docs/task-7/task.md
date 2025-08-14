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

## Current Status vs Outstanding (from gaps report)

- [x] Versioned migration manager exists (`crates/database/src/migration_system.rs`)
- [ ] Manager integrated at startup (replace `Migrations::run(..)` with manager pipeline)
- [x] Non-vector indexes present for common filters (e.g., `doc_type`, `source_name`)
- [ ] Foreign keys between `documents` and `document_sources`
- [ ] Partitioning strategy (e.g., time-based on `created_at`)
- [ ] Archival policy and DDL for aged data
- [~] Pooling support exists; [ ] tune/validate perf (< 2s) with measurements
- [ ] Live-DB safeguards: backup, staging dry-run, zero-downtime playbook, post-verification
- [ ] Migration Job alignment: update `k8s/migration-job.yaml` to call the actual built binary
- [ ] Observability: structured logs + simple status (endpoint or CLI)

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

## Detailed Requirements to Meet Acceptance Criteria

1) Versioned Migration Manager Integration
- Replace ad-hoc startup call `Migrations::run` with `DatabaseMigrationManager` pipeline:
  - Register migrations with IDs, checksums, dependencies.
  - On startup: validate schema state, compute pending set, apply with transactions, record history.
  - Expose status and history endpoints or CLI to view progress.

2) Safety Playbook (Live DB)
- Pre-flight tasks: snapshot/backup, staging dry-run from fresh prod snapshot; document commands and storage locations.
- Execution strategy: additive schema changes, backfill tables/columns in batches, optional dual-write if needed; clear rollback plan (prefer roll-forward).
- Post-flight: integrity checks, query smoke tests, and performance validation; roll-forward remediation guide.

3) Schema Optimization
- Non-vector indexes: confirm/selectivity-driven indexes on `documents(doc_type, source_name)`, and filters used by hottest queries; add partial indexes as needed.
- Foreign keys: define FK from `documents.source_name` to `document_sources.source_name` (or surrogate IDs) where applicable.
- Partitioning: evaluate time-based partitioning on `documents.created_at` for large datasets; document strategy and implement DDL if warranted.
- Archival: define archival policy for old `documents`; create archival table and migration for moving aged data.

4) Performance Objectives
- Measure/record key query latencies and pool utilization before/after migration; target < 2s for read paths.
- Tune pool (`min/max`, lifetime, idle), statement timeouts, and add missing indexes informed by `EXPLAIN ANALYZE`.

5) Tooling Alignment
- Update `k8s/migration-job.yaml` to invoke the correct migration binary:
  - If using `crates/doc-loader/src/bin/migrate.rs`, build/publish image with `migrate` binary path and change the Job `command` accordingly.
  - Or add a dedicated `doc-server-migrate` binary target and ensure the Job points to it.

6) Observability
- Emit structured logs during each migration step; summarize applied count, timing, and failures.
- Optionally add a simple `migration_status` HTTP endpoint or CLI subcommand to query migration state/history.

