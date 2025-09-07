#!/usr/bin/env bash

# Intelligent ingest regression helper
# Usage:
#   BASE_URL=https://doc-server.agent-platform.svc.cluster.local:3001 \
#   scripts/test-intelligent-ingest.sh <repo_url> <doc_type>
#
# Env:
#   BASE_URL     - Service base URL (e.g., https://doc-server.agent-platform.svc.cluster.local:3001)
#   CURL_FLAGS   - Extra flags for curl (e.g., --insecure for self-signed TLS)
#   POLL_INTERVAL - Seconds between polls (default: 10)
#   POLL_MAX     - Max polls before timeout (default: 180 => ~30 minutes)

set -euo pipefail

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  sed -n '1,40p' "$0"
  exit 0
fi

BASE_URL=${BASE_URL:-}
if [[ -z "${BASE_URL}" ]]; then
  echo "ERROR: BASE_URL is required (e.g., https://doc-server.agent-platform.svc.cluster.local:3001)" >&2
  exit 1
fi

REPO_URL=${1:-}
DOC_TYPE=${2:-}
if [[ -z "${REPO_URL}" || -z "${DOC_TYPE}" ]]; then
  echo "Usage: BASE_URL=... $0 <repo_url> <doc_type>" >&2
  exit 1
fi

CURL_FLAGS=${CURL_FLAGS:-}
POLL_INTERVAL=${POLL_INTERVAL:-10}
POLL_MAX=${POLL_MAX:-180}

echo "[INFO] Starting intelligent ingest"
echo "[INFO]  Base URL : ${BASE_URL}"
echo "[INFO]  Repo URL : ${REPO_URL}"
echo "[INFO]  Doc Type : ${DOC_TYPE}"
echo

START_PAYLOAD=$(jq -n --arg url "${REPO_URL}" --arg doc_type "${DOC_TYPE}" '{url:$url, doc_type:$doc_type}')

echo "[INFO] Enqueuing job..."
START_RESP=$(curl -sS ${CURL_FLAGS} -X POST "${BASE_URL}/ingest/intelligent" \
  -H 'Content-Type: application/json' \
  -d "${START_PAYLOAD}") || {
  echo "[ERROR] Failed to POST /ingest/intelligent" >&2
  exit 1
}

JOB_ID=$(echo "${START_RESP}" | jq -r '.job_id // empty')
if [[ -z "${JOB_ID}" ]]; then
  echo "[ERROR] No job_id in response. Full response:" >&2
  echo "${START_RESP}" >&2
  exit 1
fi

echo "[INFO] Job ID: ${JOB_ID}"
echo "[INFO] Polling status... (interval=${POLL_INTERVAL}s, max=${POLL_MAX})"

COUNT=0
LAST_STATUS=""
while :; do
  STATUS_RESP=$(curl -sS ${CURL_FLAGS} "${BASE_URL}/ingest/jobs/${JOB_ID}") || {
    echo "[WARN] Failed to GET job status (attempt ${COUNT})" >&2
    STATUS_RESP='{}'
  }
  STATUS=$(echo "${STATUS_RESP}" | jq -r '.status // empty')
  STARTED=$(echo "${STATUS_RESP}" | jq -r '.started_at // empty')
  FINISHED=$(echo "${STATUS_RESP}" | jq -r '.finished_at // empty')

  if [[ "${STATUS}" != "${LAST_STATUS}" && -n "${STATUS}" ]]; then
    echo "[INFO]  Status => ${STATUS}"
    LAST_STATUS="${STATUS}"
  fi

  case "${STATUS}" in
    Succeeded|succeeded|completed|Completed)
      echo
      echo "[PASS] Job succeeded"
      echo "[INFO]  Started : ${STARTED}"
      echo "[INFO]  Finished: ${FINISHED}"
      echo
      # Print a brief snippet of output
      echo "--- Output (first 120 lines) ---"
      echo "${STATUS_RESP}" | jq -r '.output // "(no output)"' | head -n 120
      echo "--- End Output ---"
      exit 0
      ;;
    Failed|failed)
      echo
      echo "[FAIL] Job failed"
      echo "[INFO]  Started : ${STARTED}"
      echo "[INFO]  Finished: ${FINISHED}"
      echo
      echo "--- Error ---"
      echo "${STATUS_RESP}" | jq -r '.error // "(no error)"'
      echo "--- Output (first 120 lines) ---"
      echo "${STATUS_RESP}" | jq -r '.output // "(no output)"' | head -n 120
      echo "--- End Output ---"
      exit 2
      ;;
    Running|running|Queued|queued)
      :
      ;;
    "")
      echo "[WARN] Empty status; response:" >&2
      echo "${STATUS_RESP}" >&2
      ;;
  esac

  COUNT=$((COUNT+1))
  if (( COUNT >= POLL_MAX )); then
    echo
    echo "[FAIL] Timed out waiting for job completion after $((POLL_INTERVAL*POLL_MAX))s"
    echo "[INFO] Last status response:"
    echo "${STATUS_RESP}"
    exit 3
  fi
  sleep "${POLL_INTERVAL}"
done
