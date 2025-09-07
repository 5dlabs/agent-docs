#!/usr/bin/env bash
set -euo pipefail

# Debug PVC/PV node affinity vs pod nodeSelector/tolerations for Agent Docs chart
# Usage: scripts/debug-pvc-node-affinity.sh [-n <namespace>] [--sc <storageClassName>] [--release <name>]
# Defaults: namespace=agent-docs, storageClassName=local-path

NS="agent-docs"
SC_NAME="local-path"
RELEASE_NAME=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    -n|--namespace)
      NS="$2"; shift 2;;
    --sc)
      SC_NAME="$2"; shift 2;;
    --release)
      RELEASE_NAME="$2"; shift 2;;
    -h|--help)
      echo "Usage: $0 [-n <namespace>] [--sc <storageClassName>] [--release <name>]"; exit 0;;
    *) echo "Unknown arg: $1"; exit 1;;
  esac
done

echo "[INFO] Namespace: $NS"
echo "[INFO] StorageClass: $SC_NAME"

echo
echo "=== Nodes (labels + taints) ==="
kubectl get nodes --show-labels || true
echo
for node in $(kubectl get nodes -o name | sed 's#node/##'); do
  echo "--- $node ---"
  kubectl describe node "$node" | sed -n '1,/Non-terminated Pods:/p' | sed -n '/Taints:/,/Roles:/p' || true
  echo
done

echo "=== StorageClass ($SC_NAME) ==="
if kubectl get sc "$SC_NAME" >/dev/null 2>&1; then
  kubectl get sc "$SC_NAME" -o yaml | sed -n '1,160p'
else
  echo "[WARN] StorageClass '$SC_NAME' not found"
fi

echo
echo "=== PVCs in namespace '$NS' ==="
kubectl -n "$NS" get pvc || true

# Try to identify the workdir PVC created by the chart (suffix '-workdir')
WORKDIR_PVC=$(kubectl -n "$NS" get pvc -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.spec.volumeName}{"\n"}{end}' 2>/dev/null | awk '/workdir/ {print $1; exit}')
if [[ -z "${WORKDIR_PVC:-}" ]]; then
  echo "[WARN] Could not auto-detect a '*-workdir' PVC; selecting the first PVC (if any)."
  WORKDIR_PVC=$(kubectl -n "$NS" get pvc -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}' 2>/dev/null | head -n1 || true)
fi

if [[ -n "${WORKDIR_PVC:-}" ]]; then
  echo
  echo "=== Details for PVC: $WORKDIR_PVC ==="
  kubectl -n "$NS" describe pvc "$WORKDIR_PVC" | sed -n '1,160p'
  PV_NAME=$(kubectl -n "$NS" get pvc "$WORKDIR_PVC" -o jsonpath='{.spec.volumeName}' 2>/dev/null || true)
  if [[ -n "$PV_NAME" ]]; then
    echo
    echo "=== PV: $PV_NAME (nodeAffinity excerpt) ==="
    kubectl get pv "$PV_NAME" -o yaml | sed -n '/nodeAffinity:/,/^  claimRef:/p' || true
  else
    echo "[WARN] PVC '$WORKDIR_PVC' is not bound to a PV yet."
  fi
else
  echo "[WARN] No PVCs found in namespace '$NS'"
fi

echo
echo "=== Deployments (server + worker) in '$NS' ==="
kubectl -n "$NS" get deploy -o wide || true

# Try locate worker and server deployments by labels
WORKER_DEP=$(kubectl -n "$NS" get deploy -l app.kubernetes.io/component=worker -o name 2>/dev/null | head -n1 || true)
SERVER_DEP=$(kubectl -n "$NS" get deploy -l app.kubernetes.io/name=agent-docs-server -o name 2>/dev/null | grep -v worker | head -n1 || true)

if [[ -z "$SERVER_DEP" ]]; then
  # Fallback: pick the first deployment
  SERVER_DEP=$(kubectl -n "$NS" get deploy -o name 2>/dev/null | head -n1 || true)
fi

echo
if [[ -n "$SERVER_DEP" ]]; then
  echo "=== Describe (server): $SERVER_DEP ==="
  kubectl -n "$NS" describe "$SERVER_DEP" | sed -n '/Selector:/,/Events:/p' || true
else
  echo "[WARN] Could not locate server deployment"
fi

echo
if [[ -n "$WORKER_DEP" ]]; then
  echo "=== Describe (worker): $WORKER_DEP ==="
  kubectl -n "$NS" describe "$WORKER_DEP" | sed -n '/Selector:/,/Events:/p' || true
else
  echo "[WARN] Could not locate worker deployment"
fi

echo
echo "=== Pods (wide) ==="
kubectl -n "$NS" get pods -o wide || true

echo
echo "[NEXT STEPS]"
cat <<'EOF'
- If the PV nodeAffinity points to a different node than your pod nodeSelector, either:
  1) Delete/recreate the PVC so it rebinds to the node you target (best with StorageClass volumeBindingMode=WaitForFirstConsumer), or
  2) Temporarily retarget nodeSelector/tolerations to the node hosting the PV.
- If you must schedule on the control-plane, add a toleration for the taint:
  key: node-role.kubernetes.io/control-plane, operator: Exists, effect: NoSchedule
- With local-path (RWO), keep replicas=1 for any workload that mounts the same PVC, or move to an RWX storage class.
EOF

echo "[INFO] Done. Review the sections above for mismatches between PV nodeAffinity, pod nodeSelector, and node taints."

