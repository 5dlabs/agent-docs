# Autonomous Agent Prompt: Dynamic Tooling Extensions (Config-Driven) – Task 10

## Mission

Generalize and extend the config-driven tool system from Task 9. Do not create hardcoded per-domain tools (e.g., Solana). Instead:
- Ensure the JSON config can declare additional tools (name, docType, title, description, enabled).
- Enhance the unified query handler to support optional metadata filters and adaptive formatting across all configured docTypes.
- Keep Rust docs tooling hardcoded; all other categories are config-driven.

## Execution Steps

1) Config and Loader
- Validate schema supports fields: name, docType, title, description, enabled, and optional metadata hints (e.g., supported formats, topics).
- Add/adjust example config entries to demonstrate multiple docTypes (e.g., solana, birdeye, cilium).

2) Unified Query Enhancements
- Accept optional arguments in the shared handler: `limit` (1–20), and generic filters such as `format`, `complexity`, `category`, `topic`, `api_version` when present in metadata.
- Filter results by `docType` and apply provided filters server-side.
- Maintain < 2s response target.

3) Adaptive Response Formatting
- If metadata indicates diagrams (bob/msc), preserve ASCII-block formatting.
- If metadata indicates PDFs, output a concise metadata summary (location, size, description, page count, when available).
- For markdown, present clean snippets with headers/context.
- Always include source attribution and relevance score when available.

4) Registration and Tests
- Register dynamic tools solely from the JSON config (no new hardcoded tools).
- Add tests to verify: tools/list includes configured tools; tools/call routes to the unified handler; metadata filters and formatting behave as expected.

## Success Criteria
- Tools appear via `tools/list` based on the config.
- `tools/call` returns results filtered by `docType` and optional metadata filters.
- Responses include source attribution and relevance; formatting adapts to content type.
- P50/P95 response times remain under 2 seconds in test runs.

## Quality Gates and CI/CD Process

- After every new function: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and fix all warnings.
- Before submission (blocking):
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
  - `cargo test --all-features`
- Branch & CI: Work on a feature branch, push, and require a green GitHub Actions workflow (including deployment) before opening a PR.
- Pre-push: Run `cargo fmt --all` locally (or ensure the pre-push hook is installed and active) BEFORE pushing any commits.

## Implementation Notes
- Reuse `DocumentQueries` and existing DB abstractions.
- Do not add separate structs per docType; use the unified handler and config entries only (Rust docs tool remains hardcoded).
- Ensure parameter validation returns helpful errors; keep 1–20 bound for `limit`.
- Add structured logs for filter usage and result counts.
