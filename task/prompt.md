# Autonomous Agent Prompt: Production Deployment and Validation

## Database Access Configuration

**IMPORTANT**: The database is accessible via the `DATABASE_URL` environment variable. This variable contains the full PostgreSQL connection string to the vector database in the `databases` namespace. The database includes:

- **Connection**: `postgresql://vector_user:password@vector-postgres.databases.svc.cluster.local:5432/vector_db`
- **UUID Extension**: Already enabled (`uuid-ossp`)
- **Existing Data**: 4,375 documents already loaded across multiple documentation types
- **Tables**: `documents` (with vector embeddings), `document_sources`
- **Secrets**: Available via `doc-server-secrets` in `agent-platform` namespace

The database connection is automatically configured when the `DATABASE_URL` environment variable is set. All database operations should use this connection string.

## Mission

Execute final production deployment with comprehensive validation ensuring all acceptance criteria are met.

## Success Criteria

- [ ] Production deployment successful
- [ ] All query tools functional
- [ ] Performance benchmarks met
- [ ] Cost reduction validated
- [ ] Stakeholder acceptance complete## Quality Gates and CI/CD Process

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
