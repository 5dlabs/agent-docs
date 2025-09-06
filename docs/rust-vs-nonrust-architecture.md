# Source-Type Architecture: Rust Crates vs Everything Else

This document tracks the refactor to clearly separate ingestion by source type.

## Goals
- Clear separation of concerns and code paths:
  - Rust crates: predictable flow, no intelligent discovery required
  - Everything else: Claude Code discovery → plan → ingest primitives
- Safer execution: discovery produces plans; ingest executes whitelisted steps
- Easier mental model for operators and contributors

## Target Layout
- `discovery/`: Claude Code–based discovery
  - Produces a normalized ingest plan (e.g., `git clone`, `loader cli`, `loader database`)
  - No DB writes; no file parsing

- `rust_crates/`: Ingestion helpers for Rust crates
  - Fetch crate metadata (crates.io), docs (docs.rs or API), emit DocPages
  - Called by MCP Rust tools to insert docs into DB

- `loader/` (aka ingest-docs): Execution primitives used by plans
  - `cli`: parse a filesystem path → DocPage JSON (UniversalParser)
  - `database`: load DocPage JSON into PostgreSQL
  - No Claude or discovery logic

- `embed/`: Embeddings-only (OpenAI)
  - Used by migration/backfill jobs to generate vectors

- `mcp/`: HTTP server and tools
  - Non-Rust path: call `discovery` → execute plan with allowlist → `loader`
  - Rust path: call `rust_crates` directly (no discovery)

## Current Status
- `discovery/` crate added and wired into MCP intelligent ingest
- Plan execution in MCP with strict allowlist (git clone --depth, loader cli, loader database)
- `loader` trimmed to `cli` + `database` only; legacy analyzer removed; `local` renamed to `cli`
- `rust_crates/` crate added; MCP Rust tools now use it instead of `loader`
- `llm/` removed from workspace and code references

## Next Steps (Optional)
- Add “embedding backfill” job for docs missing vectors
- Rename `loader/` → `ingest_docs/` for clarity (docs + code)
- Expand `rust_crates` beyond stub: docs.rs traversal, richer page extraction
- Harden allowlist further (arg validation), add sandbox path enforcement

## Open Questions
- Do we want a discovery-only endpoint to review plans before execution?
- What retention policy for cloned repos and JSON outputs?
- Should embeddings be inline for Rust crates or remain a separate job?
