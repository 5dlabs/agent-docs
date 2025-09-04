# Repository Guidelines

## Project Structure & Module Organization
- `mcp/`: HTTP server, tools, protocol handlers. Binaries in `src/bin/` (e.g., `http_server.rs`).
- `llm/`: embeddings, search, model clients.
- `loader/`: ingestion, parsers, migrations (`src/bin/migrate.rs`).
- `db/`: database layer and integration tests. SQL in `sql/` and `db/src`.
- `embed/`: client and batch helpers.
- `docs/`: deployment, ops, and Helm charts. `scripts/`: test/bench utilities.

## Build, Test, and Development Commands
- Build workspace: `cargo build --workspace`.
- Run server: `cargo run -p mcp --bin http_server`.
- Run migration tool: `cargo run -p loader --bin migrate`.
- Unit/integration tests: `cargo test --workspace` (DB tests skip without a DB).
- DB tests locally: `TEST_DATABASE_URL=postgres://... cargo test -p db -- --nocapture`.
- Lint strictly: `cargo clippy --workspace --all-targets --all-features -D warnings -W clippy::pedantic`.
- Format: `cargo fmt --all` (config in `rustfmt.toml`).
- Acceptance tests: `BASE_URL=http://localhost:3001 ./scripts/acceptance-tests.sh`.
- Pre-push check: `./validate-before-commit.sh` (runs clippy, tests, DB-skip checks).
- Docker: `docker build -t agent-docs .`.

## Coding Style & Naming Conventions
- Rust style via `rustfmt`; 4-space indentation.
- Use `snake_case` for files/functions, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for consts.
- Keep modules small and cohesive; prefer crate-local `mod tests` for unit tests.

## Testing Guidelines
- Framework: Rust `cargo test`. Place integration tests in `<crate>/tests/` and unit tests next to code.
- Database tests: skip gracefully when no `TEST_DATABASE_URL`; provide setup via `scripts/setup_test_db.sql` if needed.
- Name tests descriptively (what_is_expected_when_condition).

## Commit & Pull Request Guidelines
- Use Conventional Commits: `feat:`, `fix:`, `refactor:`, `test:`, `chore:`, `style:`, `config:` (see `git log`).
- PRs: include scope/impact, linked issues, and how to test (commands/URLs). Run `./validate-before-commit.sh` before pushing.

## Security & Configuration Tips
- Store secrets in env/secret stores; never commit credentials. See `SECURITY.md`.
- Local config: `.env` for development; ensure production values are provided via deployment manifests in `docs/`.

