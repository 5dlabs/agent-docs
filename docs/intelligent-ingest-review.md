# Intelligent Ingest — Code Review Notes

This document captures issues, risks, and recommendations identified during a focused review of the intelligent ingest flow and related crates. It’s intended as a working checklist and discussion anchor.

## Summary
- Intelligent discovery is implemented in the new `discovery/` crate and exposed via HTTP in `mcp/src/ingest.rs`.
- The flow relies on Claude Code to propose an ingestion plan and CLI commands, which the server executes (sandboxed/allowlisted) using the `loader` primitives.
- Supporting pieces: universal parsing (`loader/src/parsers.rs`), DB layer (`db`), embeddings in `embed/`, and the MCP server (`mcp`).

## Security & Safety
- Arbitrary command execution: analyzer executes LLM-generated CLI commands without an allowlist.
  - Risk: destructive/malicious commands (e.g., writes outside work dir, network exfiltration).
  - Recommend: strict allowlist (only `git clone --depth 1` into a confined work dir and the `loader` binary with approved subcommands/flags). Reject everything else.
- Auto-execution via API: `IntelligentIngestRequest.yes` defaults to true; jobs run commands without explicit human confirmation.
  - Recommend: default `yes = false` and/or require privileged/internal call.
- Public binding & weak headers: HTTP server defaults to `0.0.0.0` and does not wire `security::origin_validation_middleware` or `validate_server_binding`.
  - Recommend: default bind `127.0.0.1`; enable origin/DNS-rebinding middleware; narrow CORS. Gate public binding via explicit config.
- Path and workspace hygiene: no enforced constraint that operations stay within `work_base()`; cloned repos left on disk.
  - Recommend: path sanitization, sandboxed work dir, and cleanup/GC of old repos.

## Functional Correctness
- Deprecated GitHub path incomplete (`loader/src/intelligent.rs`):
  - `get_repository_tree()` returns empty; `extract_github_repo()` builds non-raw GitHub URLs (fetches HTML instead of file content).
  - `extract_web_page()` ignores crawl depth/external links (declared TODO).
  - `analyze_local_files_with_claude()` ignores LLM ranking; currently returns all files.
  - Recommend: remove/clearly mark this path as experimental or fix raw URL usage and implement tree walking (Octocrab trees or local `git2`). Prefer unifying on the analyzer path.
- Process IO: `mcp::ingest::run_cmd()` manually reads stdout and stderr sequentially; can deadlock for chatty processes.
  - Recommend: use `wait_with_output()` or concurrent pipe reads.
- Temp workspace lifecycle: analyzer clones into a work dir but never cleans up.
  - Recommend: cleanup after successful run and periodic GC.

## Developer Experience & Config
- Claude binary expectations: errors sometimes mention `ANTHROPIC_API_KEY` though Claude Code uses a binary; inconsistent user guidance.
  - Recommend: unify error/help text around `CLAUDE_BINARY_PATH`, `CLAUDE_*` envs, and document.
- CLI independence: certain subcommands (e.g., `database`) shouldn’t require Claude presence.
  - Recommend: lazy-init LLM only for paths that need it; fail soft otherwise.
- Rate limiting: HTTP `RateLimiter` uses a 6s minimum interval; may be too slow for larger repos.
  - Recommend: make tunable per-source; consider adaptive policies.
- Command normalization: good `LOADER_BIN` handling; document `LOADER_BIN` and `INGEST_WORK_DIR` in README.

## Testing & Reliability
- Add unit tests for command allowlist and path sanitization once implemented.
- `loader/src/migration.rs` release-only test calls `tracker.get_progress().await` (method is not async); likely a latent failure if release tests run.
- Consider integration test for the analyzer path with a small public repo fixture and non-executing dry run.

## Recommendations (Order of Ops)
1) Security hardening
   - Add allowlist validation in `execute_ingestion` (restrict to safe `git` and `loader` invocations). Enforce all paths under `work_base()`.
   - Flip default to `yes = false` for intelligent ingest (both CLI and HTTP request model).
   - Default HTTP bind to `127.0.0.1`; install origin/DNS-rebinding middleware; narrow CORS.
2) Functional fixes
   - Fix raw URL usage or deprecate the older `intelligent.rs` path; prefer analyzer.
   - Replace `run_cmd` with safe `wait_with_output()`; add timeouts/logging.
   - Implement cleanup/GC for cloned repos.
3) DX & docs
   - Clarify Claude requirements and envs; ensure non-LLM subcommands work without Claude.
   - Make rate limits configurable and document `LOADER_BIN`/`INGEST_WORK_DIR`.
4) Tests
   - Add allowlist/sanitization tests; fix the release-only test; add analyzer dry-run test.

## Open Questions
- Strategy source: Are we committing to analyzer-first ingestion and deprecating `loader/src/intelligent.rs`? If not, which is canonical?
- Execution policy: Should server-side intelligent ingest ever auto-execute commands, or must it require human confirmation (esp. multi-tenant/hosted)?
- Network & credentials: How should we handle private repos/auth? Any secrets policies for Git?
- Concurrency: What are expected ingest volumes? Should `IngestJobManager` impose a concurrency limit/queue size?
- Retention: How long should cloned repos and intermediate artifacts be kept? Where?
- Auditing: Do we want to persist LLM prompts/responses for compliance and reproducibility (with careful PII/secret handling)?
- Web crawling: Should we implement site crawling (respecting robots.txt) for docs sites? Any scope constraints?
- Fallbacks: If Claude Code is unavailable, should we support an OpenAI-based analyzer variant (without shelling out)?

---
Maintainers: feel free to annotate decisions inline and turn accepted items into tracked issues.
