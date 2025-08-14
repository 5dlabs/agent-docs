#!/usr/bin/env bash
set -euo pipefail

# add_quality_gate.sh
# Batch-append quality gate and CI/CD requirements to all Taskmaster task docs.
#
# Usage:
#   scripts/add_quality_gate.sh [tasks_root]
#
# Default tasks_root: docs/.taskmaster/docs

ROOT_DIR="${1:-docs/.taskmaster/docs}"

if [[ ! -d "$ROOT_DIR" ]]; then
  echo "Error: tasks root not found: $ROOT_DIR" >&2
  exit 1
fi

# Blocks to append (generic example branch names to avoid task-specific confusion)
read -r -d '' BLOCK_AC << 'EOF' || true

### NFR-0: Code Quality and Automation
- [ ] After adding any new function, run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and fix all warnings before continuing
- [ ] Prior to submission, ensure `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` all pass locally
- [ ] All changes pushed to a feature branch; GitHub Actions must complete successfully (including deployment) before opening a PR
- [ ] PR creation is gated on a green CI pipeline and successful deployment of the server artifact
EOF

read -r -d '' BLOCK_PROMPT << 'EOF' || true

## Quality Gates and CI/CD Process

- Run static analysis after every new function is written:
  - Command: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
  - Fix all warnings before proceeding to write the next function.
- Before submission, ensure the workspace is clean:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
  - `cargo test --all-features`
- Feature branch workflow and CI gating:
  - Do all work on a feature branch (e.g., `feature/<task-id>-<short-name>`).
  - Push to the remote feature branch and monitor the GitHub Actions workflow (`.github/workflows/build-server.yml`) until it is green.
  - Require the deployment stage to complete successfully before creating a pull request.
  - Only create the PR after the workflow is green and deployment has succeeded; otherwise fix issues and re-run.
EOF

read -r -d '' BLOCK_TASK << 'EOF' || true

## CI/CD and Code Quality Requirements

- Per-function linting: After creating any new function, immediately run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` and resolve all warnings.
- Pre-commit checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, and `cargo test --all-features` must pass locally.
- Branching: Implement on a feature branch (e.g., `feature/<task-id>-<short-name>`).
- CI gate: Push to the feature branch and monitor GitHub Actions until all jobs are green and deployment completes successfully.
- PR creation: Only open the pull request after CI is green and the deployment stage has succeeded.
EOF

append_if_missing() {
  local file="$1"
  local marker="$2"
  local block="$3"
  if [[ ! -f "$file" ]]; then
    return 0
  fi
  if grep -qi -- "$marker" "$file"; then
    echo "skip  : $file (already contains: $marker)"
  else
    printf "%s\n" "$block" >> "$file"
    echo "update: $file (appended: $marker)"
  fi
}

shopt -s nullglob
for dir in "$ROOT_DIR"/task-*; do
  [[ -d "$dir" ]] || continue

  # acceptance-criteria.md
  append_if_missing "$dir/acceptance-criteria.md" "NFR-0: Code Quality and Automation" "$BLOCK_AC"

  # prompt.md
  append_if_missing "$dir/prompt.md" "Quality Gates and CI/CD Process" "$BLOCK_PROMPT"

  # task.md
  append_if_missing "$dir/task.md" "CI/CD and Code Quality Requirements" "$BLOCK_TASK"
done

echo "Done."


