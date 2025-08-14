# Autonomous Agent Prompt: Database Migration and Schema Optimization

## Mission
Optimize PostgreSQL database schema and implement comprehensive migration system for production deployment.

## Primary Objectives
1. **Migration System**: Integrate a versioned migration manager at startup (register/validate/apply/record)
2. **Schema Optimization**: Optimize tables and indexes for vector search
3. **Connection Pooling**: Configure optimal connection management
4. **Performance Tuning**: Optimize queries and database configuration
5. **Monitoring Integration**: Add database health monitoring

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
- [ ] Zero data loss during migrations## Quality Gates and CI/CD Process

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

## Worktree and Parallel Branching (Required for parallel tasks)

- Use a dedicated Git worktree and feature branch for this task to avoid conflicts with other parallel tasks.

### Steps
1. Create a worktree and feature branch for this task:
```bash
git worktree add ../agent-docs-task-7 -b feature/task-7-<short-name>
```
2. Enter the worktree and do all work from there:
```bash
cd ../agent-docs-task-7
```
3. Develop in this isolated directory, follow Quality Gates (run clippy pedantic after each new function; fmt/clippy/tests before pushing).
4. Push and monitor GitHub Actions; only create the PR after CI is green and deployment succeeds.
5. When finished:
```bash
git worktree list
git worktree remove ../agent-docs-task-7
```

### Cleanup after push (PVC)
- After pushing and creating the PR, remove the worktree to free PVC storage:
```bash
git worktree remove ../agent-docs-task-7
git worktree prune
```
- After merge, optionally delete the remote/local feature branch:
```bash
git push origin --delete feature/task-7-<short-name> || true
git branch -D feature/task-7-<short-name> || true
```
