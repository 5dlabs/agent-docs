# LLM Roles and Separation of Concerns

This project uses two different LLM providers with a strict separation of duties.

- Claude Code (local binary)
  - Scope: Intelligent document ingestion and discovery only.
  - Responsibilities:
    - Analyze repositories to determine relevant documentation.
    - Propose ingestion strategies (paths, extensions, chunking hints).
    - Generate the CLI commands executed by the intelligent ingestion flow.
  - Not used for: embeddings, search, general API responses, or any runtime query handling.
  - Configuration:
    - `CLAUDE_BINARY_PATH` must point to a working Claude CLI binary (or `claude` present on `PATH`).
    - `CLAUDE_TIMEOUT_SECS` controls the process timeout when reading the Claude binary output.
  - Failure policy:
    - Intelligent analysis fails fast if Claude is unavailable or times out. There is no fallback to OpenAI.

- OpenAI (HTTP API)
  - Scope: Embedding generation and batch/vector operations only.
  - Responsibilities:
    - Generate embeddings for documents (e.g., `text-embedding-3-large`).
    - Support batch insertion and similarity/search flows via the `embed/` and `db/` crates.
  - Not used for: repository analysis, document discovery, or strategy generation.
  - Configuration:
    - `OPENAI_API_KEY` must be set for the embedding client.

## Operational Notes

- Intelligent ingestion ("analyze and then execute") requires Claude and will error if the Claude binary is not available.
- Subsequent ingestion steps may write JSON files and load them into the database; these steps rely on embeddings (OpenAI) and database connectivity but do not use Claude.
- Log expectations:
  - You should not see any log lines about "falling back to OpenAI" during intelligent analysis. If Claude fails, the analysis returns an error.
  - Embedding-related logs originate from the `embed/` crate and use OpenAI exclusively.

## Environment Summary

- Required for intelligent analysis: `CLAUDE_BINARY_PATH` (or `claude` on `PATH`), optional `CLAUDE_TIMEOUT_SECS`.
- Required for embeddings/database ingestion: `OPENAI_API_KEY`, `DATABASE_URL`.

