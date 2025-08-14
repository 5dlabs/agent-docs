#!/usr/bin/env bash
set -euo pipefail

# harmonize_task_ids.sh
# Align Task IDs inside docs with their folder numbers to avoid confusion.
# - Updates '# Task ID: <n>' in task.txt
# - Updates leading '# Task <n>:' header in task.md if present
#
# Usage:
#   scripts/harmonize_task_ids.sh [tasks_root]
# Default tasks_root: docs/.taskmaster/docs

ROOT_DIR="${1:-docs/.taskmaster/docs}"

if [[ ! -d "$ROOT_DIR" ]]; then
  echo "Error: tasks root not found: $ROOT_DIR" >&2
  exit 1
fi

changed=0
skipped=0

shopt -s nullglob
for dir in "$ROOT_DIR"/task-*; do
  base=$(basename "$dir")
  if [[ ! $base =~ ^task-([0-9]+)$ ]]; then
    continue
  fi
  folder_id=${BASH_REMATCH[1]}

  txt_file="$dir/task.txt"
  md_file="$dir/task.md"

  # Update Task ID in task.txt
  if [[ -f "$txt_file" ]]; then
    # Only update if the first matching line differs
    current_id=$(grep -m1 -E '^# Task ID: [0-9]+' "$txt_file" | awk '{print $4}') || true
    if [[ -n "${current_id:-}" && "$current_id" != "$folder_id" ]]; then
      # Replace only the first occurrence line starting with '# Task ID:'
      tmp_file="$(mktemp)"
      awk -v new_id="$folder_id" '
        BEGIN {done=0}
        {
          if (!done && $0 ~ /^# Task ID: [0-9]+/) {
            print "# Task ID: " new_id
            done=1
          } else {
            print $0
          }
        }
      ' "$txt_file" > "$tmp_file"
      mv "$tmp_file" "$txt_file"
      echo "updated: $txt_file (ID -> $folder_id)"
      ((changed++))
    else
      ((skipped++))
    fi
  fi

  # Update leading '# Task <n>:' header in task.md if present
  if [[ -f "$md_file" ]]; then
    # Detect first header line like '# Task <number>:'
    if grep -q -m1 -E '^# Task [0-9]+:' "$md_file"; then
      current_md_id=$(grep -m1 -E '^# Task [0-9]+:' "$md_file" | sed -E 's/^# Task ([0-9]+):.*/\1/')
      if [[ "$current_md_id" != "$folder_id" ]]; then
        tmp_file="$(mktemp)"
        awk -v new_id="$folder_id" '
          BEGIN {done=0}
          {
            if (!done && $0 ~ /^# Task [0-9]+:/) {
              sub(/^# Task [0-9]+:/, "# Task " new_id ":")
              done=1
            }
            print $0
          }
        ' "$md_file" > "$tmp_file"
        mv "$tmp_file" "$md_file"
        echo "updated: $md_file (header -> Task $folder_id)"
        ((changed++))
      else
        ((skipped++))
      fi
    fi
  fi
done

echo "Done. Changed: $changed, Skipped: $skipped"


