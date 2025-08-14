# Autonomous Agent Prompt: Streamable HTTP Response Semantics, Keep-Alive, and Error Handling

## Mission (MVP)
Align response behavior with MCP 2025-06-18 Streamable HTTP for JSON-only POST: ensure headers and JSON-RPC error handling with proper HTTP mapping. Streaming is out of scope for MVP.

## Primary Objectives
1. Ensure `/mcp` POST (JSON) behavior is correct; GET returns 405
2. Include `Mcp-Session-Id` and `MCP-Protocol-Version` in responses
3. Map transport and application errors to correct HTTP status codes
4. Add structured logging

## Implementation Steps
1. Ensure headers are set on all responses (session id, protocol version)
2. Implement JSON-RPC error formatting and HTTP mapping
3. Add logs for request id/session id/version

## Success Criteria
- [ ] Correct headers present on all responses
- [ ] Errors return correct HTTP status and JSON-RPC body
- [ ] Logs capture request/session/version

## Deployment Validation (Mandatory 4-step)
1. Push → CI build/tests
2. Deploy via Helm
3. Live test with MCP client (headers/errors)
4. Record validation results in task artifacts## Quality Gates and CI/CD Process

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
