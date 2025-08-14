#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is not set" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Running read-only DB audit against DATABASE_URL"
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$ROOT_DIR/sql/audit.sql"


