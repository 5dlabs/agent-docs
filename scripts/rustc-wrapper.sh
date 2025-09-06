#!/usr/bin/env bash
set -euo pipefail

# Cargo's RUSTC_WRAPPER contract: this script must behave like rustc.
# It receives the same args as rustc would; the real compiler path is in $RUSTC.

# Determine the real rustc binary and normalize args according to Cargo's
# wrapper invocation. Cargo typically calls: wrapper <path-to-rustc> <args...>.
# Some tools may set $RUSTC and omit the first arg. Handle both cases.

REAL_RUSTC="${RUSTC:-}"
if [[ $# -gt 0 && -z "$REAL_RUSTC" && -x "$1" ]]; then
  # First arg looks like a rustc path; use it and drop it from args
  REAL_RUSTC="$1"
  shift
fi

if [[ -z "$REAL_RUSTC" ]]; then
  # If first arg is literally 'rustc', drop it as the rustc path
  if [[ $# -gt 0 && "$1" == "rustc" ]]; then
    shift
  fi
  REAL_RUSTC="rustc"
fi

# Prefer sccache if available (unless explicitly disabled)
if command -v sccache >/dev/null 2>&1 && [[ "${DISABLE_SCCACHE:-0}" != "1" ]]; then
  exec sccache "$REAL_RUSTC" "$@"
fi

# Fallback to direct rustc invocation
exec "$REAL_RUSTC" "$@"
