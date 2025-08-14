# Autonomous Agent Prompt: Kubernetes Deployment Configuration

## Mission
Create and optimize Helm charts for production Kubernetes deployment with proper resource allocation and security policies.

## Success Criteria
- [ ] Helm charts created with configurable parameters
- [ ] Resource limits properly configured
- [ ] Security policies implemented
- [ ] High availability configuration complete

## Deployment Validation (Mandatory 4-step)
1. Push branch to GitHub to trigger CI
2. CI builds container image and runs clippy/tests
3. Deploy via Helm using chart values (ensure `values.yaml` exists)
4. Perform real-world validation with a compliant MCP client## Quality Gates and CI/CD Process

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
git worktree add ../agent-docs-task-13 -b feature/task-13-<short-name>
```
2. Enter the worktree and do all work from there:
```bash
cd ../agent-docs-task-13
```
3. Develop in this isolated directory, follow Quality Gates (run clippy pedantic after each new function; fmt/clippy/tests before pushing).
4. Push and monitor GitHub Actions; only create the PR after CI is green and deployment succeeds.
5. When finished:
```bash
git worktree list
git worktree remove ../agent-docs-task-13
```
