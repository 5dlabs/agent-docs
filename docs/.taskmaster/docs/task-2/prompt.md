# Autonomous Agent Remediation Prompt: Task 2 — Data Migration and Validation Pipeline (Implemented; Fix Build and Data Migration)

This task has been implemented via [PR #5](https://github.com/5dlabs/agent-docs/pull/5), which adds a comprehensive data migration and validation pipeline. Your mission now is to remediate review findings, fix the build, and execute the data migration locally. Kubernetes changes are not required at this time.

## Context from PR #5

- Implementation summary: extended `crates/doc-loader` with a full migration framework, parallel workers, checkpointing, validation levels, CLI (`migrate`) commands, and a production `k8s/migration-job.yaml`.
- Reviewer comments to address:
  - Unit tests: `cargo test --package doc-loader migration_tests` failed due to package ID mismatch.
  - Performance tests: `cargo test --release migration_performance` failed to compile; errors in `crates/doc-loader/src/migration.rs` (EmbeddingClient not recognized as a trait; pgvector unresolved).
  - Kubernetes validation: `kubectl apply -f k8s/migration-job.yaml --dry-run=client` succeeded.

Use these comments to drive fixes so the workspace builds locally and CI is green.

## Required Remediation Steps

1. Resolve compilation errors in `crates/doc-loader/src/migration.rs`:
   - Ensure the embeddings client implements/uses the correct trait(s) exported by `crates/embeddings` (e.g., `EmbeddingClient` or equivalent) and is imported with proper crate path.
   - Fix pgvector usage: add/import the `pgvector` crate and enable required features in relevant crates (`database`, `doc-loader`), and confirm model types align with `sqlx`/`postgres` integration.
2. Fix test targeting and package IDs:
   - Verify `Cargo.toml` package names/paths for `doc-loader` match test invocations; adjust tests or package names so `cargo test --package doc-loader migration_tests` resolves.
   - Ensure performance tests compile by gating long-running tests behind `--release` or feature flags as appropriate.
3. Ensure database schema and migrations are wired for tests:
   - Provide test fixtures or a lightweight schema setup for unit/integration tests.
   - Confirm vector column dimensions (3072) and types match embedding model outputs.
4. Harden CLI and rollback flows:
   - Validate resume-from-checkpoint and batch-atomic rollback with simulated failures.
   - Ensure progress/ETA reporting is thread-safe and low overhead.

## Validation Requirements (local and CI)

- Local compile and tests:
  ```bash
  cargo build --workspace
  cargo test --package doc-loader migration_tests
  cargo test --release migration_performance
  ```

- GitHub CI: push changes and verify all required checks pass for `main`.

- Local data migration run (dev database):
  ```bash
  # dry run validation
  cargo run --bin migrate -- --validate --dry-run

  # full migration with reasonable parallelism
  cargo run --bin migrate -- --full-migration --parallel 8
  ```

## Definition of Done

- All compilation errors resolved across workspace.
- Unit/performance tests pass locally and in GitHub CI (green checks on `main`).
- Local migration runs successfully (dry-run and full migration) with validated outcomes:
  - ≥1000 docs/minute achievable with configured workers
  - Checksum and schema validations pass (no unresolved duplicates)
  - Rollback and resume tested and reliable

## Notes

- Keep changes minimal and focused on making PR #5 production-ready.
- If you introduce new dependencies or feature flags, update relevant `Cargo.toml` and documentation.
- Record wall-clock throughput and confirm vector dimensionality (3072) consistency.