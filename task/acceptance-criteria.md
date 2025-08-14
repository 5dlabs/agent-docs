# Acceptance Criteria: Database Migration and Schema Optimization

## Functional Requirements

### FR-1: Migration System (Extend Existing)
- [ ] Extend and validate the existing version-controlled migration system (no reinvention)
- [ ] Forward migration execution
- [ ] Rollback plan documented; prefer roll-forward remediation
- [ ] Migration history tracking intact (history table present in live DB)
- [ ] Atomic migration operations where feasible

### FR-2: Schema Optimization
- [ ] Optimized indexes for current query paths (metadata filters, non-vector indexes)
- [ ] Proper foreign key constraints (e.g., `documents` → `document_sources`)
- [ ] Optimized data types for storage efficiency
- [ ] Partitioning strategy for large datasets
- [ ] Archive strategy for old data

### FR-3: Performance Optimization
- [ ] Query execution time < 2 seconds
- [ ] Optimized connection pooling configuration
- [ ] Database configuration tuning
- [ ] Index usage optimization
- [ ] Memory usage optimization

## Test Cases

### TC-1: Migration Execution (Live DB with Safeguards)
**Given**: New migration available
**And**: Pre-migration backup and staging dry-run complete
**When**: Migration executed against live DB using zero-downtime strategy
**Then**: Schema updated successfully
**And**: Migration recorded in history
**And**: No data loss occurs
**And**: Post-migration verification passes (integrity, performance smoke tests)

### TC-1b: Live DB Snapshot Validation (Read-only)
**Given**: Connection to live `DATABASE_URL`
**When**: Running read-only audit queries
**Then**: Verify presence of history table; verify FK existence; verify indexes; verify no vector index on `embedding(3072)`; record row counts

### TC-2: Performance Validation
**Given**: Optimized database configuration
**When**: Query performance tested
**Then**: Response time < 2 seconds
**And**: Connection pool efficiency validated
**And**: Resource usage within limits

## Deliverables
- [ ] Validated extensions to the existing migration framework
 - [ ] `k8s/migration-job.yaml` updated to point to the actual migration binary built by this repo
## Live Database Requirements
- [ ] Use `DATABASE_URL` for the live (production) cluster
- [ ] Backup taken and verified prior to migration
- [ ] Staging dry-run completed using fresh prod snapshot
- [ ] Zero-downtime plan documented and followed
- [ ] Post-migration verification checklist completed
- [ ] Optimized database schema
- [ ] Performance tuning configuration
- [ ] Monitoring integration
- [ ] Migration documentation

### NFR-0: Code Quality and Automation
- [ ] After adding any new function, run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and fix all warnings before continuing
- [ ] Prior to submission, ensure `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` all pass locally
- [ ] All changes pushed to a feature branch; GitHub Actions must complete successfully (including deployment) before opening a PR
- [ ] If a PR already exists, fix all formatting and linter errors locally and push updates to the SAME PR branch; do not open a new PR
- [ ] PR creation/update is gated on a green CI pipeline and successful deployment of the server artifact
