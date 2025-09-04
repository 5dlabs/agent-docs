#!/usr/bin/env bash
set -euo pipefail

# Run the same checks as CI locally
# Usage: scripts/ci-local.sh [DATABASE_URL]

ROOT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

export RUST_LOG=debug
export RUST_BACKTRACE=1

# Optionally take a DB URL arg to mimic CI; otherwise use mock
if [[ $# -gt 0 ]]; then
  export DATABASE_URL="$1"
  export TEST_DATABASE_URL="$1"
else
  export TEST_DATABASE_URL=mock
  unset DATABASE_URL || true
fi

echo "🔧 Formatting check"
cargo fmt --all -- --check || (echo "Applying fmt" && cargo fmt --all)

echo "🧹 Clippy pedantic"
cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic

echo "🧪 Running tests (workspace, all-targets, all-features)"
cargo test --workspace --all-features --all-targets -- --nocapture

echo "✅ ci-local checks completed"

