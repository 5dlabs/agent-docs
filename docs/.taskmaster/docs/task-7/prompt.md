# Autonomous Agent Prompt: Database Migration and Schema Optimization

## Mission
Optimize PostgreSQL database schema and implement comprehensive migration system for production deployment.

## Primary Objectives
1. **Migration System**: Create versioned database migration framework
2. **Schema Optimization**: Optimize tables and indexes for vector search
3. **Connection Pooling**: Configure optimal connection management
4. **Performance Tuning**: Optimize queries and database configuration
5. **Monitoring Integration**: Add database health monitoring

## Implementation Steps
1. Create migration framework with version control
2. Analyze and optimize existing schema
3. Add performance indexes and constraints  
4. Configure connection pooling
5. Implement database monitoring

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
