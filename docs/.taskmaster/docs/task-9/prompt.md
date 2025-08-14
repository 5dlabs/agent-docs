# Autonomous Agent Prompt: BirdEye Query Tool Implementation

## Mission
Implement `birdeye_query` tool for semantic search across BirdEye blockchain API documentation.

## Primary Objectives
1. **Tool Implementation**: Create BirdEyeQueryTool with vector search
2. **Metadata Integration**: Filter by API endpoints, methods, and categories
3. **Response Formatting**: Structure responses with source attribution
4. **MCP Integration**: Register tool in existing MCP handler system
5. **Performance Optimization**: Ensure < 2 second query response time

## Implementation Steps
1. Create BirdEyeQueryTool struct and implementation
2. Add metadata filtering for API documentation
3. Implement response formatting and relevance scoring
4. Register tool in MCP handler system
5. Add comprehensive testing and validation

## Success Criteria
- [ ] Vector similarity search functional
- [ ] Metadata filtering by API categories
- [ ] Proper MCP tool registration
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
