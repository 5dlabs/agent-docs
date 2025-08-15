#!/usr/bin/env bash
set -euo pipefail

# install_git_hooks.sh
# Installs a pre-push hook that enforces Rust formatting locally.

ROOT_DIR=$(git rev-parse --show-toplevel 2>/dev/null || true)
if [[ -z "${ROOT_DIR}" ]]; then
  echo "Error: not inside a Git repository" >&2
  exit 1
fi

HOOKS_DIR="${ROOT_DIR}/.git/hooks"
mkdir -p "${HOOKS_DIR}"

cat > "${HOOKS_DIR}/pre-push" << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

# Pre-push hook: enforce cargo fmt before pushing

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found; skipping format check" >&2
  exit 0
fi

echo "[pre-push] Running cargo fmt --all -- --check"
if ! cargo fmt --all -- --check; then
  echo "[pre-push] Formatting needed. Applying cargo fmt..." >&2
  cargo fmt --all
  echo "[pre-push] Formatting applied. Please add/commit the changes and push again." >&2
  exit 1
fi

exit 0
EOF

chmod +x "${HOOKS_DIR}/pre-push"

echo "Installed pre-push hook at ${HOOKS_DIR}/pre-push"


