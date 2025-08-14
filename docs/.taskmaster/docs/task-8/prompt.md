# Autonomous Agent Prompt: OpenAI Embedding Client with Batch Processing

## Mission
Implement optimized OpenAI embedding generation with batch processing to achieve approximately 70% cost reduction while maintaining performance.

## Live Environment Requirement (Mandatory)
- Test end-to-end against the live database (`DATABASE_URL`) and the real OpenAI Batch API using `OPENAI_API_KEY`.
- Do not use mocks or stubs for API calls or database operations; collect real batch/job IDs and timings.

## Primary Objectives
1. **Batch API Integration**: Implement OpenAI Batch API with JSONL format
2. **Rate Limiting**: Token bucket algorithm for API compliance
3. **Queue Management**: Batch processing system for large datasets
4. **Error Handling**: Retry logic with exponential backoff
5. **Cost Optimization**: Track and validate 50% cost reduction

## Implementation Steps
1. Create OpenAI Batch API models and data structures
2. Implement rate limiting with token bucket algorithm
3. Build batch processing queue system
4. Add comprehensive retry logic and error handling
5. Implement cost tracking and reporting

## Success Criteria
- [ ] Batch API integration with JSONL format
- [ ] Rate limiting compliance (3000 RPM / 1M TPM)
- [ ] ~70% cost reduction achieved and validated
- [ ] Processing time < 20 minutes for 20k embeddings
- [ ] Robust error handling and retry mechanisms## Quality Gates and CI/CD Process

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
