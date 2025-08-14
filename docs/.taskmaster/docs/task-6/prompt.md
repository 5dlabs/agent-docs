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
