#!/usr/bin/env bash
set -euo pipefail

# Minimal rustc wrapper: just forwards to the real rustc binary.
# Cargo invokes the wrapper as: wrapper <path-to-rustc> <args...>

if [[ $# -lt 1 ]]; then
  echo "rustc-wrapper: missing rustc path" >&2
  exit 1
fi

RUSTC_BIN="$1"
shift

# If sccache is available and allowed, use it transparently
if command -v sccache >/dev/null 2>&1 && [[ "${DISABLE_SCCACHE:-0}" != "1" ]]; then
  exec sccache "$RUSTC_BIN" "$@"
fi

# Otherwise, invoke rustc directly
if [[ -x "$RUSTC_BIN" ]]; then
  exec "$RUSTC_BIN" "$@"
else
  exec rustc "$@"
fi
