# Autonomous Agent Prompt: Database Migration and Schema Optimization

 
## Mission
Optimize PostgreSQL database schema and implement comprehensive migration system for production deployment.

 
## Primary Objectives
1. **Migration System**: Integrate a versioned migration manager at startup (register/validate/apply/record)
2. **Schema Optimization**: Optimize tables and indexes for vector search
3. **Connection Pooling**: Configure optimal connection management
4. **Performance Tuning**: Optimize queries and database configuration
5. **Monitoring Integration**: Add database health monitoring

 
## Current State (reference)
- A versioned migration manager exists but is not wired into startup; startup still uses `Migrations::run(...)`.
- Non-vector indexes exist on common filters; FK/partitioning/archival are not implemented.
- Pooling config exists; performance targets and measurements are not recorded.
- Migration Job command does not match a produced binary; align it to the actual `migrate` target or add a dedicated binary.

 
## Resume Instructions (verify and continue)
- Before making changes, evaluate existing Task 7 work against "Implementation Steps" and "Success Criteria".
- Re-run quality gates locally (`cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, `cargo test --all-features`).
- If tests fail, fix only failures directly related to Task 7 scope; otherwise, log the issues and proceed with outstanding Task 7 items.
- Continue only with items still outstanding; do not re-implement completed work.

 
## Implementation Steps
1. Replace ad-hoc `Migrations::run` with `DatabaseMigrationManager` pipeline (register, validate, apply, record)
2. Define migration IDs/checksums/dependencies; validate pending set; transactional apply; history recording
3. Live-DB safety: backup, staging dry-run, zero-downtime strategy, post-verification checklist
4. Align K8s migration Job `command` with actual binary produced by this repo
5. Schema: add non-vector indexes, FKs, evaluate partitioning; define archival policy and DDL
6. Performance: measure p95 latencies (<2s) and tune pool/indexes with EXPLAIN
7. Monitoring: structured logs + simple status endpoint/CLI

 
## Success Criteria
- [ ] Migration system with rollback capability
- [ ] Query performance < 2 seconds
- [ ] Optimized connection pooling
- [ ] Database monitoring operational
- [ ] Zero data loss during migrations

 
## Quality Gates and CI/CD Process

- Run static analysis after every new function is written:
  - Command: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
  - Fix all warnings before proceeding to write the next function.
- Before submission, ensure the workspace is clean:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
  - `cargo test --all-features`
- Feature branch workflow and CI gating:
  - Do all work on a feature branch (e.g., `feature/<task-id>-<short-name>`).
  - Push to the remote feature branch and monitor the GitHub Actions workflow (`.github/workflows/build-server.yml`) until it is green.
  - Require the deployment stage to complete successfully before creating a pull request.
   - Only create the PR after the workflow is green and deployment has succeeded; otherwise fix issues and re-run.

 
