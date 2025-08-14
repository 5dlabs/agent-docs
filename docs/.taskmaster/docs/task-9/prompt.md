# Autonomous Agent Prompt: Config-Driven Documentation Query Tools (Dynamic Registration)

## Mission
Implement dynamic tool registration from a JSON config. Each configured tool maps to a `docType` and shares one unified query method. Keep Rust docs tools hardcoded; all other categories (e.g., birdeye, solana, cilium) are defined in config.

## Primary Objectives
1. **Config Loader**: Read and validate JSON config (tools: name, docType, title, description, enabled)
2. **Dynamic Registration**: Create tools at startup for each enabled config entry
3. **Unified Query**: Route all tool calls to a shared handler filtering by `docType`
4. **Response Formatting**: Include source attribution and relevance
5. **Performance**: Ensure < 2 second query response time

## Implementation Steps
1. Define config schema and place example config file
2. Implement config reader and validation
3. Register tools dynamically in MCP server startup
4. Implement unified query handler that accepts `docType`
5. Add tests for registration and query routing

## Success Criteria
- [ ] Tools loaded from config and listed in `tools/list`
- [ ] Unified query returns results filtered by `docType`
- [ ] Response time < 2 seconds
- [ ] Source attribution in responses## Quality Gates and CI/CD Process

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

- Use Git worktrees to isolate this task's working directory and feature branch to avoid conflicts with other tasks running in parallel.

### Steps
1. Create a dedicated worktree and feature branch for this task:

2. Enter the worktree and do all work from there:

3. Run your development session here (e.g., Claude Code) and follow the Quality Gates section (Clippy pedantic after each new function; fmt/clippy/tests before pushing).

4. Push from this worktree and monitor GitHub Actions; create a PR only after CI is green and deployment succeeds.

5. Manage worktrees when finished:
/Users/jonathonfritz/code/work-projects/5dlabs/agent-docs  610a801 [main]
