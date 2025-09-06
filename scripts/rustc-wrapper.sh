#!/usr/bin/env bash
set -euo pipefail

# Cargo's RUSTC_WRAPPER contract: this script must behave like rustc.
# It receives the same args as rustc would; the real compiler path is in $RUSTC.

REAL_RUSTC="${RUSTC:-rustc}"

# Prefer sccache if available (unless explicitly disabled)
if command -v sccache >/dev/null 2>&1 && [[ "${DISABLE_SCCACHE:-0}" != "1" ]]; then
  exec sccache "$REAL_RUSTC" "$@"
fi

# Fallback to direct rustc invocation
if [[ -x "$REAL_RUSTC" ]]; then
  exec "$REAL_RUSTC" "$@"
else
  exec rustc "$@"
fi
